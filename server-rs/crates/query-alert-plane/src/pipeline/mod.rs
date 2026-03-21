pub mod signal;

use std::fmt;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::DetectionConfig;

pub use signal::{BaselineReference, SignalEnvelope, SignalSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DetectionMode {
    #[default]
    Medium,
    Light,
    Heavy,
}

impl DetectionMode {
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "medium" => Some(Self::Medium),
            "light" => Some(Self::Light),
            "heavy" => Some(Self::Heavy),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Heavy => "heavy",
        }
    }
}

impl fmt::Display for DetectionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct DetectionContext<'a> {
    pub mode: DetectionMode,
    pub config: &'a DetectionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyScore {
    pub detector: String,
    pub score: f64,
    pub threshold: f64,
    #[serde(default)]
    pub evidence: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionStatus {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub source_component: String,
    pub created_at: DateTime<Utc>,
    pub correlation_id: String,
    pub suggested_action: String,
}

impl DetectionStatus {
    pub fn new(code: &str, message: &str, severity: &str, suggested_action: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            source_component: "server-rs.query-alert-plane".to_string(),
            created_at: Utc::now(),
            correlation_id: String::new(),
            suggested_action: suggested_action.to_string(),
        }
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = correlation_id.into();
        self
    }

    pub fn created_at_rfc3339(&self) -> String {
        self.created_at.to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DetectionOutcome {
    Triggered { score: AnomalyScore },
    Suppressed { status: DetectionStatus },
    Skipped,
}
