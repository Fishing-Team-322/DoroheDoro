use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use anyhow::Context;
use async_nats::Client;
use chrono::{DateTime, Duration, Utc};
use common::{
    json::{AlertStreamEvent, AuditAppendEvent, NormalizedLogEvent},
    nats_subjects::{AUDIT_EVENTS_APPEND, UI_STREAM_ALERTS},
    proto::{
        alerts, query,
        runtime::{AuditContext, PagingRequest, PagingResponse},
    },
    AppError, AppResult,
};
use serde_json::{json, Value};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    config::DetectionConfig,
    models::{format_ts, AlertRuleCondition, AlertRuleRecord},
    pipeline::DetectionMode,
    repository::QueryAlertRepository,
    storage::{ClickHouseClient, OpenSearchClient},
};

#[derive(Debug, Clone)]
pub struct AuditActor {
    pub actor_id: String,
    pub actor_type: String,
    pub request_id: String,
    pub reason: String,
}

impl AuditActor {
    pub fn from_proto(
        correlation_id: &str,
        audit: Option<AuditContext>,
        default_reason: &str,
    ) -> Self {
        let audit = audit.unwrap_or_default();
        Self {
            actor_id: non_empty_or(&audit.actor_id, "system"),
            actor_type: non_empty_or(&audit.actor_type, "system"),
            request_id: non_empty_or(&audit.request_id, correlation_id),
            reason: non_empty_or(&audit.reason, default_reason),
        }
    }
}

struct DetectionRuntime {
    config: DetectionConfig,
    metrics: DetectionMetrics,
}

