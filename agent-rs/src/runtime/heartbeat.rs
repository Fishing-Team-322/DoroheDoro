use std::{collections::BTreeMap, sync::Arc, time::Duration};

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

pub fn spawn_heartbeat_worker(
    transport: Arc<dyn AgentTransport>,
    status: RuntimeStatusHandle,
    state_writer: StateWriterHandle,
    shutdown: CancellationToken,
    edge_url: String,
    interval_sec: u64,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                _ = ticker.tick() => {
                    let snapshot = status.snapshot();
                    let heartbeat = build_heartbeat_payload(&snapshot, &edge_url);
                    if let Err(error) = transport.send_heartbeat(heartbeat).await {
                        let detail = format!("heartbeat send failed: {error}");
                        status.record_error(detail);
                        status.record_connectivity_error(&error);
                        let _ = state_writer
                            .update_runtime_state(connectivity_error_patch(&error))
                            .await;
                        error!(error = %error, "failed to send heartbeat");
                    } else {
                        let now = chrono::Utc::now().timestamp_millis();
                        status.record_connectivity_success(now);
                        let _ = state_writer
                            .update_runtime_state(RuntimeStatePatch {
                                last_handshake_success_at_unix_ms: Some(Some(now)),
                                ..RuntimeStatePatch::default()
                            })
                            .await;
                    }
                }
            }
        }
    })
}

pub fn build_heartbeat_payload(
    snapshot: &DiagnosticsSnapshot,
    edge_url: &str,
) -> agent::HeartbeatPayload {
    let mut host_metadata = BTreeMap::new();
    host_metadata.insert("os_family".to_string(), snapshot.platform.os_family.clone());
    host_metadata.insert("arch".to_string(), snapshot.platform.architecture.clone());
    host_metadata.insert(
        "transport_mode".to_string(),
        snapshot.transport_state.mode.clone(),
    );
    host_metadata.insert("edge_url".to_string(), edge_url.to_string());
    host_metadata.insert(
        "resolved_install_mode".to_string(),
        snapshot.install.resolved_mode.clone(),
    );
    host_metadata.insert(
        "build_profile".to_string(),
        snapshot.build.build_profile.clone(),
    );
    host_metadata.insert("build_id".to_string(), snapshot.build.build_id.clone());
    host_metadata.insert(
        "degraded_mode".to_string(),
        snapshot.degraded_mode.to_string(),
    );
    host_metadata.insert(
        "blocked_delivery".to_string(),
        snapshot.blocked_delivery.to_string(),
    );
    host_metadata.insert(
        "event_queue_len".to_string(),
        snapshot.event_queue_len.to_string(),
    );
    host_metadata.insert(
        "send_queue_len".to_string(),
        snapshot.send_queue_len.to_string(),
    );
    host_metadata.insert(
        "spooled_batches".to_string(),
        snapshot.spooled_batches.to_string(),
    );
    host_metadata.insert(
        "systemd_detected".to_string(),
        snapshot.platform.systemd_detected.to_string(),
    );
    host_metadata.insert(
        "systemd_expected".to_string(),
        snapshot.install.systemd_expected.to_string(),
    );
    if let Some(value) = snapshot.platform.distro_name.clone() {
        host_metadata.insert("distro_name".to_string(), value);
    }
    if let Some(value) = snapshot.platform.distro_version.clone() {
        host_metadata.insert("distro_version".to_string(), value);
    }
    if let Some(value) = snapshot.platform.kernel_version.clone() {
        host_metadata.insert("kernel_version".to_string(), value);
    }
    if let Some(value) = snapshot.platform.machine_id_hash.clone() {
        host_metadata.insert("machine_id_hash".to_string(), value);
    }
    if let Some(value) = snapshot.cluster.configured_cluster_id.clone() {
        host_metadata.insert("cluster_id".to_string(), value);
    }
    if let Some(value) = snapshot.cluster.cluster_name.clone() {
        host_metadata.insert("cluster_name".to_string(), value);
    }
    if let Some(value) = snapshot.cluster.environment.clone() {
        host_metadata.insert("environment".to_string(), value);
    }
    if let Some(revision) = snapshot.current_policy_revision.clone() {
        host_metadata.insert("policy_revision".to_string(), revision);
    }

    // TODO: the current edge ingress heartbeat request does not forward agent host_metadata
    // end-to-end yet. Keep building the summary here so the agent stays contract-ready for the
    // later edge/contracts update.
    agent::HeartbeatPayload {
        agent_id: snapshot.agent_id.clone(),
        hostname: snapshot.hostname.clone(),
        version: snapshot.version.clone(),
        status: snapshot.runtime_status.clone(),
        host_metadata,
        sent_at_unix_ms: chrono::Utc::now().timestamp_millis(),
    }
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

    use super::build_heartbeat_payload;

    #[test]
    fn heartbeat_includes_platform_and_install_summary() {
        let status = RuntimeStatusHandle::new(
            "agent-1".to_string(),
            "demo-host".to_string(),
            "0.1.0".to_string(),
            "edge".to_string(),
            test_static_context(),
            true,
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
        let snapshot = status.snapshot();
        let heartbeat = build_heartbeat_payload(&snapshot, "https://edge.example.local");

        assert_eq!(
            heartbeat
                .host_metadata
                .get("resolved_install_mode")
                .map(String::as_str),
            Some("dev")
        );
        assert_eq!(
            heartbeat
                .host_metadata
                .get("distro_name")
                .map(String::as_str),
            Some("demo")
        );
        assert_eq!(
            heartbeat
                .host_metadata
                .get("cluster_id")
                .map(String::as_str),
            Some("cluster-a")
        );
        assert_eq!(
            heartbeat
                .host_metadata
                .get("systemd_detected")
                .map(String::as_str),
            Some("true")
        );
    }
}
