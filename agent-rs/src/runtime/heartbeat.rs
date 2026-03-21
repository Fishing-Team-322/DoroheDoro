use std::{collections::BTreeMap, sync::Arc, time::Duration};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::{
    error::AppResult,
    proto::agent,
    runtime::{DiagnosticsSnapshot, RuntimeStatusHandle},
    transport::AgentTransport,
};

pub fn spawn_heartbeat_worker(
    transport: Arc<dyn AgentTransport>,
    status: RuntimeStatusHandle,
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
                        status.record_error(format!("heartbeat send failed: {error}"));
                        error!(error = %error, "failed to send heartbeat");
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
    host_metadata.insert("os".to_string(), std::env::consts::OS.to_string());
    host_metadata.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    host_metadata.insert(
        "transport_mode".to_string(),
        snapshot.transport_state.mode.clone(),
    );
    host_metadata.insert("edge_url".to_string(), edge_url.to_string());
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
    if let Some(revision) = snapshot.current_policy_revision.clone() {
        host_metadata.insert("policy_revision".to_string(), revision);
    }

    agent::HeartbeatPayload {
        agent_id: snapshot.agent_id.clone(),
        hostname: snapshot.hostname.clone(),
        version: snapshot.version.clone(),
        status: if snapshot.degraded_mode {
            "degraded".to_string()
        } else {
            "online".to_string()
        },
        host_metadata,
        sent_at_unix_ms: chrono::Utc::now().timestamp_millis(),
    }
}
