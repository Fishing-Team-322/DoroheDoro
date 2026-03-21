use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use serde_json::json;

use common::json::NormalizedLogEvent;

use crate::config::{ClickHouseConfig, OpenSearchConfig};

#[derive(Clone)]
pub struct OpenSearchClient {
    http: Client,
    config: OpenSearchConfig,
}

impl OpenSearchClient {
    pub fn new(config: OpenSearchConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    pub async fn ensure_schema(&self) -> anyhow::Result<()> {
        let url = format!(
            "{}/_index_template/{}-logs",
            self.config.url.trim_end_matches('/'),
            self.config.index_prefix
        );
        let body = json!({
            "index_patterns": [format!("{}-logs-*", self.config.index_prefix)],
            "template": {
                "mappings": {
                    "properties": {
                        "timestamp": { "type": "date" },
                        "host": { "type": "keyword" },
                        "agent_id": { "type": "keyword" },
                        "source_type": { "type": "keyword" },
                        "source": { "type": "keyword" },
                        "service": { "type": "keyword" },
                        "severity": { "type": "keyword" },
                        "message": { "type": "text" },
                        "fingerprint": { "type": "keyword" },
                        "labels": { "type": "object", "dynamic": true },
                        "fields": { "type": "object", "dynamic": true },
                        "raw": { "type": "text", "index": false }
                    }
                }
            }
        });

        let response = self
            .request(self.http.put(url))
            .json(&body)
            .send()
            .await
            .context("put opensearch index template")?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow!("opensearch schema setup failed: {status} {body}"))
        }
    }

    pub async fn index_events(&self, events: &[NormalizedLogEvent]) -> anyhow::Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let mut payload = String::new();
        for event in events {
            let index = self.index_name(&event.timestamp)?;
            let header = json!({ "index": { "_index": index, "_id": event.id } });
            payload.push_str(&serde_json::to_string(&header)?);
            payload.push('\n');
            payload.push_str(&serde_json::to_string(event)?);
            payload.push('\n');
        }

        let url = format!("{}/_bulk", self.config.url.trim_end_matches('/'));
        let response = self
            .request(
                self.http
                    .post(url)
                    .header("content-type", "application/x-ndjson"),
            )
            .body(payload)
            .send()
            .await
            .context("post opensearch bulk request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("opensearch bulk index failed: {status} {body}"));
        }

        let body: OpenSearchBulkResponse = response
            .json()
            .await
            .context("decode opensearch bulk response")?;
        if body.errors {
            return Err(anyhow!("opensearch bulk index returned item errors"));
        }

        Ok(())
    }

    fn index_name(&self, timestamp: &str) -> anyhow::Result<String> {
        let ts = DateTime::parse_from_rfc3339(timestamp)
            .with_context(|| format!("parse normalized timestamp {timestamp}"))?
            .with_timezone(&Utc);
        Ok(format!(
            "{}-logs-{}",
            self.config.index_prefix,
            ts.format("%Y.%m.%d")
        ))
    }

    fn request(&self, builder: RequestBuilder) -> RequestBuilder {
        match (&self.config.username, &self.config.password) {
            (Some(username), Some(password)) => {
                builder.basic_auth(username.to_string(), Some(password.to_string()))
            }
            _ => builder,
        }
    }
}

#[derive(Clone)]
pub struct ClickHouseClient {
    http: Client,
    config: ClickHouseConfig,
}

impl ClickHouseClient {
    pub fn new(config: ClickHouseConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    pub async fn ensure_schema(&self) -> anyhow::Result<()> {
        self.execute_ddl(&format!(
            "CREATE DATABASE IF NOT EXISTS {}",
            quote_identifier(&self.config.database)
        ))
        .await?;

        self.execute_ddl(&format!(
            "CREATE TABLE IF NOT EXISTS {}.{} (
                id String,
                timestamp DateTime64(3, 'UTC'),
                host String,
                agent_id String,
                source_type String,
                source String,
                service String,
                severity String,
                message String,
                fingerprint String,
                labels_json String,
                fields_json String,
                raw String
            )
            ENGINE = MergeTree
            PARTITION BY toDate(timestamp)
            ORDER BY (timestamp, id)",
            quote_identifier(&self.config.database),
            quote_identifier(&self.config.table)
        ))
        .await
    }

    pub async fn insert_events(&self, events: &[NormalizedLogEvent]) -> anyhow::Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let url = format!(
            "{}?query={}",
            self.config.dsn.trim_end_matches('/'),
            urlencoding::encode(&format!(
                "INSERT INTO {}.{} FORMAT JSONEachRow",
                quote_identifier(&self.config.database),
                quote_identifier(&self.config.table)
            ))
        );

        let mut body = String::new();
        for event in events {
            let row = json!({
                "id": event.id,
                "timestamp": normalize_ch_timestamp(&event.timestamp)?,
                "host": event.host,
                "agent_id": event.agent_id,
                "source_type": event.source_type,
                "source": event.source,
                "service": event.service,
                "severity": event.severity,
                "message": event.message,
                "fingerprint": event.fingerprint,
                "labels_json": serde_json::to_string(&event.labels)?,
                "fields_json": serde_json::to_string(&event.fields)?,
                "raw": event.raw,
            });
            body.push_str(&serde_json::to_string(&row)?);
            body.push('\n');
        }

        let response = self
            .http
            .post(url)
            .header("content-type", "application/x-ndjson")
            .body(body)
            .send()
            .await
            .context("post clickhouse insert")?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow!("clickhouse insert failed: {status} {body}"))
        }
    }

    async fn execute_ddl(&self, query: &str) -> anyhow::Result<()> {
        let response = self
            .http
            .post(self.config.dsn.trim_end_matches('/'))
            .body(query.to_string())
            .send()
            .await
            .with_context(|| format!("execute clickhouse query: {query}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow!("clickhouse ddl failed: {status} {body}"))
        }
    }
}

fn quote_identifier(value: &str) -> String {
    format!("`{}`", value.replace('`', ""))
}

fn normalize_ch_timestamp(value: &str) -> anyhow::Result<String> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("parse clickhouse timestamp {value}"))?
        .with_timezone(&Utc);
    Ok(parsed.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
}

#[derive(Debug, Deserialize)]
struct OpenSearchBulkResponse {
    #[serde(default)]
    errors: bool,
}

#[cfg(test)]
mod tests {
    use super::normalize_ch_timestamp;

    #[test]
    fn normalizes_clickhouse_timestamp_format() {
        let normalized = normalize_ch_timestamp("2026-03-21T20:28:20.245Z").unwrap();
        assert_eq!(normalized, "2026-03-21 20:28:20.245");
    }
}
