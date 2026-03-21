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
            degraded_mode: false,
            blocked_delivery: false,
            blocked_reason: None,
            spool_enabled: true,
            consecutive_send_failures: 0,
            updated_at_unix_ms: 0,
        }
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