#[derive(Default)]
struct DetectionMetrics {
    signals_total: AtomicU64,
    anomalies_total: AtomicU64,
    resolved_total: AtomicU64,
    rejected_total: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct DetectionStatusSnapshot {
    pub mode: DetectionMode,
    pub safe_mode: bool,
    pub signals_total: u64,
    pub anomalies_total: u64,
    pub resolved_total: u64,
    pub rejected_total: u64,
}

pub struct QueryAlertService {
    repo: QueryAlertRepository,
    nats: Client,
    opensearch: OpenSearchClient,
    clickhouse: ClickHouseClient,
    detection: DetectionRuntime,
    ready: Arc<AtomicBool>,
}

impl QueryAlertService {
    pub fn new(
        repo: QueryAlertRepository,
        nats: Client,
        opensearch: OpenSearchClient,
        clickhouse: ClickHouseClient,
        detection_config: DetectionConfig,
    ) -> Self {
        Self {
            repo,
            nats,
            opensearch,
            clickhouse,
            detection: DetectionRuntime {
                config: detection_config,
                metrics: DetectionMetrics::default(),
            },
            ready: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        self.repo.ping().await.context("ping postgres")?;
        self.opensearch.ping().await.context("ping opensearch")?;
        self.clickhouse.ping().await.context("ping clickhouse")?;
        self.ready.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn detection_snapshot(&self) -> DetectionStatusSnapshot {
        let metrics = &self.detection.metrics;
        DetectionStatusSnapshot {
            mode: self.detection.config.mode,
            safe_mode: self.detection.config.safe_mode,
            signals_total: metrics.signals_total.load(Ordering::Relaxed),
            anomalies_total: metrics.anomalies_total.load(Ordering::Relaxed),
            resolved_total: metrics.resolved_total.load(Ordering::Relaxed),
            rejected_total: metrics.rejected_total.load(Ordering::Relaxed),
        }
    }

    fn record_signal(&self) {
        self.detection
            .metrics
            .signals_total
            .fetch_add(1, Ordering::Relaxed);
    }

    fn record_anomaly(&self) {
        self.detection
            .metrics
            .anomalies_total
            .fetch_add(1, Ordering::Relaxed);
    }

    fn record_resolution(&self) {
        self.detection
            .metrics
            .resolved_total
            .fetch_add(1, Ordering::Relaxed);
    }

    fn record_rejected(&self) {
        self.detection
            .metrics
            .rejected_total
            .fetch_add(1, Ordering::Relaxed);
    }

    async fn record_baseline(
        &self,
        host: &str,
        service: &str,
        signal_kind: &str,
        observation: f64,
    ) -> AppResult<()> {
        let window = self.detection.config.medium_window_min as i32;
        let existing = self
            .repo
            .get_anomaly_baseline(host, service, signal_kind, window)
            .await
            .map_err(map_db_error)?;

        let (samples, mean, stddev) = if let Some(record) = existing {
            let prev_samples = record.samples.max(0) as f64;
            let new_samples = prev_samples + 1.0;
            let new_mean = ((record.mean * prev_samples) + observation) / new_samples;
            (new_samples as i32, new_mean, record.stddev)
        } else {
            (1, observation, 0.0)
        };

        self.repo
            .upsert_anomaly_baseline(
                host,
                service,
                signal_kind,
                window,
                samples,
                mean,
                stddev,
                None,
                &json!({ "last_observation": observation }),
            )
            .await
            .map_err(map_db_error)?;
        Ok(())
    }

    pub async fn search_logs(
        &self,
        request: query::SearchLogsRequest,
    ) -> AppResult<query::SearchLogsResponse> {
        self.opensearch
            .search_logs(
                &request.filter.unwrap_or_default(),
                sanitize_limit(request.limit, 50, 200),
                request.offset,
            )
            .await
            .map_err(map_integration_error)
    }

    pub async fn get_log_event(
        &self,
        request: query::GetLogEventRequest,
    ) -> AppResult<query::GetLogEventResponse> {
        if request.event_id.trim().is_empty() {
            return Err(AppError::invalid_argument("event_id is required"));
        }
        self.opensearch
            .get_log_event(request.event_id.trim())
            .await
            .map_err(|error| {
                if error.to_string().contains("not found") {
                    AppError::not_found(error.to_string())
                } else {
                    map_integration_error(error)
                }
            })
    }

    pub async fn get_log_context(
        &self,
        request: query::GetLogContextRequest,
    ) -> AppResult<query::GetLogContextResponse> {
        if request.event_id.trim().is_empty() {
            return Err(AppError::invalid_argument("event_id is required"));
        }
        self.opensearch
            .get_log_context(
                request.event_id.trim(),
                request.before.min(200),
                request.after.min(200),
            )
            .await
            .map_err(map_integration_error)
    }

    pub async fn histogram(
        &self,
        request: query::LogAnalyticsRequest,
    ) -> AppResult<query::HistogramResponse> {
        self.clickhouse
            .histogram(&request.filter.unwrap_or_default())
            .await
            .map_err(map_integration_error)
    }

    pub async fn severity(
        &self,
        request: query::LogAnalyticsRequest,
    ) -> AppResult<query::CountBucketsResponse> {
        self.clickhouse
            .count_buckets(
                &request.filter.unwrap_or_default(),
                "severity",
                sanitize_limit(request.limit, 10, 50),
            )
            .await
            .map_err(map_integration_error)
    }

    pub async fn top_hosts(
        &self,
        request: query::LogAnalyticsRequest,
    ) -> AppResult<query::CountBucketsResponse> {
        self.clickhouse
            .count_buckets(
                &request.filter.unwrap_or_default(),
                "host",
                sanitize_limit(request.limit, 10, 100),
            )
            .await
            .map_err(map_integration_error)
    }

    pub async fn top_services(
        &self,
        request: query::LogAnalyticsRequest,
    ) -> AppResult<query::CountBucketsResponse> {
        self.clickhouse
            .count_buckets(
                &request.filter.unwrap_or_default(),
                "service",
                sanitize_limit(request.limit, 10, 100),
            )
            .await
            .map_err(map_integration_error)
    }

    pub async fn heatmap(
        &self,
        request: query::LogAnalyticsRequest,
    ) -> AppResult<query::HeatmapResponse> {
        self.clickhouse
            .heatmap(&request.filter.unwrap_or_default())
            .await
            .map_err(map_integration_error)
    }

    pub async fn top_patterns(
        &self,
        request: query::LogAnalyticsRequest,
    ) -> AppResult<query::TopPatternsResponse> {
        self.clickhouse
            .top_patterns(
                &request.filter.unwrap_or_default(),
                sanitize_limit(request.limit, 10, 100),
            )
            .await
            .map_err(map_integration_error)
    }

    pub async fn list_log_anomalies(
        &self,
        request: query::ListLogAnomaliesRequest,
    ) -> AppResult<query::ListLogAnomaliesResponse> {
        let filter = request.filter.unwrap_or_default();
        let limit = sanitize_limit(request.limit, 50, 200);
        let (items, total) = self
            .repo
            .list_log_anomaly_projections(
                optional_trimmed(&filter.host),
                optional_trimmed(&filter.service),
                optional_trimmed(&filter.severity),
                limit,
                request.offset,
            )
            .await
            .map_err(map_db_error)?;

        Ok(query::ListLogAnomaliesResponse {
            items: items
                .into_iter()
                .map(|item| item.into_log_projection())
                .collect(),
            total,
            limit,
            offset: request.offset,
        })
    }

    pub async fn dashboard_overview(
        &self,
        request: query::DashboardOverviewRequest,
    ) -> AppResult<query::DashboardOverviewResponse> {
        let (from, to) = parse_time_window(&request.from, &request.to)?;
        let analytics_filter = query::LogQueryFilter {
            query: String::new(),
            from: format_ts(from),
            to: format_ts(to),
            host: String::new(),
            service: String::new(),
            severity: String::new(),
        };

        let active_hosts = self
            .repo
            .count_active_hosts_since(Utc::now() - Duration::minutes(15))
            .await
            .map_err(map_db_error)?;
        let open_alerts = self.repo.count_open_alerts().await.map_err(map_db_error)?;
        let deployment_jobs = self
            .repo
            .count_deployment_jobs_since(from)
            .await
            .map_err(map_db_error)?;
        let ingested_events = self
            .clickhouse
            .ingested_events(from, to)
            .await
            .map_err(map_integration_error)?;

        let recent_activity = self
            .repo
            .recent_activity(8)
            .await
            .map_err(map_db_error)?
            .into_iter()
            .map(|item| item.into_dashboard_item())
            .collect();
        let log_histogram = self
            .clickhouse
            .histogram(&analytics_filter)
            .await
            .map_err(map_integration_error)?
            .items;
        let top_services = self
            .clickhouse
            .count_buckets(&analytics_filter, "service", 6)
            .await
            .map_err(map_integration_error)?
            .items;
        let top_hosts = self
            .clickhouse
            .count_buckets(&analytics_filter, "host", 6)
            .await
            .map_err(map_integration_error)?
            .items;

        Ok(query::DashboardOverviewResponse {
            metrics: vec![
                metric("active_hosts", "Active hosts", active_hosts),
                metric("open_alerts", "Open alerts", open_alerts),
                metric("deployment_jobs", "Deployments", deployment_jobs),
                metric("ingested_events", "Ingested events", ingested_events),
            ],
            active_hosts,
            open_alerts,
            deployment_jobs,
            ingested_events,
            recent_activity,
            log_histogram,
            top_services,
            top_hosts,
        })
    }

    pub async fn list_alert_instances(
        &self,
        request: alerts::ListAlertInstancesRequest,
    ) -> AppResult<alerts::ListAlertInstancesResponse> {
        let (limit, offset) = paging_or_default(request.paging, 50, 200);
        let (items, total) = self
            .repo
            .list_alert_instances(
                optional_trimmed(&request.status),
                optional_trimmed(&request.severity),
                optional_trimmed(&request.host),
                optional_trimmed(&request.service),
                limit,
                offset,
            )
            .await
            .map_err(map_db_error)?;

        Ok(alerts::ListAlertInstancesResponse {
            items: items.into_iter().map(|item| item.into_proto()).collect(),
            paging: Some(PagingResponse {
                limit,
                offset,
                total,
            }),
        })
    }

    pub async fn get_alert_instance(
        &self,
        request: alerts::GetAlertInstanceRequest,
    ) -> AppResult<alerts::GetAlertInstanceResponse> {
        let alert_instance_id = parse_uuid("alert_instance_id", &request.alert_instance_id)?;
        let item = self
            .repo
            .get_alert_instance(alert_instance_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| {
                AppError::not_found(format!("alert instance {alert_instance_id} not found"))
            })?;
        Ok(alerts::GetAlertInstanceResponse {
            item: Some(item.into_proto()),
        })
    }

    pub async fn list_alert_rules(
        &self,
        request: alerts::ListAlertRulesRequest,
    ) -> AppResult<alerts::ListAlertRulesResponse> {
        let (limit, offset) = paging_or_default(request.paging, 50, 200);
        let (items, total) = self
            .repo
            .list_alert_rules(
                optional_trimmed(&request.query),
                optional_trimmed(&request.status),
                limit,
                offset,
            )
            .await
            .map_err(map_db_error)?;

        Ok(alerts::ListAlertRulesResponse {
            items: items.into_iter().map(|item| item.into_proto()).collect(),
            paging: Some(PagingResponse {
                limit,
                offset,
                total,
            }),
        })
    }

    pub async fn get_alert_rule(
        &self,
        request: alerts::GetAlertRuleRequest,
    ) -> AppResult<alerts::GetAlertRuleResponse> {
        let alert_rule_id = parse_uuid("alert_rule_id", &request.alert_rule_id)?;
        let item = self
            .repo
            .get_alert_rule(alert_rule_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("alert rule {alert_rule_id} not found")))?;
        Ok(alerts::GetAlertRuleResponse {
            item: Some(item.into_proto()),
        })
    }

    pub async fn create_alert_rule(
        &self,
        request: alerts::CreateAlertRuleRequest,
    ) -> AppResult<alerts::AlertRuleMutationResponse> {
        let audit =
            AuditActor::from_proto(&request.correlation_id, request.audit, "alert rule created");
        let condition = parse_condition_json(&request.condition_json)?;
        validate_alert_rule_input(
            &request.name,
            &request.severity,
            &request.scope_type,
            optional_trimmed(&request.scope_id),
            &condition,
        )?;

        let severity = normalize_severity(&request.severity);
        let scope_type = normalize_scope_type(&request.scope_type);
        let created = self
            .repo
            .create_alert_rule(
                request.name.trim(),
                request.description.trim(),
                &severity,
                &scope_type,
                optional_trimmed(&request.scope_id),
                &serde_json::to_value(&condition).map_err(|error| {
                    AppError::internal(format!("serialize alert condition: {error}"))
                })?,
                &audit.actor_id,
            )
            .await
            .map_err(map_db_error)?;

        self.publish_audit_event(AuditAppendEvent {
            event_type: "alert.rule.create".to_string(),
            entity_type: "alert_rule".to_string(),
            entity_id: created.id.to_string(),
            actor_id: audit.actor_id,
            actor_type: audit.actor_type,
            request_id: audit.request_id,
            reason: audit.reason,
            payload_json: json!({
                "name": created.name,
                "severity": created.severity,
                "scope_type": created.scope_type,
                "scope_id": created.scope_id,
            }),
            created_at: None,
        })
        .await;

        Ok(alerts::AlertRuleMutationResponse {
            item: Some(created.into_proto()),
        })
    }

    pub async fn update_alert_rule(
        &self,
        request: alerts::UpdateAlertRuleRequest,
    ) -> AppResult<alerts::AlertRuleMutationResponse> {
        let audit =
            AuditActor::from_proto(&request.correlation_id, request.audit, "alert rule updated");
        let alert_rule_id = parse_uuid("alert_rule_id", &request.alert_rule_id)?;
        let condition = parse_condition_json(&request.condition_json)?;
        validate_alert_rule_input(
            &request.name,
            &request.severity,
            &request.scope_type,
            optional_trimmed(&request.scope_id),
            &condition,
        )?;

        let severity = normalize_severity(&request.severity);
        let scope_type = normalize_scope_type(&request.scope_type);
        let updated = self
            .repo
            .update_alert_rule(
                alert_rule_id,
                request.name.trim(),
                request.description.trim(),
                &normalize_rule_status(&request.status)?,
                &severity,
                &scope_type,
                optional_trimmed(&request.scope_id),
                &serde_json::to_value(&condition).map_err(|error| {
                    AppError::internal(format!("serialize alert condition: {error}"))
                })?,
                &audit.actor_id,
            )
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("alert rule {alert_rule_id} not found")))?;

