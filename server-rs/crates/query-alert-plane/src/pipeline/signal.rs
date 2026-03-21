use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::DetectionMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalSource {
    Logs,
    Heartbeat,
    Diagnostics,
    SecurityPosture,
}

impl SignalSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalSource::Logs => "logs",
            SignalSource::Heartbeat => "heartbeat",
            SignalSource::Diagnostics => "diagnostics",
            SignalSource::SecurityPosture => "security_posture",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaselineReference {
    pub host: Option<String>,
    pub service: Option<String>,
    pub window_minutes: u32,
    pub samples: u32,
    pub mean: f64,
    pub stddev: f64,
    pub p95: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalEnvelope {
    pub schema_version: String,
    pub signal_id: String,
    pub source: SignalSource,
    pub agent_id: Option<String>,
    pub host: Option<String>,
    pub service: Option<String>,
    pub severity: Option<String>,
    pub event_ts: DateTime<Utc>,
    pub ingest_ts: DateTime<Utc>,
    pub detection_mode: DetectionMode,
    #[serde(default)]
    pub payload: Value,
    pub baseline: Option<BaselineReference>,
    #[serde(default)]
    pub metadata: Value,
}

impl SignalEnvelope {
    pub const SCHEMA_VERSION: &'static str = "2024-03-DEV3";

    pub fn new(signal_id: impl Into<String>, source: SignalSource) -> Self {
        let now = Utc::now();
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            signal_id: signal_id.into(),
            source,
            agent_id: None,
            host: None,
            service: None,
            severity: None,
            event_ts: now,
            ingest_ts: now,
            detection_mode: DetectionMode::Medium,
            payload: Value::Object(Default::default()),
            baseline: None,
            metadata: Value::Object(Default::default()),
        }
    }
}
