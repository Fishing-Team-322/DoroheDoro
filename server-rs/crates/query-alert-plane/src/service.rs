use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration as StdDuration, Instant},
};

use anyhow::{anyhow, Context};
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
use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;

use crate::{
    anomaly::{
        parse_anomaly_rule, BaselineRule, ParsedAnomalyRule, RareFingerprintRule, ThresholdRule,
    },
    config::{AnomalyEngineConfig, RareFingerprintConfig},
    models::{format_ts, AlertRuleCondition, AlertRuleRecord},
    repository::QueryAlertRepository,
    storage::{ClickHouseClient, OpenSearchClient},
};

const SYSTEM_RARE_RULE_NAME: &str = "[system] rare fingerprint detector";
const SYSTEM_RARE_RULE_DESCRIPTION: &str =
    "Automatically flags log fingerprints that rarely appear within the configured window";

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

#[derive(Clone)]
pub struct QueryAlertService {
    repo: QueryAlertRepository,
    nats: Client,
    opensearch: OpenSearchClient,
    clickhouse: ClickHouseClient,
    ready: Arc<AtomicBool>,
    rare_config: RareFingerprintConfig,
    anomaly_cache: Arc<RwLock<AnomalyRuleCache>>,
    anomaly_config: AnomalyEngineConfig,
}

impl QueryAlertService {
    pub fn new(
        repo: QueryAlertRepository,
        nats: Client,
        opensearch: OpenSearchClient,
        clickhouse: ClickHouseClient,
        rare_config: RareFingerprintConfig,
        anomaly_config: AnomalyEngineConfig,
    ) -> Self {
        Self {
            repo,
            nats,
            opensearch,
            clickhouse,
            ready: Arc::new(AtomicBool::new(false)),
            rare_config,
            anomaly_cache: Arc::new(RwLock::new(AnomalyRuleCache::new())),
            anomaly_config,
        }
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        self.repo.ping().await.context("ping postgres")?;
        self.opensearch.ping().await.context("ping opensearch")?;
        self.clickhouse.ping().await.context("ping clickhouse")?;
        self.bootstrap_system_rules().await?;
        self.ready.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn bootstrap_system_rules(&self) -> anyhow::Result<()> {
        let condition = AlertRuleCondition {
            mode: Some("rare_fingerprint".to_string()),
            threshold: self.rare_config.max_count,
            window_minutes: self.rare_config.window_minutes,
            ..AlertRuleCondition::default()
        };
        let condition_json = serde_json::to_value(&condition)
            .map_err(|error| anyhow!("serialize rare fingerprint rule: {error}"))?;
        let status = if self.rare_config.enabled {
            "active"
        } else {
            "paused"
        };
        self.repo
            .upsert_system_rule(
                SYSTEM_RARE_RULE_NAME,
                SYSTEM_RARE_RULE_DESCRIPTION,
                &self.rare_config.severity,
                status,
                &condition_json,
            )
            .await?;
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
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
        let rules = self.repo.list_active_rules().await.map_err(map_db_error)?;
        for rule in rules {
            let condition = parse_condition_value(&rule.condition_json)?;
            if !event_matches_rule(&rule, &condition, &event) {
                continue;
            }

            let since = parse_event_timestamp(&event.timestamp)?
                - Duration::minutes(condition.window_minutes as i64);
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
            if !meets_condition(condition.mode.as_deref(), hit_count, condition.threshold) {
                continue;
            }

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
            });

            if let Some(existing) = self
                .repo
                .find_active_instance(rule.id, &event.host, &event.service, &event.fingerprint)
                .await
                .map_err(map_db_error)?
            {
                self.repo
                    .touch_alert_instance(existing.id, &payload_json)
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
                    parse_event_timestamp(&event.timestamp)?,
                )
                .await
                .map_err(map_db_error)?;

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

        self.evaluate_rare_anomalies(&event).await
    }

    async fn evaluate_rare_anomalies(&self, event: &NormalizedLogEvent) -> AppResult<()> {
        let rules = self.anomaly_rules().await?;
        for parsed in rules.iter() {
            if let ParsedAnomalyRule::RareFingerprint(rule) = parsed {
                self.process_rare_rule(rule, event).await?;
            }
        }
        Ok(())
    }

