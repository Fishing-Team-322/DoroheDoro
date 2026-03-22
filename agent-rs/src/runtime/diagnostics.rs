use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::{
    error::{AppError, AppResult},
    proto::agent,
    runtime::{state_writer::StateWriterHandle, DiagnosticsSnapshot, RuntimeStatusHandle},
    state::RuntimeStatePatch,
    transport::AgentTransport,
};

pub fn spawn_diagnostics_worker(
    transport: Arc<dyn AgentTransport>,
    status: RuntimeStatusHandle,
    state_writer: StateWriterHandle,
    shutdown: CancellationToken,
    state_dir: PathBuf,
    interval_sec: u64,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
        status.mark_diagnostics_scheduler_running(true);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    status.mark_diagnostics_scheduler_running(false);
                    return Ok(());
                },
                _ = ticker.tick() => {
                    let attempt_at = chrono::Utc::now().timestamp_millis();
                    let snapshot_path = local_diagnostics_snapshot_path(&state_dir);
                    status.record_diagnostics_attempt(attempt_at);

                    let snapshot = status.snapshot();
                    if let Err(error) = persist_local_snapshot(&snapshot_path, &snapshot) {
                        let detail = format!("diagnostics snapshot persist failed: {error}");
                        status.record_diagnostics_failure(detail.clone());
                        status.record_error(detail);
                        error!(error = %error, snapshot_path = %snapshot_path.display(), "failed to persist diagnostics snapshot");
                        continue;
                    }
                    status.record_local_snapshot_write(
                        attempt_at,
                        snapshot_path.display().to_string(),
                    );

                    let payload_snapshot = status.snapshot();
                    match build_diagnostics_payload(&payload_snapshot) {
                        Ok(payload) => {
                            if let Err(error) = transport.send_diagnostics(payload).await {
                                let detail = format!("diagnostics send failed: {error}");
                                status.record_diagnostics_failure(detail.clone());
                                status.record_error(detail);
                                status.record_connectivity_error(&error);
                                let _ = state_writer
                                    .update_runtime_state(connectivity_error_patch(&error))
                                    .await;
                                error!(error = %error, "failed to send diagnostics");
                            } else {
                                let now = chrono::Utc::now().timestamp_millis();
                                status.record_connectivity_success(now);
                                status.record_diagnostics_success(now);
                                let _ = state_writer
                                    .update_runtime_state(RuntimeStatePatch {
                                        last_handshake_success_at_unix_ms: Some(Some(now)),
                                        last_connect_error: Some(None),
                                        last_tls_error: Some(None),
                                        ..RuntimeStatePatch::default()
                                    })
                                    .await;
                            }
                        }
                        Err(error) => {
                            let detail = format!("diagnostics build failed: {error}");
                            status.record_diagnostics_failure(detail.clone());
                            status.record_error(detail);
                            error!(error = %error, "failed to build diagnostics payload");
                        }
                    }
                }
            }
        }
    })
}

pub fn build_diagnostics_payload(
    snapshot: &DiagnosticsSnapshot,
) -> AppResult<agent::DiagnosticsPayload> {
    Ok(agent::DiagnosticsPayload {
        agent_id: snapshot.agent_id.clone(),
        payload_json: serde_json::to_string(snapshot)?,
        sent_at_unix_ms: chrono::Utc::now().timestamp_millis(),
    })
}

pub fn local_diagnostics_snapshot_path(state_dir: &Path) -> PathBuf {
    state_dir.join("runtime").join("diagnostics-snapshot.json")
}

fn persist_local_snapshot(path: &Path, snapshot: &DiagnosticsSnapshot) -> AppResult<()> {
    let Some(parent) = path.parent() else {
        return Err(AppError::protocol(
            "diagnostics snapshot path is missing a parent directory",
        ));
    };
    fs::create_dir_all(parent)?;

    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serde_json::to_vec_pretty(snapshot)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp_path, path)?;
    Ok(())
}

fn connectivity_error_patch(error: &AppError) -> RuntimeStatePatch {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("tls")
        || lower.contains("certificate")
        || lower.contains("rustls")
        || lower.contains("ssl")
    {
        RuntimeStatePatch {
            last_tls_error: Some(Some(message)),
            ..RuntimeStatePatch::default()
        }
    } else {
        RuntimeStatePatch {
            last_connect_error: Some(Some(message)),
            ..RuntimeStatePatch::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        config::{SourceConfig, StartAt},
        runtime::{test_static_context, RuntimeStatusHandle},
        state::FileOffsetRecord,
    };

    use super::build_diagnostics_payload;

    #[test]
    fn diagnostics_payload_serializes_nested_metadata() {
        let status = RuntimeStatusHandle::new(
            "agent-1".to_string(),
            "demo-host".to_string(),
            "0.1.0".to_string(),
            "edge".to_string(),
            test_static_context(),
            true,
            30,
            30,
            &[SourceConfig {
                kind: "file".to_string(),
                source_id: Some("file:/tmp/demo.log".to_string()),
                path: PathBuf::from("/tmp/demo.log"),
                start_at: StartAt::End,
                source: "demo".to_string(),
                service: "svc".to_string(),
                severity_hint: "info".to_string(),
            }],
            &[FileOffsetRecord {
                path: "/tmp/demo.log".to_string(),
                file_key: Some("1:2".to_string()),
                durable_read_offset: 1,
                acked_offset: 1,
                updated_at_unix_ms: 0,
            }],
            Some(42),
        );

        let payload = build_diagnostics_payload(&status.snapshot()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&payload.payload_json).unwrap();

        assert_eq!(value["platform"]["service_manager"], "systemd");
        assert_eq!(value["install"]["resolved_mode"], "dev");
        assert_eq!(value["cluster"]["configured_cluster_id"], "cluster-a");
        assert_eq!(value["identity_status"]["status"], "reused");
        assert_eq!(value["state"]["state_db_path"], "/tmp/doro-agent/state.db");
    }
}