        self.publish_audit_event(AuditAppendEvent {
            event_type: "alert.rule.update".to_string(),
            entity_type: "alert_rule".to_string(),
            entity_id: updated.id.to_string(),
            actor_id: audit.actor_id,
            actor_type: audit.actor_type,
            request_id: audit.request_id,
            reason: audit.reason,
            payload_json: json!({
                "name": updated.name,
                "severity": updated.severity,
                "status": updated.status,
                "scope_type": updated.scope_type,
                "scope_id": updated.scope_id,
            }),
            created_at: None,
        })
        .await;

        Ok(alerts::AlertRuleMutationResponse {
            item: Some(updated.into_proto()),
        })
    }

    pub async fn handle_normalized_event(&self, event: NormalizedLogEvent) -> AppResult<()> {
        self.record_signal();
        let event_ts = parse_event_timestamp(&event.timestamp)?;
        self.record_baseline(&event.host, &event.service, "logs", 1.0)
            .await?;
        let rules = self.repo.list_active_rules().await.map_err(map_db_error)?;
        for rule in rules {
            let condition = parse_condition_value(&rule.condition_json)?;
            if !event_matches_rule(&rule, &condition, &event) {
                continue;
            }

            let since = event_ts - Duration::minutes(condition.window_minutes as i64);
            let hit_count = self
                .clickhouse
                .matching_count(
                    &event.host,
                    &event.service,
                    &event.severity,
                    &event.fingerprint,
                    condition.query.as_deref(),
                    since,
                )
                .await
                .map_err(map_integration_error)?;
            if hit_count < condition.threshold {
                continue;
            }

            let detection_mode = self.detection.config.mode;
            let correlation_key = format!(
                "rule:{}:{}:{}:{}",
                rule.id, event.host, event.service, event.fingerprint
            );
            let signal_meta = json!({
                "signal_id": event.id,
                "timestamp": event.timestamp,
                "severity": event.severity,
                "message": event.message,
                "source_type": event.source_type,
            });
            let evidence_json = json!({
                "hit_count": hit_count,
                "threshold": condition.threshold,
                "window_minutes": condition.window_minutes,
                "fingerprint": event.fingerprint,
            });
            let score = if condition.threshold == 0 {
                0.0
            } else {
                (hit_count as f64) / (condition.threshold as f64)
            };

            self.repo
                .insert_anomaly_score(
                    Some(rule.id),
                    "log_burst",
                    "logs",
                    &event.host,
                    &event.service,
                    &correlation_key,
                    detection_mode.as_str(),
                    &event.id,
                    score,
                    condition.threshold as f64,
                    &evidence_json,
                )
                .await
                .map_err(map_db_error)?;

            let payload_json = json!({
                "event_id": event.id,
                "message": event.message,
                "threshold": condition.threshold,
                "window_minutes": condition.window_minutes,
                "hit_count": hit_count,
                "host": event.host,
                "service": event.service,
                "severity": event.severity,
                "fingerprint": event.fingerprint,
                "timestamp": event.timestamp,
                "detection_mode": detection_mode.as_str(),
            });

            if let Some(existing) = self
                .repo
                .find_active_instance(rule.id, &event.host, &event.service, &event.fingerprint)
                .await
                .map_err(map_db_error)?
            {
                let merged_signals =
                    merge_source_signals(&existing.source_signals, &signal_meta, 10);
                self.repo
                    .touch_alert_instance(existing.id, &payload_json, &merged_signals)
                    .await
                    .map_err(map_db_error)?;
                continue;
            }

            let title = condition
                .title
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(rule.name.as_str());
            let created = self
                .repo
                .create_alert_instance(
                    &rule,
                    title,
                    &event.host,
                    &event.service,
                    &event.fingerprint,
                    &payload_json,
                    event_ts,
                    detection_mode.as_str(),
                    &correlation_key,
                    &json!([signal_meta]),
                )
                .await
                .map_err(map_db_error)?;

            self.record_anomaly();
            self.publish_alert_stream(&created).await;
            self.publish_audit_event(AuditAppendEvent {
                event_type: "alert.instance.triggered".to_string(),
                entity_type: "alert_instance".to_string(),
                entity_id: created.id.to_string(),
                actor_id: "system".to_string(),
                actor_type: "system".to_string(),
                request_id: format!("alert-instance-{}", created.id),
                reason: "threshold matched".to_string(),
                payload_json,
                created_at: None,
            })
            .await;
        }

        Ok(())
    }

