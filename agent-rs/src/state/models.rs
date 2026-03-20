#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    pub agent_id: String,
    pub hostname: String,
    pub version: String,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeStateRecord {
    pub applied_policy_revision: Option<String>,
    pub policy_body_json: Option<String>,
    pub last_successful_send_at_unix_ms: Option<i64>,
    pub last_known_edge_url: Option<String>,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOffsetRecord {
    pub path: String,
    pub file_key: Option<String>,
    pub offset: u64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOffsetUpdate {
    pub path: String,
    pub file_key: Option<String>,
    pub offset: u64,
}
