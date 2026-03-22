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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorStatus {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub source_component: String,
    pub created_at: String,
    pub correlation_id: String,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationEnvelopeV1 {
    pub schema_version: String,
    pub notification_id: String,
    pub correlation_id: String,
    pub created_at: String,
    pub event_type: String,
    pub severity: String,
    pub source_component: String,
    pub cluster_id: String,
    pub cluster_name: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub entity_kind: String,
    #[serde(default)]
    pub entity_id: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub service: String,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub details_url: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramDispatchRequestedEvent {
    pub schema_version: String,
    pub delivery_id: String,
    pub notification_id: String,
    pub integration_id: String,
    pub integration_binding_id: String,
    pub event_type: String,
    pub cluster_id: String,
    pub created_at: String,
    pub correlation_id: String,
    pub status: OperatorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramDispatchResultEvent {
    pub schema_version: String,
    pub delivery_id: String,
    pub notification_id: String,
    pub integration_id: String,
    pub integration_binding_id: String,
    pub attempt_id: String,
    pub attempt_number: u32,
    pub classification: String,
    pub delivery_status: String,
    pub telegram_message_id: String,
    pub retry_at: String,
    pub created_at: String,
    pub correlation_id: String,
    pub status: OperatorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramHealthcheckRequestV1 {
    pub schema_version: String,
    pub request_id: String,
    pub integration_id: String,
    pub correlation_id: String,
    pub created_at: String,
    #[serde(default)]
    pub chat_id_override: String,
    #[serde(default)]
    pub actor_id: String,
    #[serde(default)]
    pub actor_type: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramHealthcheckResultV1 {
    pub schema_version: String,
    pub request_id: String,
    pub healthcheck_run_id: String,
    pub integration_id: String,
    pub resolved_chat_id: String,
    pub classification: String,
    pub delivery_status: String,
    pub telegram_message_id: String,
    pub created_at: String,
    pub correlation_id: String,
    pub status: OperatorStatus,
}
