use std::{sync::Arc, time::Duration};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    config::TransportMode,
    error::AppResult,
    runtime::{
        heartbeat::{build_diagnostics_payload, build_heartbeat_payload},
        RuntimeStatusHandle,
    },
    transport::AgentTransport,
};

pub fn spawn_heartbeat_loop(
    transport: Arc<dyn AgentTransport>,
    status: RuntimeStatusHandle,
    shutdown: CancellationToken,
    agent_id: String,
    hostname: String,
    version: String,
    edge_url: String,
    transport_mode: TransportMode,
    interval_sec: u64,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!("heartbeat loop stopping");
                    return Ok(());
                }
                _ = ticker.tick() => {
                    let snapshot = status.snapshot();
                    let heartbeat = build_heartbeat_payload(
                        agent_id.clone(),
                        hostname.clone(),
                        version.clone(),
                        transport_mode.clone(),
                        &edge_url,
                        snapshot.current_policy_revision.clone(),
                    );
                    if let Err(error) = transport.send_heartbeat(heartbeat).await {
                        status.record_error(format!("heartbeat send failed: {error}"));
                        error!(error = %error, "failed to send heartbeat");
                    }

                    match build_diagnostics_payload(agent_id.clone(), &snapshot) {
                        Ok(payload) => {
                            if let Err(error) = transport.send_diagnostics(payload).await {
                                status.record_error(format!("diagnostics send failed: {error}"));
                                error!(error = %error, "failed to send diagnostics");
                            } else {
                                status.clear_error();
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
