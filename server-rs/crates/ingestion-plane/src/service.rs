use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
};

use async_nats::Client;
use chrono::{DateTime, SecondsFormat, Utc};
use common::{
    json::NormalizedLogEvent,
    nats_subjects::{LOGS_INGEST_NORMALIZED, UI_STREAM_LOGS},
    proto::edge,
    AppError, AppResult,
};
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing::warn;
use uuid::Uuid;

use crate::{
    config::IngestionPlaneConfig,
    storage::{ClickHouseClient, OpenSearchClient},
};

#[derive(Clone)]
pub struct IngestionService {
    nats: Client,
    opensearch: OpenSearchClient,
    clickhouse: ClickHouseClient,
    ready: Arc<AtomicBool>,
}

impl IngestionService {
    pub fn new(config: &IngestionPlaneConfig, nats: Client) -> Self {
        Self {
            nats,
            opensearch: OpenSearchClient::new(config.opensearch.clone()),
            clickhouse: ClickHouseClient::new(config.clickhouse.clone()),
            ready: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        self.opensearch.ensure_schema().await?;
        self.clickhouse.ensure_schema().await?;
        self.ready.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub async fn ingest_batch(&self, request: edge::IngestLogsRequest) -> AppResult<usize> {
        let events = normalize_batch(request)?;
        self.opensearch
            .index_events(&events)
            .await
            .map_err(map_integration_error)?;
        self.clickhouse
            .insert_events(&events)
            .await
            .map_err(map_integration_error)?;

        for event in events.iter().cloned() {
            let payload = serde_json::to_vec(&event)
                .map_err(|error| AppError::internal(format!("serialize normalized event: {error}")))?;
            if let Err(error) = self
                .nats
                .publish(LOGS_INGEST_NORMALIZED.to_string(), payload.clone().into())
                .await
            {
                warn!(error = %error, event_id = %event.id, "failed to publish normalized log event");
            }
            if let Err(error) = self
                .nats
                .publish(UI_STREAM_LOGS.to_string(), payload.into())
                .await
            {
                warn!(error = %error, event_id = %event.id, "failed to publish ui stream log event");
            }
        }

        Ok(events.len())
    }
}

pub fn normalize_batch(request: edge::IngestLogsRequest) -> AppResult<Vec<NormalizedLogEvent>> {
    if request.agent_id.trim().is_empty() {
        return Err(AppError::invalid_argument("agent_id is required"));
    }
    if request.events.is_empty() {
        return Err(AppError::invalid_argument("events are required"));
    }

    let batch_sent_at = timestamp_or_now(request.sent_at_unix_ms);
    let host = non_empty_or(&request.host, "unknown-host");

    request
        .events
        .into_iter()
        .map(|event| normalize_event(&request.agent_id, &host, batch_sent_at, event))
        .collect()
}

fn normalize_event(
    agent_id: &str,
    host: &str,
    batch_sent_at: DateTime<Utc>,
    event: edge::AgentLog,
) -> AppResult<NormalizedLogEvent> {
    let timestamp = timestamp_or_now(event.timestamp_unix_ms);
    let labels = normalize_labels(event.labels);
    let service = non_empty_or(&event.service, "agent");
    let severity = normalize_severity(&event.severity);
    let source_type = labels
        .get("source_type")
        .cloned()
        .unwrap_or_else(|| "file".to_string());
    let source = labels
        .get("source")
        .cloned()
        .or_else(|| labels.get("path").cloned())
        .unwrap_or_default();
    let message = event.message.trim().to_string();
    if message.is_empty() {
        return Err(AppError::invalid_argument("log message cannot be empty"));
    }

    let raw = serde_json::to_string(&json!({
        "service": service,
        "severity": severity,
        "message": message,
        "labels": labels,
        "batch_sent_at": batch_sent_at.to_rfc3339_opts(SecondsFormat::Millis, true),
    }))
    .map_err(|error| AppError::internal(format!("serialize raw log payload: {error}")))?;

    Ok(NormalizedLogEvent {
        id: Uuid::new_v4().to_string(),
        timestamp: timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
        host: host.to_string(),
        agent_id: agent_id.trim().to_string(),
        source_type,
        source,
        service,
        severity,
        message: message.clone(),
        fingerprint: fingerprint_for(&message),
        labels,
        fields: Value::Object(Default::default()),
        raw,
    })
}

fn normalize_labels(labels: BTreeMap<String, String>) -> BTreeMap<String, String> {
    labels
        .into_iter()
        .filter_map(|(key, value)| {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}

fn normalize_severity(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "trace" => "trace".to_string(),
        "debug" => "debug".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "error" | "err" => "error".to_string(),
        "critical" | "fatal" => "critical".to_string(),
        "info" | "" => "info".to_string(),
        other => other.to_string(),
    }
}

fn fingerprint_for(message: &str) -> String {
    static NUMBER_RE: OnceLock<Regex> = OnceLock::new();
    static HEX_RE: OnceLock<Regex> = OnceLock::new();
    let number_re =
        NUMBER_RE.get_or_init(|| Regex::new(r"\b\d+\b").expect("compile number regex"));
    let hex_re = HEX_RE.get_or_init(|| {
        Regex::new(r"\b[0-9a-fA-F]{8,}\b").expect("compile hex regex")
    });

    let normalized = number_re.replace_all(message, "<n>");
    let normalized = hex_re.replace_all(&normalized, "<hex>");

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

fn timestamp_or_now(unix_ms: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp_millis(unix_ms).unwrap_or_else(Utc::now)
}

fn non_empty_or(value: &str, default: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn map_integration_error(error: anyhow::Error) -> AppError {
    AppError::internal(format!("ingestion storage error: {error:#}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common::proto::edge::{AgentLog, IngestLogsRequest};

    use super::normalize_batch;

    #[test]
    fn normalizes_ingest_batch() {
        let batch = IngestLogsRequest {
            agent_id: "agent-1".to_string(),
            host: "srv-1".to_string(),
            sent_at_unix_ms: 1_742_517_200_000,
            events: vec![AgentLog {
                timestamp_unix_ms: 1_742_517_200_100,
                service: "nginx".to_string(),
                severity: "warning".to_string(),
                message: "GET /healthz returned 503".to_string(),
                labels: BTreeMap::from([
                    ("path".to_string(), "/var/log/nginx/access.log".to_string()),
                    ("source_type".to_string(), "file".to_string()),
                ]),
            }],
        };

        let events = normalize_batch(batch).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].host, "srv-1");
        assert_eq!(events[0].service, "nginx");
        assert_eq!(events[0].severity, "warn");
        assert_eq!(events[0].source, "/var/log/nginx/access.log");
    }
}