    pub async fn handle_heartbeat_signal(&self, payload: Value) -> AppResult<()> {
        self.record_signal();
        let host = payload
            .get("host")
            .and_then(Value::as_str)
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if host.is_empty() {
            self.record_rejected();
            warn!("heartbeat signal missing host");
            return Ok(());
        }
        let service = payload
            .get("service")
            .and_then(Value::as_str)
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|| "agent".to_string());
        let gap = payload
            .get("heartbeat_gap_sec")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        self.record_baseline(&host, &service, "heartbeat", gap)
            .await?;
        let threshold = self.detection.config.cooldown_sec as f64;
        if gap < threshold {
            return Ok(());
        }
        let signal_id = payload
            .get("signal_id")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let correlation_key = format!("heartbeat:{host}:{service}");
        let evidence = json!({ "heartbeat_gap_sec": gap });
        self.repo
            .insert_anomaly_score(
                None,
                "heartbeat_gap",
                "heartbeat",
                &host,
                &service,
                &correlation_key,
                self.detection.config.mode.as_str(),
                &signal_id,
                gap / threshold,
                threshold,
                &evidence,
            )
            .await
            .map_err(map_db_error)?;
        info!(
            host = %host,
            service = %service,
            gap = gap,
            "heartbeat anomaly recorded"
        );
        Ok(())
    }

    pub async fn handle_diagnostics_signal(&self, payload: Value) -> AppResult<()> {
        self.record_signal();
        let host = payload
            .get("host")
            .and_then(Value::as_str)
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if host.is_empty() {
            self.record_rejected();
            warn!("diagnostics signal missing host");
            return Ok(());
        }
        let service = payload
            .get("service")
            .and_then(Value::as_str)
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|| "agent".to_string());
        let queue_depth = payload
            .get("queue_depth")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        self.record_baseline(&host, &service, "diagnostics", queue_depth)
            .await?;
        if queue_depth < 100.0 {
            return Ok(());
        }
        let signal_id = payload
            .get("signal_id")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let correlation_key = format!("diag:{host}:{service}");
        let evidence = json!({ "queue_depth": queue_depth });
        self.repo
            .insert_anomaly_score(
                None,
                "queue_depth",
                "diagnostics",
                &host,
                &service,
                &correlation_key,
                self.detection.config.mode.as_str(),
                &signal_id,
                queue_depth / 100.0,
                100.0,
                &evidence,
            )
            .await
            .map_err(map_db_error)?;
        info!(
            host = %host,
            service = %service,
            depth = queue_depth,
            "diagnostics anomaly recorded"
        );
        Ok(())
    }

    pub async fn handle_security_signal(&self, payload: Value) -> AppResult<()> {
        self.record_signal();
        let host = payload
            .get("host")
            .and_then(Value::as_str)
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if host.is_empty() {
            self.record_rejected();
            warn!("security signal missing host");
            return Ok(());
        }
        let service = payload
            .get("service")
            .and_then(Value::as_str)
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|| "agent".to_string());
        let findings = payload
            .get("findings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if findings.is_empty() {
            return Ok(());
        }
        let severe_count = findings
            .iter()
            .filter(|finding| {
                finding
                    .get("severity")
                    .and_then(Value::as_str)
                    .map(|v| matches!(v.to_ascii_lowercase().as_str(), "high" | "critical"))
                    .unwrap_or(false)
            })
            .count();
        if severe_count == 0 {
            return Ok(());
        }
        self.record_baseline(&host, &service, "security_posture", severe_count as f64)
            .await?;
        let signal_id = payload
            .get("report_id")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let correlation_key = format!("security:{host}:{service}");
        let evidence = json!({ "findings": findings });
        self.repo
            .insert_anomaly_score(
                None,
                "security_finding",
                "security_posture",
                &host,
                &service,
                &correlation_key,
                self.detection.config.mode.as_str(),
                &signal_id,
                severe_count as f64,
                1.0,
                &evidence,
            )
            .await
            .map_err(map_db_error)?;
        info!(
            host = %host,
            service = %service,
            count = severe_count,
            "security posture anomaly recorded"
        );
        Ok(())
    }

    pub async fn resolve_stale_alerts(&self) -> AppResult<()> {
        let active = self
            .repo
            .list_active_instances_with_rules()
            .await
            .map_err(map_db_error)?;

        for (instance, rule) in active {
            let condition = parse_condition_value(&rule.condition_json)?;
            let since = Utc::now() - Duration::minutes(condition.window_minutes as i64);
            let count = self
                .clickhouse
                .matching_count(
                    &instance.host,
                    &instance.service,
                    &instance.severity,
                    &instance.fingerprint,
                    condition.query.as_deref(),
                    since,
                )
                .await
                .map_err(map_integration_error)?;
            if count >= condition.threshold {
                continue;
            }

            let payload_json = json!({
                "resolved_by": "query-alert-plane",
                "reason": "window threshold no longer met",
                "threshold": condition.threshold,
                "window_minutes": condition.window_minutes,
                "remaining_count": count,
            });
            if let Some(resolved) = self
                .repo
                .resolve_alert_instance(
                    instance.id,
                    &payload_json,
                    &merge_source_signals(
                        &instance.source_signals,
                        &json!({
                            "resolved_at": format_ts(Utc::now()),
                            "reason": "threshold no longer met",
                        }),
                        10,
                    ),
                    true,
                )
                .await
                .map_err(map_db_error)?
            {
                self.record_resolution();
                self.publish_alert_stream(&resolved).await;
                self.publish_audit_event(AuditAppendEvent {
                    event_type: "alert.instance.resolved".to_string(),
                    entity_type: "alert_instance".to_string(),
                    entity_id: resolved.id.to_string(),
                    actor_id: "system".to_string(),
                    actor_type: "system".to_string(),
                    request_id: format!("alert-resolve-{}", resolved.id),
                    reason: "threshold no longer met".to_string(),
                    payload_json,
                    created_at: None,
                })
                .await;
            }
        }

        Ok(())
    }

    async fn publish_alert_stream(&self, instance: &crate::models::AlertInstanceRecord) {
        let payload = AlertStreamEvent {
            event_type: instance.status.clone(),
            alert_instance_id: instance.id.to_string(),
            alert_rule_id: instance.rule_id.to_string(),
            title: instance.title.clone(),
            status: instance.status.clone(),
            severity: instance.severity.clone(),
            triggered_at: format_ts(instance.triggered_at),
            host: instance.host.clone(),
            service: instance.service.clone(),
        };
        match serde_json::to_vec(&payload) {
            Ok(encoded) => {
                if let Err(error) = self
                    .nats
                    .publish(UI_STREAM_ALERTS.to_string(), encoded.into())
                    .await
                {
                    warn!(error = %error, alert_instance_id = %instance.id, "failed to publish alert stream event");
                }
            }
            Err(error) => {
                warn!(error = %error, alert_instance_id = %instance.id, "failed to encode alert stream event")
            }
        }
    }

    async fn publish_audit_event(&self, event: AuditAppendEvent) {
        match serde_json::to_vec(&event) {
            Ok(encoded) => {
                if let Err(error) = self
                    .nats
                    .publish(AUDIT_EVENTS_APPEND.to_string(), encoded.into())
                    .await
                {
                    warn!(error = %error, entity_type = %event.entity_type, entity_id = %event.entity_id, "failed to publish audit append event");
                }
            }
            Err(error) => {
                warn!(error = %error, entity_type = %event.entity_type, entity_id = %event.entity_id, "failed to encode audit append event")
            }
        }
    }
}

