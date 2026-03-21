use std::{path::PathBuf, sync::Arc, time::Duration};

use chrono::Utc;
use serde::Serialize;
use sha2::Digest;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    config::SecurityScanConfig,
    error::{AppError, AppResult},
    proto::agent,
    runtime::{state_writer::StateWriterHandle, RuntimeStatusHandle},
    security::{
        persist_report, run_security_scan, SecurityPostureStatusSnapshot, SecurityScanContext,
    },
    state::RuntimeStatePatch,
    transport::AgentTransport,
};

pub fn spawn_security_scan_worker(
    transport: Arc<dyn AgentTransport>,
    status: RuntimeStatusHandle,
    state_writer: StateWriterHandle,
    shutdown: CancellationToken,
    state_dir: PathBuf,
    hostname: String,
    config: SecurityScanConfig,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let mut first_iteration = true;
        let mut consecutive_failures = status.snapshot().security_posture.consecutive_failures;

        loop {
            let mut delay_sec = if first_iteration {
                0
            } else {
                config.interval_sec
            };
            if !first_iteration {
                delay_sec = delay_sec.saturating_add(scan_jitter_sec(
                    &status.current_agent_id(),
                    &hostname,
                    config.jitter_sec,
                ));
                if consecutive_failures > 0 {
                    delay_sec = delay_sec.saturating_add(failure_backoff_sec(consecutive_failures));
                }
            }

            if delay_sec > 0 {
                tokio::select! {
                    _ = shutdown.cancelled() => return Ok(()),
                    _ = tokio::time::sleep(Duration::from_secs(delay_sec)) => {}
                }
            }
            first_iteration = false;

            let context = SecurityScanContext {
                agent_id: status.current_agent_id(),
                hostname: hostname.clone(),
                state_dir: state_dir.clone(),
                config: config.clone(),
            };

            let mut artifacts = tokio::task::spawn_blocking(move || run_security_scan(&context))
                .await
                .map_err(AppError::from)?;

            let now = Utc::now().timestamp_millis();
            let mut delivery_error = None;

            if let (Some(path), Some(report_json)) =
                (artifacts.report_path.clone(), artifacts.report_json.clone())
            {
                if let Err(error) =
                    tokio::task::spawn_blocking(move || persist_report(&path, &report_json))
                        .await
                        .map_err(AppError::from)?
                {
                    warn!(error = %error, "security posture report persistence failed");
                    delivery_error = Some(error);
                } else {
                    artifacts.state.last_report_path = artifacts
                        .report_path
                        .as_ref()
                        .map(|path| path.display().to_string());
                }
            }

            if config.publish_as_diagnostics {
                if let Some(event) = artifacts.rules_loaded.as_ref() {
                    if let Err(error) =
                        publish_event(&*transport, &context_agent_id(event), event).await
                    {
                        delivery_error = Some(error.to_string());
                    }
                }
                if let Some(event) = artifacts.report.as_ref() {
                    if let Err(error) =
                        publish_event(&*transport, &context_agent_id(event), event).await
                    {
                        delivery_error = Some(error.to_string());
                    }
                }
                if let Some(event) = artifacts.failure.as_ref() {
                    if let Err(error) =
                        publish_event(&*transport, &context_agent_id(event), event).await
                    {
                        delivery_error = Some(error.to_string());
                    }
                }
                if let Some(event) = artifacts.skipped.as_ref() {
                    if let Err(error) =
                        publish_event(&*transport, &context_agent_id(event), event).await
                    {
                        delivery_error = Some(error.to_string());
                    }
                }
            }

            if let Some(error_message) = delivery_error.clone() {
                status.record_error(format!("security posture publish failed: {error_message}"));
                artifacts.state.last_delivery_status = Some("failed".to_string());
                artifacts.state.last_delivery_error = Some(error_message.clone());
                let _ = state_writer
                    .update_runtime_state(connectivity_error_patch(&error_message))
                    .await;
            } else if config.publish_as_diagnostics {
                artifacts.state.last_delivery_status = Some("published".to_string());
                artifacts.state.last_delivery_error = None;
            } else {
                artifacts.state.last_delivery_status = Some("local_only".to_string());
                artifacts.state.last_delivery_error = None;
            }

            let scan_failed = artifacts.failure.is_some() || delivery_error.is_some();
            if scan_failed {
                consecutive_failures = consecutive_failures.saturating_add(1);
            } else if artifacts.report.is_some() {
                consecutive_failures = 0;
            }

            artifacts.state.consecutive_failures = consecutive_failures;
            artifacts.state.backoff_until_unix_ms = if consecutive_failures == 0 {
                None
            } else {
                Some(now.saturating_add((failure_backoff_sec(consecutive_failures) * 1_000) as i64))
            };
            artifacts.state.updated_at_unix_ms = now;

            state_writer
                .save_security_scan_state(artifacts.state.clone())
                .await?;
            status.set_security_posture_snapshot(SecurityPostureStatusSnapshot::from_record(
                &config,
                &artifacts.state,
            ));

            if let Some(event) = artifacts.failure.as_ref() {
                error!(
                    event_id = event.event_id,
                    error_code = event.error_code,
                    error_message = event.error_message,
                    "security posture scan failed"
                );
            } else if let Some(event) = artifacts.skipped.as_ref() {
                info!(
                    event_id = event.event_id,
                    reason = event.reason_code,
                    "security posture scan skipped"
                );
                if matches!(
                    event.reason_code.as_str(),
                    "disabled_by_config" | "unsupported_platform"
                ) {
                    return Ok(());
                }
            } else if let Some(report) = artifacts.report.as_ref() {
                info!(
                    report_id = report.report_id,
                    findings = report.summary.total,
                    critical = report.summary.critical,
                    high = report.summary.high,
                    "security posture report generated"
                );
            }

            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                else => {}
            }
        }
    })
}