    async fn process_rare_rule(
        &self,
        rule: &RareFingerprintRule,
        event: &NormalizedLogEvent,
    ) -> AppResult<()> {
        if !rule.matches(event) {
            return Ok(());
        }
        let event_ts = parse_event_timestamp(&event.timestamp)?;
        let since = event_ts - Duration::minutes(rule.window_minutes as i64);
        let filter = rule
            .resolved_filter(event)
            .with_fingerprint(event.fingerprint.clone());
        let hit_count = self
            .clickhouse
            .count_events(&filter, since, event_ts)
            .await
            .map_err(map_integration_error)?;
        if hit_count > rule.max_count {
            return Ok(());
        }
        let dedupe_key = rule.dedupe_key(event);
        let payload = json!({
            "dedupe_key": dedupe_key,
            "kind": "rare_fingerprint",
            "host": event.host,
            "service": event.service,
            "fingerprint": event.fingerprint,
            "message": event.message,
            "window_minutes": rule.window_minutes,
            "max_count": rule.max_count,
            "observed_count": hit_count,
            "event_timestamp": event.timestamp,
            "last_seen_at": format_ts(event_ts),
        });
        self.ensure_anomaly_open(
            rule.record.id,
            rule.record.cluster_id(),
            &rule.severity,
            &dedupe_key,
            payload,
            "rare fingerprint detected",
        )
        .await
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
            if meets_condition(condition.mode.as_deref(), count, condition.threshold) {
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
                .resolve_alert_instance(instance.id, &payload_json)
                .await
                .map_err(map_db_error)?
            {
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

    pub async fn evaluate_anomaly_rules(&self) -> AppResult<()> {
        let now = Utc::now();
        let rules = self.anomaly_rules().await?;
        for parsed in rules.iter() {
            match parsed {
                ParsedAnomalyRule::Threshold(rule) => {
                    self.evaluate_threshold_rule(rule, now).await?
                }
                ParsedAnomalyRule::Baseline(rule) => self.evaluate_baseline_rule(rule, now).await?,
                ParsedAnomalyRule::RareFingerprint(_) => {}
            }
        }
        Ok(())
    }

    pub fn anomaly_interval(&self) -> StdDuration {
        StdDuration::from_secs(self.anomaly_config.evaluation_interval_secs.max(10))
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

    async fn evaluate_threshold_rule(
        &self,
        rule: &ThresholdRule,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        let window_start = now - Duration::minutes(rule.window_minutes as i64);
        let filter = rule.filter.as_resolved();
        let count = self
            .clickhouse
            .count_events(&filter, window_start, now)
            .await
            .map_err(map_integration_error)?;
        let dedupe_key = rule.dedupe_key();
        if count >= rule.threshold {
            let payload = json!({
                "dedupe_key": dedupe_key,
                "kind": "threshold",
                "window_minutes": rule.window_minutes,
                "threshold": rule.threshold,
                "observed_count": count,
                "evaluated_at": format_ts(now),
            });
            self.ensure_anomaly_open(
                rule.record.id,
                rule.record.cluster_id(),
                &rule.severity,
                &dedupe_key,
                payload,
                "threshold anomaly detected",
            )
            .await
        } else {
            let payload = json!({
                "dedupe_key": dedupe_key,
                "kind": "threshold",
                "window_minutes": rule.window_minutes,
                "threshold": rule.threshold,
                "observed_count": count,
                "evaluated_at": format_ts(now),
            });
            self.resolve_anomaly_by_key(
                rule.record.id,
                &dedupe_key,
                payload,
                "threshold normalized",
            )
            .await
        }
    }

    async fn evaluate_baseline_rule(
        &self,
        rule: &BaselineRule,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        let window_start = now - Duration::minutes(rule.window_minutes as i64);
        let baseline_end = window_start;
        let baseline_start = baseline_end - Duration::minutes(rule.baseline_minutes as i64);
        let filter = rule.filter.as_resolved();
        let current = self
            .clickhouse
            .count_events(&filter, window_start, now)
            .await
            .map_err(map_integration_error)?;
        let baseline_total = self
            .clickhouse
            .count_events(&filter, baseline_start, baseline_end)
            .await
            .map_err(map_integration_error)?;
        let windows = rule.baseline_windows() as f64;
        let baseline_avg = if windows > 0.0 {
            baseline_total as f64 / windows
        } else {
            0.0
        };
        let dedupe_key = rule.dedupe_key();
        let significant = baseline_total >= rule.min_baseline_total
            && current >= rule.min_count
            && baseline_avg > 0.0;
        let exceeds = (current as f64) >= baseline_avg * rule.multiplier;
        if significant && exceeds {
            let payload = json!({
                "dedupe_key": dedupe_key,
                "kind": "baseline",
                "window_minutes": rule.window_minutes,
                "baseline_minutes": rule.baseline_minutes,
                "multiplier": rule.multiplier,
                "observed_count": current,
                "baseline_total": baseline_total,
                "baseline_average": baseline_avg,
                "evaluated_at": format_ts(now),
            });
            self.ensure_anomaly_open(
                rule.record.id,
                rule.record.cluster_id(),
                &rule.severity,
                &dedupe_key,
                payload,
                "baseline deviation detected",
            )
            .await
        } else {
            let payload = json!({
                "dedupe_key": dedupe_key,
                "kind": "baseline",
                "window_minutes": rule.window_minutes,
                "baseline_minutes": rule.baseline_minutes,
                "multiplier": rule.multiplier,
                "observed_count": current,
                "baseline_total": baseline_total,
                "baseline_average": baseline_avg,
                "evaluated_at": format_ts(now),
            });
            self.resolve_anomaly_by_key(
                rule.record.id,
                &dedupe_key,
                payload,
                "baseline deviation resolved",
            )
            .await
        }
    }

    async fn anomaly_rules(&self) -> AppResult<Vec<ParsedAnomalyRule>> {
        let ttl = StdDuration::from_secs(self.anomaly_config.rule_cache_ttl_secs.max(5));
        {
            let cache = self.anomaly_cache.read().await;
            if !cache.is_stale(ttl) {
                return Ok(cache.items.clone());
            }
        }

        let records = self
            .repo
            .list_active_anomaly_rules()
            .await
            .map_err(map_db_error)?;
        let mut parsed = Vec::new();
        for record in records {
            match parse_anomaly_rule(record) {
                Ok(rule) => parsed.push(rule),
                Err(error) => {
                    warn!(error = %error, "skipping invalid anomaly rule configuration");
                }
            }
        }

        let mut cache = self.anomaly_cache.write().await;
        cache.items = parsed.clone();
        cache.loaded_at = Some(Instant::now());
        Ok(parsed)
    }

    async fn ensure_anomaly_open(
        &self,
        rule_id: Uuid,
        cluster_id: Option<Uuid>,
        severity: &str,
        dedupe_key: &str,
        payload: Value,
        reason: &str,
    ) -> AppResult<()> {
        if let Some(existing) = self
            .repo
            .find_open_anomaly_instance(rule_id, dedupe_key)
            .await
            .map_err(map_db_error)?
        {
            self.repo
                .touch_anomaly_instance(existing.id, &payload)
                .await
                .map_err(map_db_error)?;
            return Ok(());
        }

        let created = self
            .repo
            .create_anomaly_instance(rule_id, cluster_id, severity, &payload)
            .await
            .map_err(map_db_error)?;
        self.publish_audit_event(AuditAppendEvent {
            event_type: "anomaly.instance.opened".to_string(),
            entity_type: "anomaly_instance".to_string(),
            entity_id: created.id.to_string(),
            actor_id: "system".to_string(),
            actor_type: "system".to_string(),
            request_id: format!("anomaly-open-{}", created.id),
            reason: reason.to_string(),
            payload_json: payload,
            created_at: None,
        })
        .await;
        Ok(())
    }

    async fn resolve_anomaly_by_key(
        &self,
        rule_id: Uuid,
        dedupe_key: &str,
        payload: Value,
        reason: &str,
    ) -> AppResult<()> {
        if let Some(resolved) = self
            .repo
            .resolve_anomaly_instance_by_key(rule_id, dedupe_key, &payload)
            .await
            .map_err(map_db_error)?
        {
            self.publish_audit_event(AuditAppendEvent {
                event_type: "anomaly.instance.resolved".to_string(),
                entity_type: "anomaly_instance".to_string(),
                entity_id: resolved.id.to_string(),
                actor_id: "system".to_string(),
                actor_type: "system".to_string(),
                request_id: format!("anomaly-resolve-{}", resolved.id),
                reason: reason.to_string(),
                payload_json: payload,
                created_at: None,
            })
            .await;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct AnomalyRuleCache {
    items: Vec<ParsedAnomalyRule>,
    loaded_at: Option<Instant>,
}

impl AnomalyRuleCache {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            loaded_at: None,
        }
    }

    fn is_stale(&self, ttl: StdDuration) -> bool {
        match self.loaded_at {
            Some(instant) => instant.elapsed() >= ttl,
            None => true,
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
    if condition.mode.is_some() {
        return Err(AppError::invalid_argument(
            "condition_json.mode is reserved",
        ));
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

fn meets_condition(mode: Option<&str>, hit_count: u64, threshold: u64) -> bool {
    match mode.unwrap_or("threshold") {
        "rare_fingerprint" => hit_count <= threshold,
        _ => hit_count >= threshold,
    }
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
