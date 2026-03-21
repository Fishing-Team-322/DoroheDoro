use chrono::{DateTime, SecondsFormat, Utc};
use common::proto::{alerts, query};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

pub fn format_ts(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn format_opt_ts(value: Option<DateTime<Utc>>) -> String {
    value.map(format_ts).unwrap_or_default()
}

#[derive(Debug, Clone, FromRow)]
pub struct AlertRuleRecord {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub severity: String,
    pub scope_type: String,
    pub scope_id: Option<String>,
    pub condition_json: Value,
    pub created_by: String,
    pub updated_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AlertRuleRecord {
    pub fn into_proto(self) -> alerts::AlertRule {
        alerts::AlertRule {
            alert_rule_id: self.id.to_string(),
            name: self.name,
            description: self.description,
            status: self.status,
            severity: self.severity,
            scope_type: self.scope_type,
            scope_id: self.scope_id.unwrap_or_default(),
            condition_json: self.condition_json.to_string(),
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct AlertInstanceRecord {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub host: String,
    pub service: String,
    pub fingerprint: String,
    pub payload_json: Value,
    pub detection_mode: String,
    pub correlation_key: String,
    pub source_signals: Value,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub auto_resolved_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub rule_name: Option<String>,
}

impl AlertInstanceRecord {
    pub fn into_proto(self) -> alerts::AlertInstance {
        alerts::AlertInstance {
            alert_instance_id: self.id.to_string(),
            alert_rule_id: self.rule_id.to_string(),
            title: self.title,
            status: self.status,
            severity: self.severity,
            triggered_at: format_ts(self.triggered_at),
            acknowledged_at: format_opt_ts(self.acknowledged_at),
            resolved_at: format_opt_ts(self.resolved_at),
            host: self.host,
            service: self.service,
            fingerprint: self.fingerprint,
            payload_json: self.payload_json.to_string(),
        }
    }

    pub fn into_log_projection(self) -> query::LogAnomalyProjection {
        query::LogAnomalyProjection {
            alert_instance_id: self.id.to_string(),
            alert_rule_id: self.rule_id.to_string(),
            status: self.status,
            severity: self.severity,
            title: self.title,
            fingerprint: self.fingerprint,
            host: self.host,
            service: self.service,
            triggered_at: format_ts(self.triggered_at),
            payload_json: self.payload_json.to_string(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct AuditActivityRecord {
    pub event_type: String,
    pub entity_type: String,
    pub entity_id: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AnomalyBaselineRecord {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub host: String,
    pub service: String,
    pub signal_kind: String,
    pub window_minutes: i32,
    pub samples: i32,
    pub mean: f64,
    pub stddev: f64,
    pub p95: Option<f64>,
    pub payload_json: Value,
    pub last_refreshed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AnomalyScoreRecord {
    pub id: Uuid,
    pub rule_id: Option<Uuid>,
    pub detector: String,
    pub signal_kind: String,
    pub host: String,
    pub service: String,
    pub correlation_key: String,
    pub detection_mode: String,
    pub signal_id: String,
    pub score: f64,
    pub threshold: f64,
    pub evidence_json: Value,
    pub created_at: DateTime<Utc>,
}

impl AuditActivityRecord {
    pub fn into_dashboard_item(self) -> query::DashboardActivityItem {
        query::DashboardActivityItem {
            kind: self.event_type,
            title: format!("{} {}", self.entity_type, self.entity_id),
            description: self.reason,
            timestamp: format_ts(self.created_at),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertRuleCondition {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default = "default_threshold")]
    pub threshold: u64,
    #[serde(default = "default_window_minutes")]
    pub window_minutes: u32,
    #[serde(default)]
    pub title: Option<String>,
}

impl Default for AlertRuleCondition {
    fn default() -> Self {
        Self {
            query: None,
            host: None,
            service: None,
            severity: None,
            fingerprint: None,
            threshold: default_threshold(),
            window_minutes: default_window_minutes(),
            title: None,
        }
    }
}

fn default_threshold() -> u64 {
    1
}

fn default_window_minutes() -> u32 {
    5
}