async fn publish_event<T>(
    transport: &dyn AgentTransport,
    agent_id: &str,
    event: &T,
) -> AppResult<()>
where
    T: Serialize,
{
    let payload = agent::DiagnosticsPayload {
        agent_id: agent_id.to_string(),
        payload_json: serde_json::to_string(event)?,
        sent_at_unix_ms: Utc::now().timestamp_millis(),
    };
    transport.send_diagnostics(payload).await
}

fn failure_backoff_sec(consecutive_failures: u32) -> u64 {
    (consecutive_failures as u64).saturating_mul(30).min(300)
}

fn scan_jitter_sec(agent_id: &str, hostname: &str, max_jitter_sec: u64) -> u64 {
    if max_jitter_sec == 0 {
        return 0;
    }
    let seed = format!(
        "{}:{}:{}",
        agent_id,
        hostname,
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let digest = sha2::Sha256::digest(seed.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(bytes) % (max_jitter_sec + 1)
}

fn connectivity_error_patch(message: &str) -> RuntimeStatePatch {
    let lower = message.to_ascii_lowercase();
    if lower.contains("tls")
        || lower.contains("certificate")
        || lower.contains("rustls")
        || lower.contains("ssl")
    {
        RuntimeStatePatch {
            last_tls_error: Some(Some(message.to_string())),
            ..RuntimeStatePatch::default()
        }
    } else {
        RuntimeStatePatch {
            last_connect_error: Some(Some(message.to_string())),
            ..RuntimeStatePatch::default()
        }
    }
}

trait SecurityEventAgentId {
    fn agent_id(&self) -> &str;
}

impl SecurityEventAgentId for crate::security::SecurityRulesLoadedEvent {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

impl SecurityEventAgentId for crate::security::SecurityPostureReport {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

impl SecurityEventAgentId for crate::security::SecurityScanFailureEvent {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

impl SecurityEventAgentId for crate::security::SecurityScanSkippedEvent {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

fn context_agent_id<T: SecurityEventAgentId>(event: &T) -> String {
    event.agent_id().to_string()
}