fn paging_or_default(paging: Option<PagingRequest>, default: u32, max: u32) -> (u32, u64) {
    let paging = paging.unwrap_or_default();
    (sanitize_limit(paging.limit, default, max), paging.offset)
}

fn sanitize_limit(limit: u32, default: u32, max: u32) -> u32 {
    if limit == 0 {
        default
    } else {
        limit.min(max)
    }
}

fn metric(key: &str, label: &str, value: u64) -> query::DashboardMetric {
    query::DashboardMetric {
        key: key.to_string(),
        label: label.to_string(),
        value: value.to_string(),
        change: 0.0,
        trend: "flat".to_string(),
        description: String::new(),
    }
}

fn parse_time_window(from: &str, to: &str) -> AppResult<(DateTime<Utc>, DateTime<Utc>)> {
    let end = if let Some(value) = optional_trimmed(to) {
        DateTime::parse_from_rfc3339(value)
            .map_err(|error| AppError::invalid_argument(format!("invalid to timestamp: {error}")))?
            .with_timezone(&Utc)
    } else {
        Utc::now()
    };
    let start = if let Some(value) = optional_trimmed(from) {
        DateTime::parse_from_rfc3339(value)
            .map_err(|error| {
                AppError::invalid_argument(format!("invalid from timestamp: {error}"))
            })?
            .with_timezone(&Utc)
    } else {
        end - Duration::hours(24)
    };

    if start > end {
        return Err(AppError::invalid_argument("from must be before to"));
    }

    Ok((start, end))
}

