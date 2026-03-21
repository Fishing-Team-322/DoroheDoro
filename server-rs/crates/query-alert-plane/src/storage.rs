use anyhow::{anyhow, Context};
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use serde_json::{json, Value};

use common::proto::query;

use crate::{
    anomaly::ResolvedLogFilter,
    config::{ClickHouseConfig, OpenSearchConfig},
};

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

    pub async fn ping(&self) -> anyhow::Result<()> {
        let response = self
            .request(self.http.get(self.config.url.trim_end_matches('/')))
            .send()
            .await
            .context("ping opensearch")?;
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow!("opensearch ping failed: {status} {body}"))
        }
    }

    pub async fn search_logs(
        &self,
        filter: &query::LogQueryFilter,
        limit: u32,
        offset: u64,
    ) -> anyhow::Result<query::SearchLogsResponse> {
        let body = json!({
            "from": offset,
            "size": limit,
            "sort": [{ "timestamp": { "order": "desc" } }],
            "query": build_opensearch_query(filter),
        });
        let response = self.search(body).await?;
        let items = response
            .hits
            .hits
            .into_iter()
            .map(hit_to_log_event)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(query::SearchLogsResponse {
            items,
            total: response.hits.total.value.max(0) as u64,
            limit,
            offset,
            took_ms: response.took.max(0) as u32,
        })
    }

    pub async fn get_log_event(
        &self,
        event_id: &str,
    ) -> anyhow::Result<query::GetLogEventResponse> {
        let body = json!({
            "size": 1,
            "query": { "ids": { "values": [event_id] } },
        });
        let response = self.search(body).await?;
        let item = response
            .hits
            .hits
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("log event {event_id} not found"))
            .and_then(hit_to_log_event)?;
        Ok(query::GetLogEventResponse {
            item: Some(item),
            took_ms: response.took.max(0) as u32,
        })
    }

    pub async fn get_log_context(
        &self,
        event_id: &str,
        before: u32,
        after: u32,
    ) -> anyhow::Result<query::GetLogContextResponse> {
        let anchor = self.get_log_event(event_id).await?;
        let Some(anchor_item) = anchor.item.clone() else {
            return Err(anyhow!("anchor log event missing"));
        };

        let before_response = self
            .search(json!({
                "size": before,
                "sort": [{ "timestamp": { "order": "desc" } }],
                "query": {
                    "bool": {
                        "must": [
                            { "term": { "host": anchor_item.host } },
                            { "range": { "timestamp": { "lt": anchor_item.timestamp } } }
                        ]
                    }
                }
            }))
            .await?;
        let mut before_items = before_response
            .hits
            .hits
            .into_iter()
            .map(hit_to_log_event)
            .collect::<anyhow::Result<Vec<_>>>()?;
        before_items.reverse();

        let after_response = self
            .search(json!({
                "size": after,
                "sort": [{ "timestamp": { "order": "asc" } }],
                "query": {
                    "bool": {
                        "must": [
                            { "term": { "host": anchor_item.host.clone() } },
                            { "range": { "timestamp": { "gt": anchor_item.timestamp.clone() } } }
                        ]
                    }
                }
            }))
            .await?;
        let after_items = after_response
            .hits
            .hits
            .into_iter()
            .map(hit_to_log_event)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(query::GetLogContextResponse {
            anchor: Some(anchor_item),
            before: before_items,
            after: after_items,
            took_ms: anchor.took_ms
                + before_response.took.max(0) as u32
                + after_response.took.max(0) as u32,
        })
    }

    async fn search(&self, body: Value) -> anyhow::Result<OpenSearchSearchResponse> {
        let url = format!(
            "{}/{}-logs-*/_search",
            self.config.url.trim_end_matches('/'),
            self.config.index_prefix
        );
        let response = self
            .request(self.http.post(url))
            .json(&body)
            .send()
            .await
            .context("execute opensearch search")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("opensearch search failed: {status} {body}"));
        }

        response
            .json::<OpenSearchSearchResponse>()
            .await
            .context("decode opensearch search response")
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

    pub async fn ping(&self) -> anyhow::Result<()> {
        self.query_json("SELECT 1 AS ok").await.map(|_| ())
    }

    pub async fn histogram(
        &self,
        filter: &query::LogQueryFilter,
    ) -> anyhow::Result<query::HistogramResponse> {
        let where_sql = build_clickhouse_where(filter);
        let sql = format!(
            "SELECT formatDateTime(toStartOfHour(timestamp), '%Y-%m-%dT%H:00:00Z') AS bucket,
                    count() AS count
             FROM {}.{}
             {}
             GROUP BY bucket
             ORDER BY bucket ASC",
            quote_identifier(&self.config.database),
            quote_identifier(&self.config.table),
            where_sql
        );
        let rows = self.query_json(&sql).await?;
        Ok(query::HistogramResponse {
            items: rows
                .into_iter()
                .map(|row| query::HistogramBucket {
                    bucket: row["bucket"].as_str().unwrap_or_default().to_string(),
                    count: as_u64(&row["count"]),
                })
                .collect(),
        })
    }

    pub async fn count_buckets(
        &self,
        filter: &query::LogQueryFilter,
        field: &'static str,
        limit: u32,
    ) -> anyhow::Result<query::CountBucketsResponse> {
        let where_sql = build_clickhouse_where(filter);
        let sql = format!(
            "SELECT {field} AS key, count() AS count
             FROM {}.{}
             {}
             GROUP BY key
             ORDER BY count DESC, key ASC
             LIMIT {}",
            quote_identifier(&self.config.database),
            quote_identifier(&self.config.table),
            where_sql,
            limit.max(1)
        );
        let rows = self.query_json(&sql).await?;
        Ok(query::CountBucketsResponse {
            items: rows
                .into_iter()
                .map(|row| query::CountBucket {
                    key: row["key"].as_str().unwrap_or_default().to_string(),
                    count: as_u64(&row["count"]),
                })
                .collect(),
        })
    }

    pub async fn heatmap(
        &self,
        filter: &query::LogQueryFilter,
    ) -> anyhow::Result<query::HeatmapResponse> {
        let where_sql = build_clickhouse_where(filter);
        let sql = format!(
            "SELECT formatDateTime(toStartOfHour(timestamp), '%Y-%m-%dT%H:00:00Z') AS bucket,
                    severity,
                    count() AS count
             FROM {}.{}
             {}
             GROUP BY bucket, severity
             ORDER BY bucket ASC, severity ASC",
            quote_identifier(&self.config.database),
            quote_identifier(&self.config.table),
            where_sql
        );
        let rows = self.query_json(&sql).await?;
        Ok(query::HeatmapResponse {
            items: rows
                .into_iter()
                .map(|row| query::HeatmapBucket {
                    bucket: row["bucket"].as_str().unwrap_or_default().to_string(),
                    severity: row["severity"].as_str().unwrap_or_default().to_string(),
                    count: as_u64(&row["count"]),
                })
                .collect(),
        })
    }

    pub async fn top_patterns(
        &self,
        filter: &query::LogQueryFilter,
        limit: u32,
    ) -> anyhow::Result<query::TopPatternsResponse> {
        let where_sql = build_clickhouse_where(filter);
        let sql = format!(
            "SELECT fingerprint,
                    any(message) AS sample_message,
                    count() AS count
             FROM {}.{}
             {}
             GROUP BY fingerprint
             ORDER BY count DESC
             LIMIT {}",
            quote_identifier(&self.config.database),
            quote_identifier(&self.config.table),
            where_sql,
            limit.max(1)
        );
        let rows = self.query_json(&sql).await?;
        Ok(query::TopPatternsResponse {
            items: rows
                .into_iter()
                .map(|row| query::PatternBucket {
                    fingerprint: row["fingerprint"].as_str().unwrap_or_default().to_string(),
                    sample_message: row["sample_message"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    count: as_u64(&row["count"]),
                })
                .collect(),
        })
    }

    pub async fn ingested_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<u64> {
        let sql = format!(
            "SELECT count() AS count
             FROM {}.{}
             WHERE timestamp >= {}
               AND timestamp <= {}",
            quote_identifier(&self.config.database),
            quote_identifier(&self.config.table),
            clickhouse_timestamp_expr(&from.to_rfc3339_opts(SecondsFormat::Millis, true)),
            clickhouse_timestamp_expr(&to.to_rfc3339_opts(SecondsFormat::Millis, true))
        );
        let rows = self.query_json(&sql).await?;
        Ok(rows
            .first()
            .map(|row| as_u64(&row["count"]))
            .unwrap_or_default())
    }

    pub async fn count_events(
        &self,
        filter: &ResolvedLogFilter,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<u64> {
        let where_sql = build_anomaly_where(filter, from, to);
        let sql = format!(
            "SELECT count() AS count
             FROM {}.{}
             WHERE {}",
            quote_identifier(&self.config.database),
            quote_identifier(&self.config.table),
            where_sql
        );
        let rows = self.query_json(&sql).await?;
        Ok(rows
            .first()
            .map(|row| as_u64(&row["count"]))
            .unwrap_or_default())
    }

    pub async fn matching_count(
        &self,
        host: &str,
        service: &str,
        severity: &str,
        fingerprint: &str,
        query_fragment: Option<&str>,
        since: DateTime<Utc>,
    ) -> anyhow::Result<u64> {
        let mut clauses = vec![
            format!("host = {}", sql_string(host)),
            format!("service = {}", sql_string(service)),
            format!("severity = {}", sql_string(severity)),
            format!("fingerprint = {}", sql_string(fingerprint)),
            format!(
                "timestamp >= {}",
                clickhouse_timestamp_expr(&since.to_rfc3339_opts(SecondsFormat::Millis, true))
            ),
        ];
        if let Some(query_fragment) = query_fragment.filter(|value| !value.trim().is_empty()) {
            clauses.push(format!(
                "positionCaseInsensitive(message, {}) > 0",
                sql_string(query_fragment.trim())
            ));
        }

        let sql = format!(
            "SELECT count() AS count
             FROM {}.{}
             WHERE {}",
            quote_identifier(&self.config.database),
            quote_identifier(&self.config.table),
            clauses.join(" AND ")
        );
        let rows = self.query_json(&sql).await?;
        Ok(rows
            .first()
            .map(|row| as_u64(&row["count"]))
            .unwrap_or_default())
    }

    async fn query_json(&self, sql: &str) -> anyhow::Result<Vec<Value>> {
        let response = self
            .http
            .post(self.config.dsn.trim_end_matches('/'))
            .body(format!("{sql} FORMAT JSON"))
            .send()
            .await
            .with_context(|| format!("execute clickhouse query: {sql}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("clickhouse query failed: {status} {body}"));
        }

        let payload: ClickHouseJsonResponse = response
            .json()
            .await
            .context("decode clickhouse json response")?;
        Ok(payload.data)
    }
}

fn build_opensearch_query(filter: &query::LogQueryFilter) -> Value {
    let mut must = Vec::new();
    if !filter.query.trim().is_empty() {
        must.push(json!({
            "simple_query_string": {
                "query": filter.query,
                "fields": ["message^3", "service^2", "host", "fingerprint"]
            }
        }));
    }
    if !filter.host.trim().is_empty() {
        must.push(json!({ "term": { "host": filter.host } }));
    }
    if !filter.service.trim().is_empty() {
        must.push(json!({ "term": { "service": filter.service } }));
    }
    if !filter.severity.trim().is_empty() {
        must.push(json!({ "term": { "severity": filter.severity } }));
    }

    let mut filter_clauses = Vec::new();
    if !filter.from.trim().is_empty() || !filter.to.trim().is_empty() {
        let mut range = serde_json::Map::new();
        if !filter.from.trim().is_empty() {
            range.insert("gte".to_string(), Value::String(filter.from.clone()));
        }
        if !filter.to.trim().is_empty() {
            range.insert("lte".to_string(), Value::String(filter.to.clone()));
        }
        filter_clauses.push(json!({ "range": { "timestamp": Value::Object(range) } }));
    }

    if must.is_empty() && filter_clauses.is_empty() {
        json!({ "match_all": {} })
    } else {
        json!({
            "bool": {
                "must": must,
                "filter": filter_clauses,
            }
        })
    }
}

fn build_clickhouse_where(filter: &query::LogQueryFilter) -> String {
    let mut clauses = Vec::new();
    if !filter.host.trim().is_empty() {
        clauses.push(format!("host = {}", sql_string(filter.host.trim())));
    }
    if !filter.service.trim().is_empty() {
        clauses.push(format!("service = {}", sql_string(filter.service.trim())));
    }
    if !filter.severity.trim().is_empty() {
        clauses.push(format!("severity = {}", sql_string(filter.severity.trim())));
    }
    if !filter.query.trim().is_empty() {
        clauses.push(format!(
            "positionCaseInsensitive(message, {}) > 0",
            sql_string(filter.query.trim())
        ));
    }
    if !filter.from.trim().is_empty() {
        clauses.push(format!(
            "timestamp >= {}",
            clickhouse_timestamp_expr(filter.from.trim())
        ));
    }
    if !filter.to.trim().is_empty() {
        clauses.push(format!(
            "timestamp <= {}",
            clickhouse_timestamp_expr(filter.to.trim())
        ));
    }

    if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    }
}

fn build_anomaly_where(
    filter: &ResolvedLogFilter,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> String {
    let mut clauses = vec![
        format!(
            "timestamp >= {}",
            clickhouse_timestamp_expr(&from.to_rfc3339_opts(SecondsFormat::Millis, true))
        ),
        format!(
            "timestamp <= {}",
            clickhouse_timestamp_expr(&to.to_rfc3339_opts(SecondsFormat::Millis, true))
        ),
    ];

    if let Some(host) = filter.host.as_deref() {
        clauses.push(format!("host = {}", sql_string(host)));
    }
    if let Some(service) = filter.service.as_deref() {
        clauses.push(format!("service = {}", sql_string(service)));
    }
    if let Some(severity) = filter.severity.as_deref() {
        clauses.push(format!("severity = {}", sql_string(severity)));
    }
    if let Some(fingerprint) = filter.fingerprint.as_deref() {
        clauses.push(format!("fingerprint = {}", sql_string(fingerprint)));
    }
    if let Some(query) = filter.query.as_deref() {
        clauses.push(format!(
            "positionCaseInsensitive(message, {}) > 0",
            sql_string(query)
        ));
    }

    clauses.join(" AND ")
}

fn quote_identifier(value: &str) -> String {
    format!("`{}`", value.replace('`', ""))
}

fn sql_string(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn clickhouse_timestamp_expr(value: &str) -> String {
    format!("parseDateTime64BestEffort({}, 3, 'UTC')", sql_string(value))
}

fn as_u64(value: &Value) -> u64 {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|raw| raw.try_into().ok()))
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<u64>().ok()))
        .unwrap_or_default()
}

