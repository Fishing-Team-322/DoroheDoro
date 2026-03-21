use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    pub agent_id: String,
    pub hostname: String,
    pub version: String,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStateRecord {
    pub applied_policy_revision: Option<String>,
    pub policy_body_json: Option<String>,
    pub last_successful_send_at_unix_ms: Option<i64>,
    pub last_known_edge_url: Option<String>,
    pub runtime_status: Option<String>,
    pub runtime_status_reason: Option<String>,
    pub identity_status: Option<String>,
    pub identity_status_reason: Option<String>,
    pub last_policy_fetch_at_unix_ms: Option<i64>,
    pub last_policy_apply_at_unix_ms: Option<i64>,
    pub last_policy_error: Option<String>,
    pub last_connect_error: Option<String>,
    pub last_tls_error: Option<String>,
    pub last_handshake_success_at_unix_ms: Option<i64>,
    pub degraded_mode: bool,
    pub blocked_delivery: bool,
    pub blocked_reason: Option<String>,
    pub spool_enabled: bool,
    pub consecutive_send_failures: u32,
    pub updated_at_unix_ms: i64,
}

impl Default for RuntimeStateRecord {
    fn default() -> Self {
        Self {
            applied_policy_revision: None,
            policy_body_json: None,
            last_successful_send_at_unix_ms: None,
            last_known_edge_url: None,
            runtime_status: None,
            runtime_status_reason: None,
            identity_status: None,
            identity_status_reason: None,
            last_policy_fetch_at_unix_ms: None,
            last_policy_apply_at_unix_ms: None,
            last_policy_error: None,
            last_connect_error: None,
            last_tls_error: None,
            last_handshake_success_at_unix_ms: None,
            degraded_mode: false,
            blocked_delivery: false,
            blocked_reason: None,
            spool_enabled: true,
            consecutive_send_failures: 0,
            updated_at_unix_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeStatePatch {
    pub applied_policy_revision: Option<Option<String>>,
    pub policy_body_json: Option<Option<String>>,
    pub last_successful_send_at_unix_ms: Option<Option<i64>>,
    pub last_known_edge_url: Option<Option<String>>,
    pub runtime_status: Option<Option<String>>,
    pub runtime_status_reason: Option<Option<String>>,
    pub identity_status: Option<Option<String>>,
    pub identity_status_reason: Option<Option<String>>,
    pub last_policy_fetch_at_unix_ms: Option<Option<i64>>,
    pub last_policy_apply_at_unix_ms: Option<Option<i64>>,
    pub last_policy_error: Option<Option<String>>,
    pub last_connect_error: Option<Option<String>>,
    pub last_tls_error: Option<Option<String>>,
    pub last_handshake_success_at_unix_ms: Option<Option<i64>>,
    pub degraded_mode: Option<bool>,
    pub blocked_delivery: Option<bool>,
    pub blocked_reason: Option<Option<String>>,
    pub spool_enabled: Option<bool>,
    pub consecutive_send_failures: Option<u32>,
}

impl RuntimeStateRecord {
    pub fn apply_patch(&mut self, patch: RuntimeStatePatch) {
        if let Some(value) = patch.applied_policy_revision {
            self.applied_policy_revision = value;
        }
        if let Some(value) = patch.policy_body_json {
            self.policy_body_json = value;
        }
        if let Some(value) = patch.last_successful_send_at_unix_ms {
            self.last_successful_send_at_unix_ms = value;
        }
        if let Some(value) = patch.last_known_edge_url {
            self.last_known_edge_url = value;
        }
        if let Some(value) = patch.runtime_status {
            self.runtime_status = value;
        }
        if let Some(value) = patch.runtime_status_reason {
            self.runtime_status_reason = value;
        }
        if let Some(value) = patch.identity_status {
            self.identity_status = value;
        }
        if let Some(value) = patch.identity_status_reason {
            self.identity_status_reason = value;
        }
        if let Some(value) = patch.last_policy_fetch_at_unix_ms {
            self.last_policy_fetch_at_unix_ms = value;
        }
        if let Some(value) = patch.last_policy_apply_at_unix_ms {
            self.last_policy_apply_at_unix_ms = value;
        }
        if let Some(value) = patch.last_policy_error {
            self.last_policy_error = value;
        }
        if let Some(value) = patch.last_connect_error {
            self.last_connect_error = value;
        }
        if let Some(value) = patch.last_tls_error {
            self.last_tls_error = value;
        }
        if let Some(value) = patch.last_handshake_success_at_unix_ms {
            self.last_handshake_success_at_unix_ms = value;
        }
        if let Some(value) = patch.degraded_mode {
            self.degraded_mode = value;
        }
        if let Some(value) = patch.blocked_delivery {
            self.blocked_delivery = value;
        }
        if let Some(value) = patch.blocked_reason {
            self.blocked_reason = value;
        }
        if let Some(value) = patch.spool_enabled {
            self.spool_enabled = value;
        }
        if let Some(value) = patch.consecutive_send_failures {
            self.consecutive_send_failures = value;
        }
        self.updated_at_unix_ms = chrono::Utc::now().timestamp_millis();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOffsetRecord {
    pub path: String,
    pub file_key: Option<String>,
    // Persisted sqlite column is still named `read_offset`, but semantically this is the
    // highest durable local progress: acknowledged or safely written to fallback spool.
    pub durable_read_offset: u64,
    pub acked_offset: u64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOffsetUpdate {
    pub path: String,
    pub file_key: Option<String>,
    pub durable_read_offset: u64,
    pub acked_offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceOffsetMarker {
    pub source_id: String,
    pub path: String,
    pub file_key: Option<String>,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpoolBatchRecord {
    pub batch_id: String,
    pub payload_path: PathBuf,
    pub codec: String,
    pub created_at_unix_ms: i64,
    pub attempt_count: u32,
    pub next_retry_at_unix_ms: i64,
    pub approx_bytes: usize,
    pub source_offsets: Vec<SourceOffsetMarker>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpoolStats {
    pub batch_count: usize,
    pub total_bytes: u64,
}