fn parse_condition_json(payload: &str) -> AppResult<AlertRuleCondition> {
    if payload.trim().is_empty() {
        return Ok(AlertRuleCondition::default());
    }
    let value: Value = serde_json::from_str(payload).map_err(|error| {
        AppError::invalid_argument(format!("condition_json must be valid JSON: {error}"))
    })?;
    parse_condition_value(&value)
}

fn parse_condition_value(value: &Value) -> AppResult<AlertRuleCondition> {
    if !value.is_object() {
        return Err(AppError::invalid_argument(
            "condition_json must be a JSON object",
        ));
    }
    serde_json::from_value::<AlertRuleCondition>(value.clone())
        .map_err(|error| AppError::invalid_argument(format!("invalid alert condition: {error}")))
}

fn validate_alert_rule_input(
    name: &str,
    severity: &str,
    scope_type: &str,
    scope_id: Option<&str>,
    condition: &AlertRuleCondition,
) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_argument("name is required"));
    }
    let severity = normalize_severity(severity);
    if !matches!(
        severity.as_str(),
        "info" | "low" | "medium" | "high" | "critical"
    ) {
        return Err(AppError::invalid_argument("unsupported alert severity"));
    }
    let scope_type = normalize_scope_type(scope_type);
    match scope_type.as_str() {
        "global" => {}
        "host" | "service" => {
            if scope_id.is_none() {
                return Err(AppError::invalid_argument(
                    "scope_id is required for scoped alert rules",
                ));
            }
        }
        _ => return Err(AppError::invalid_argument("unsupported scope_type")),
    }
    if condition.threshold == 0 {
        return Err(AppError::invalid_argument(
            "condition threshold must be greater than 0",
        ));
    }
    if condition.window_minutes == 0 || condition.window_minutes > 24 * 60 {
        return Err(AppError::invalid_argument(
            "condition window_minutes must be between 1 and 1440",
        ));
    }
    Ok(())
}

