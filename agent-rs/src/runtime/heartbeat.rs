use std::collections::BTreeMap;

use crate::{config::TransportMode, error::AppResult, proto::agent, runtime::DiagnosticsSnapshot};

pub fn build_heartbeat_payload(
    agent_id: String,
    hostname: String,
    version: String,
    transport_mode: TransportMode,
    edge_url: &str,
    current_policy_revision: Option<String>,
) -> agent::HeartbeatPayload {
    let mut host_metadata = BTreeMap::new();
    host_metadata.insert("os".to_string(), std::env::consts::OS.to_string());
    host_metadata.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    host_metadata.insert(
        "transport_mode".to_string(),
        match transport_mode {
            TransportMode::Edge => "edge".to_string(),
            TransportMode::Mock => "mock".to_string(),
        },
    );
    host_metadata.insert("edge_url".to_string(), edge_url.to_string());
    if let Some(revision) = current_policy_revision {
        host_metadata.insert("policy_revision".to_string(), revision);
    }

    agent::HeartbeatPayload {
        agent_id,
        hostname,
        version,
        status: "online".to_string(),
        host_metadata,
        sent_at_unix_ms: chrono::Utc::now().timestamp_millis(),
    }
}

pub fn build_diagnostics_payload(
    agent_id: String,
    snapshot: &DiagnosticsSnapshot,
) -> AppResult<agent::DiagnosticsPayload> {
    Ok(agent::DiagnosticsPayload {
        agent_id,
        payload_json: serde_json::to_string(snapshot)?,
        sent_at_unix_ms: chrono::Utc::now().timestamp_millis(),
    })
}
