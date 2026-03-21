use std::{sync::Arc, time::Duration};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::{
    error::AppResult,
    proto::agent,
    runtime::{DiagnosticsSnapshot, RuntimeStatusHandle},
    transport::AgentTransport,
};

pub fn spawn_diagnostics_worker(
    transport: Arc<dyn AgentTransport>,
    status: RuntimeStatusHandle,
    shutdown: CancellationToken,
    interval_sec: u64,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                _ = ticker.tick() => {
                    let snapshot = status.snapshot();
                    match build_diagnostics_payload(&snapshot) {
                        Ok(payload) => {
                            if let Err(error) = transport.send_diagnostics(payload).await {
                                status.record_error(format!("diagnostics send failed: {error}"));
                                error!(error = %error, "failed to send diagnostics");
                            }
                        }
                        Err(error) => {
                            status.record_error(format!("diagnostics build failed: {error}"));
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