fn normalize_rule_status(value: &str) -> AppResult<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "active" => Ok("active".to_string()),
        "paused" => Ok("paused".to_string()),
        _ => Err(AppError::invalid_argument("unsupported alert rule status")),
    }
}

fn normalize_scope_type(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "global" => "global".to_string(),
        "host" => "host".to_string(),
        "service" => "service".to_string(),
        other => other.to_string(),
    }
}

fn normalize_severity(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "info" => "info".to_string(),
        "low" => "low".to_string(),
        "medium" => "medium".to_string(),
        "high" => "high".to_string(),
        "critical" => "critical".to_string(),
        other => other.to_string(),
    }
}

fn event_matches_rule(
    rule: &AlertRuleRecord,
    condition: &AlertRuleCondition,
    event: &NormalizedLogEvent,
) -> bool {
    match rule.scope_type.as_str() {
        "global" => {}
        "host" => {
            if rule.scope_id.as_deref() != Some(event.host.as_str()) {
                return false;
            }
        }
        "service" => {
            if rule.scope_id.as_deref() != Some(event.service.as_str()) {
                return false;
            }
        }
        _ => return false,
    }

    if let Some(host) = condition.host.as_deref() {
        if !host.eq_ignore_ascii_case(&event.host) {
            return false;
        }
    }
    if let Some(service) = condition.service.as_deref() {
        if !service.eq_ignore_ascii_case(&event.service) {
            return false;
        }
    }
    if let Some(severity) = condition.severity.as_deref() {
        if !severity.eq_ignore_ascii_case(&event.severity) {
            return false;
        }
    }
    if let Some(fingerprint) = condition.fingerprint.as_deref() {
        if fingerprint != event.fingerprint {
            return false;
        }
    }
    if let Some(query) = condition.query.as_deref() {
        if !event
            .message
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
        {
            return false;
        }
    }

    true
}