fn hit_to_log_event(hit: OpenSearchHit) -> anyhow::Result<query::LogEvent> {
    let source = hit.source;
    Ok(query::LogEvent {
        id: source.id.or(Some(hit.id)).unwrap_or_default(),
        timestamp: source.timestamp.unwrap_or_default(),
        host: source.host.unwrap_or_default(),
        agent_id: source.agent_id.unwrap_or_default(),
        source_type: source.source_type.unwrap_or_default(),
        source: source.source.unwrap_or_default(),
        service: source.service.unwrap_or_default(),
        severity: source.severity.unwrap_or_default(),
        message: source.message.unwrap_or_default(),
        fingerprint: source.fingerprint.unwrap_or_default(),
        labels: source.labels.unwrap_or_default(),
        fields_json: serde_json::to_string(&source.fields.unwrap_or_else(|| json!({})))?,
        raw: source.raw.unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
struct OpenSearchSearchResponse {
    #[serde(default)]
    took: i64,
    hits: OpenSearchHits,
}

#[derive(Debug, Deserialize)]
struct OpenSearchHits {
    total: OpenSearchTotal,
    #[serde(default)]
    hits: Vec<OpenSearchHit>,
}

#[derive(Debug, Deserialize)]
struct OpenSearchTotal {
    value: i64,
}

#[derive(Debug, Deserialize)]
struct OpenSearchHit {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "_source")]
    source: OpenSearchLogSource,
}

#[derive(Debug, Deserialize)]
struct OpenSearchLogSource {
    id: Option<String>,
    timestamp: Option<String>,
    host: Option<String>,
    agent_id: Option<String>,
    source_type: Option<String>,
    source: Option<String>,
    service: Option<String>,
    severity: Option<String>,
    message: Option<String>,
    fingerprint: Option<String>,
    labels: Option<std::collections::BTreeMap<String, String>>,
    fields: Option<Value>,
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClickHouseJsonResponse {
    #[serde(default)]
    data: Vec<Value>,
}
