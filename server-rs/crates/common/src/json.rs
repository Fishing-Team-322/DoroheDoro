use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedLogEvent {
    pub id: String,
    pub timestamp: String,
    pub host: String,
    pub agent_id: String,
    pub source_type: String,
    pub source: String,
    pub service: String,
    pub severity: String,
    pub message: String,
    pub fingerprint: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub fields: Value,
    #[serde(default)]
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditAppendEvent {
    pub event_type: String,
    pub entity_type: String,
    pub entity_id: String,
    pub actor_id: String,
    pub actor_type: String,
    pub request_id: String,
    pub reason: String,
    #[serde(default)]
    pub payload_json: Value,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentStreamEvent {
    pub event_type: String,
    pub agent_id: String,
    pub hostname: String,
    pub status: String,
    pub version: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertStreamEvent {
    pub event_type: String,
    pub alert_instance_id: String,
    pub alert_rule_id: String,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub triggered_at: String,
    pub host: String,
    pub service: String,
}