fn parse_event_timestamp(value: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| AppError::invalid_argument(format!("invalid event timestamp: {error}")))
}

fn optional_trimmed(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_uuid(field: &str, value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value.trim())
        .map_err(|error| AppError::invalid_argument(format!("invalid {field}: {error}")))
}

fn non_empty_or(value: &str, default: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn merge_source_signals(existing: &Value, new_signal: &Value, max_items: usize) -> Value {
    let mut items = existing
        .as_array()
        .cloned()
        .unwrap_or_else(|| Vec::with_capacity(1));
    items.push(new_signal.clone());
    if items.len() > max_items {
        let drain = items.len().saturating_sub(max_items);
        items.drain(0..drain);
    }
    Value::Array(items)
}

fn map_db_error(error: sqlx::Error) -> AppError {
    match &error {
        sqlx::Error::Database(db_error) => {
            if let Some(code) = db_error.code() {
                if code.as_ref() == "23505" {
                    return AppError::invalid_argument(format!(
                        "constraint violation: {}",
                        db_error.message()
                    ));
                }
            }
            AppError::internal(format!("database error: {db_error}"))
        }
        _ => AppError::internal(format!("database error: {error}")),
    }
}

fn map_integration_error(error: anyhow::Error) -> AppError {
    AppError::internal(format!("runtime integration error: {error:#}"))
}
