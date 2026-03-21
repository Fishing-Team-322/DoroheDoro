use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use crate::models::{
    AlertInstanceRecord, AlertRuleRecord, AnomalyInstanceRecord, AnomalyRuleRecord,
    AuditActivityRecord,
};

#[derive(Clone)]
pub struct QueryAlertRepository {
    pool: PgPool,
}

impl QueryAlertRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
    }

    pub async fn list_alert_rules(
        &self,
        query: Option<&str>,
        status: Option<&str>,
        limit: u32,
        offset: u64,
    ) -> Result<(Vec<AlertRuleRecord>, u64), sqlx::Error> {
        let rows = sqlx::query_as::<_, AlertRuleRecord>(
            "SELECT id, name, description, status, severity, scope_type, scope_id,
                    condition_json, created_by, updated_by, created_at, updated_at
             FROM alert_rules
             WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%' OR description ILIKE '%' || $1 || '%')
               AND ($2::text IS NULL OR status = $2)
             ORDER BY updated_at DESC, name ASC
             LIMIT $3 OFFSET $4",
        )
        .bind(query)
        .bind(status)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM alert_rules
             WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%' OR description ILIKE '%' || $1 || '%')
               AND ($2::text IS NULL OR status = $2)",
        )
        .bind(query)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok((rows, total.max(0) as u64))
    }

    pub async fn get_alert_rule(
        &self,
        alert_rule_id: Uuid,
    ) -> Result<Option<AlertRuleRecord>, sqlx::Error> {
        sqlx::query_as::<_, AlertRuleRecord>(
            "SELECT id, name, description, status, severity, scope_type, scope_id,
                    condition_json, created_by, updated_by, created_at, updated_at
             FROM alert_rules
             WHERE id = $1
             LIMIT 1",
        )
        .bind(alert_rule_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_alert_rule(
        &self,
        name: &str,
        description: &str,
        severity: &str,
        scope_type: &str,
        scope_id: Option<&str>,
        condition_json: &Value,
        actor_id: &str,
    ) -> Result<AlertRuleRecord, sqlx::Error> {
        sqlx::query_as::<_, AlertRuleRecord>(
            "INSERT INTO alert_rules (
                id, name, description, status, severity, scope_type, scope_id,
                condition_json, created_by, updated_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'active', $4, $5, $6, $7, $8, $8, NOW(), NOW())
            RETURNING id, name, description, status, severity, scope_type, scope_id,
                      condition_json, created_by, updated_by, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(description)
        .bind(severity)
        .bind(scope_type)
        .bind(scope_id)
        .bind(condition_json)
        .bind(actor_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_alert_rule(
        &self,
        alert_rule_id: Uuid,
        name: &str,
        description: &str,
        status: &str,
        severity: &str,
        scope_type: &str,
        scope_id: Option<&str>,
        condition_json: &Value,
        actor_id: &str,
    ) -> Result<Option<AlertRuleRecord>, sqlx::Error> {
        sqlx::query_as::<_, AlertRuleRecord>(
            "UPDATE alert_rules
             SET name = $2,
                 description = $3,
                 status = $4,
                 severity = $5,
                 scope_type = $6,
                 scope_id = $7,
                 condition_json = $8,
                 updated_by = $9,
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, name, description, status, severity, scope_type, scope_id,
                       condition_json, created_by, updated_by, created_at, updated_at",
        )
        .bind(alert_rule_id)
        .bind(name)
        .bind(description)
        .bind(status)
        .bind(severity)
        .bind(scope_type)
        .bind(scope_id)
        .bind(condition_json)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_active_rules(&self) -> Result<Vec<AlertRuleRecord>, sqlx::Error> {
        sqlx::query_as::<_, AlertRuleRecord>(
            "SELECT id, name, description, status, severity, scope_type, scope_id,
                    condition_json, created_by, updated_by, created_at, updated_at
             FROM alert_rules
             WHERE status = 'active'
             ORDER BY updated_at DESC, name ASC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_active_anomaly_rules(&self) -> Result<Vec<AnomalyRuleRecord>, sqlx::Error> {
        sqlx::query_as::<_, AnomalyRuleRecord>(
            "SELECT id, name, kind, scope_type, scope_id, config_json, is_active,
                    created_at, updated_at, created_by, updated_by
             FROM anomaly_rules
             WHERE is_active = TRUE
             ORDER BY updated_at DESC, name ASC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn upsert_system_rule(
        &self,
        name: &str,
        description: &str,
        severity: &str,
        status: &str,
        condition_json: &Value,
    ) -> Result<AlertRuleRecord, sqlx::Error> {
        if let Some(existing) = sqlx::query_as::<_, AlertRuleRecord>(
            "SELECT id, name, description, status, severity, scope_type, scope_id,
                    condition_json, created_by, updated_by, created_at, updated_at
             FROM alert_rules
             WHERE name = $1 AND created_by = 'system'
             LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        {
            if existing.severity == severity
                && existing.status == status
                && existing.description == description
                && existing.condition_json == *condition_json
            {
                return Ok(existing);
            }

            return sqlx::query_as::<_, AlertRuleRecord>(
                "UPDATE alert_rules
                 SET description = $2,
                     status = $3,
                     severity = $4,
                     condition_json = $5,
                     scope_type = 'global',
                     scope_id = NULL,
                     updated_by = 'system',
                     updated_at = NOW()
                 WHERE id = $1
                 RETURNING id, name, description, status, severity, scope_type, scope_id,
                           condition_json, created_by, updated_by, created_at, updated_at",
            )
            .bind(existing.id)
            .bind(description)
            .bind(status)
            .bind(severity)
            .bind(condition_json)
            .fetch_one(&self.pool)
            .await;
        }

        sqlx::query_as::<_, AlertRuleRecord>(
            "INSERT INTO alert_rules (
                id, name, description, status, severity, scope_type, scope_id,
                condition_json, created_by, updated_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 'global', NULL, $6, 'system', 'system', NOW(), NOW())
            RETURNING id, name, description, status, severity, scope_type, scope_id,
                      condition_json, created_by, updated_by, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(description)
        .bind(status)
        .bind(severity)
        .bind(condition_json)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_alert_instances(
        &self,
        status: Option<&str>,
        severity: Option<&str>,
        host: Option<&str>,
        service: Option<&str>,
        limit: u32,
        offset: u64,
    ) -> Result<(Vec<AlertInstanceRecord>, u64), sqlx::Error> {
        let rows = sqlx::query_as::<_, AlertInstanceRecord>(
            "SELECT ai.id, ai.rule_id, ai.title, ai.status, ai.severity, ai.host, ai.service,
                    ai.fingerprint, ai.payload_json, ai.detection_mode, ai.correlation_key,
                    ai.source_signals, ai.triggered_at, ai.acknowledged_at, ai.resolved_at,
                    ai.auto_resolved_at, ai.updated_at, ar.name AS rule_name
             FROM alert_instances ai
             JOIN alert_rules ar ON ar.id = ai.rule_id
             WHERE ($1::text IS NULL OR ai.status = $1)
               AND ($2::text IS NULL OR ai.severity = $2)
               AND ($3::text IS NULL OR ai.host = $3)
               AND ($4::text IS NULL OR ai.service = $4)
             ORDER BY ai.triggered_at DESC, ai.id DESC
             LIMIT $5 OFFSET $6",
        )
        .bind(status)
        .bind(severity)
        .bind(host)
        .bind(service)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM alert_instances
             WHERE ($1::text IS NULL OR status = $1)
               AND ($2::text IS NULL OR severity = $2)
               AND ($3::text IS NULL OR host = $3)
               AND ($4::text IS NULL OR service = $4)",
        )
        .bind(status)
        .bind(severity)
        .bind(host)
        .bind(service)
        .fetch_one(&self.pool)
        .await?;

        Ok((rows, total.max(0) as u64))
    }

    pub async fn get_alert_instance(
        &self,
        alert_instance_id: Uuid,
    ) -> Result<Option<AlertInstanceRecord>, sqlx::Error> {
        sqlx::query_as::<_, AlertInstanceRecord>(
            "SELECT ai.id, ai.rule_id, ai.title, ai.status, ai.severity, ai.host, ai.service,
                    ai.fingerprint, ai.payload_json, ai.detection_mode, ai.correlation_key,
                    ai.source_signals, ai.triggered_at, ai.acknowledged_at,
                    ai.resolved_at, ai.auto_resolved_at, ai.updated_at, ar.name AS rule_name
             FROM alert_instances ai
             JOIN alert_rules ar ON ar.id = ai.rule_id
             WHERE ai.id = $1
             LIMIT 1",
        )
        .bind(alert_instance_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_log_anomaly_projections(
        &self,
        host: Option<&str>,
        service: Option<&str>,
        severity: Option<&str>,
        limit: u32,
        offset: u64,
    ) -> Result<(Vec<AlertInstanceRecord>, u64), sqlx::Error> {
        self.list_alert_instances(None, severity, host, service, limit, offset)
            .await
    }

    pub async fn find_active_instance(
        &self,
        rule_id: Uuid,
        host: &str,
        service: &str,
        fingerprint: &str,
    ) -> Result<Option<AlertInstanceRecord>, sqlx::Error> {
        sqlx::query_as::<_, AlertInstanceRecord>(
            "SELECT ai.id, ai.rule_id, ai.title, ai.status, ai.severity, ai.host, ai.service,
                    ai.fingerprint, ai.payload_json, ai.detection_mode, ai.correlation_key,
                    ai.source_signals, ai.triggered_at, ai.acknowledged_at,
                    ai.resolved_at, ai.auto_resolved_at, ai.updated_at, ar.name AS rule_name
             FROM alert_instances ai
             JOIN alert_rules ar ON ar.id = ai.rule_id
             WHERE ai.rule_id = $1
               AND ai.host = $2
               AND ai.service = $3
               AND ai.fingerprint = $4
               AND ai.status IN ('active', 'acknowledged')
             LIMIT 1",
        )
        .bind(rule_id)
        .bind(host)
        .bind(service)
        .bind(fingerprint)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_alert_instance(
        &self,
        rule: &AlertRuleRecord,
        title: &str,
        host: &str,
        service: &str,
        fingerprint: &str,
        payload_json: &Value,
        triggered_at: DateTime<Utc>,
        detection_mode: &str,
        correlation_key: &str,
        source_signals: &Value,
    ) -> Result<AlertInstanceRecord, sqlx::Error> {
        sqlx::query_as::<_, AlertInstanceRecord>(
            "INSERT INTO alert_instances (
                id, rule_id, title, status, severity, host, service, fingerprint,
                payload_json, detection_mode, correlation_key, source_signals,
                triggered_at, acknowledged_at, resolved_at, auto_resolved_at, updated_at
            )
            VALUES ($1, $2, $3, 'active', $4, $5, $6, $7, $8, $9, $10, $11, $12, NULL, NULL, NULL, NOW())
            RETURNING id, rule_id, title, status, severity, host, service, fingerprint,
                      payload_json, detection_mode, correlation_key, source_signals,
                      triggered_at, acknowledged_at, resolved_at, auto_resolved_at, updated_at,
                      NULL::text AS rule_name",
        )
        .bind(Uuid::new_v4())
        .bind(rule.id)
        .bind(title)
        .bind(&rule.severity)
        .bind(host)
        .bind(service)
        .bind(fingerprint)
        .bind(payload_json)
        .bind(detection_mode)
        .bind(correlation_key)
        .bind(source_signals)
        .bind(triggered_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn touch_alert_instance(
        &self,
        alert_instance_id: Uuid,
        payload_json: &Value,
        source_signals: &Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE alert_instances
             SET payload_json = $2,
                 source_signals = $3,
                 updated_at = NOW()
             WHERE id = $1",
        )
        .bind(alert_instance_id)
        .bind(payload_json)
        .bind(source_signals)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_active_instances_with_rules(
        &self,
    ) -> Result<Vec<(AlertInstanceRecord, AlertRuleRecord)>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT
                ai.id AS ai_id,
                ai.rule_id AS ai_rule_id,
                ai.title AS ai_title,
                ai.status AS ai_status,
                ai.severity AS ai_severity,
                ai.host AS ai_host,
                ai.service AS ai_service,
                ai.fingerprint AS ai_fingerprint,
                ai.payload_json AS ai_payload_json,
                ai.detection_mode AS ai_detection_mode,
                ai.correlation_key AS ai_correlation_key,
                ai.source_signals AS ai_source_signals,
                ai.triggered_at AS ai_triggered_at,
                ai.acknowledged_at AS ai_acknowledged_at,
                ai.resolved_at AS ai_resolved_at,
                ai.auto_resolved_at AS ai_auto_resolved_at,
                ai.updated_at AS ai_updated_at,
                ar.id AS ar_id,
                ar.name AS ar_name,
                ar.description AS ar_description,
                ar.status AS ar_status,
                ar.severity AS ar_severity,
                ar.scope_type AS ar_scope_type,
                ar.scope_id AS ar_scope_id,
                ar.condition_json AS ar_condition_json,
                ar.created_by AS ar_created_by,
                ar.updated_by AS ar_updated_by,
                ar.created_at AS ar_created_at,
                ar.updated_at AS ar_updated_at
             FROM alert_instances ai
             JOIN alert_rules ar ON ar.id = ai.rule_id
             WHERE ai.status IN ('active', 'acknowledged')
               AND ar.status = 'active'
             ORDER BY ai.triggered_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(active_pair_from_row).collect())
    }

    pub async fn resolve_alert_instance(
        &self,
        alert_instance_id: Uuid,
        payload_json: &Value,
        source_signals: &Value,
        auto_resolved: bool,
    ) -> Result<Option<AlertInstanceRecord>, sqlx::Error> {
        sqlx::query_as::<_, AlertInstanceRecord>(
            "UPDATE alert_instances
             SET status = 'resolved',
                 resolved_at = NOW(),
                 auto_resolved_at = CASE WHEN $4 THEN NOW() ELSE auto_resolved_at END,
                 updated_at = NOW(),
                 payload_json = $2,
                 source_signals = $3
             WHERE id = $1
               AND status IN ('active', 'acknowledged')
             RETURNING id, rule_id, title, status, severity, host, service, fingerprint,
                       payload_json, detection_mode, correlation_key, source_signals,
                       triggered_at, acknowledged_at, resolved_at, auto_resolved_at, updated_at,
                       NULL::text AS rule_name",
        )
        .bind(alert_instance_id)
        .bind(payload_json)
        .bind(source_signals)
        .bind(auto_resolved)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_open_anomaly_instance(
        &self,
        rule_id: Uuid,
        dedupe_key: &str,
    ) -> Result<Option<AnomalyInstanceRecord>, sqlx::Error> {
        sqlx::query_as::<_, AnomalyInstanceRecord>(
            "SELECT id, rule_id, cluster_id, severity, status, started_at, resolved_at, payload_json
             FROM anomaly_instances
             WHERE rule_id = $1
               AND status = 'open'
               AND payload_json ->> 'dedupe_key' = $2
             ORDER BY started_at DESC
             LIMIT 1",
        )
        .bind(rule_id)
        .bind(dedupe_key)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_anomaly_instance(
        &self,
        rule_id: Uuid,
        cluster_id: Option<Uuid>,
        severity: &str,
        payload_json: &Value,
    ) -> Result<AnomalyInstanceRecord, sqlx::Error> {
        sqlx::query_as::<_, AnomalyInstanceRecord>(
            "INSERT INTO anomaly_instances (
                id, rule_id, cluster_id, severity, status, started_at, payload_json
            )
            VALUES ($1, $2, $3, $4, 'open', NOW(), $5)
            RETURNING id, rule_id, cluster_id, severity, status, started_at, resolved_at, payload_json",
        )
        .bind(Uuid::new_v4())
        .bind(rule_id)
        .bind(cluster_id)
        .bind(severity)
        .bind(payload_json)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn touch_anomaly_instance(
        &self,
        anomaly_instance_id: Uuid,
        payload_json: &Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE anomaly_instances
             SET payload_json = payload_json || $2::jsonb
             WHERE id = $1",
        )
        .bind(anomaly_instance_id)
        .bind(payload_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn resolve_anomaly_instance_by_key(
        &self,
        rule_id: Uuid,
        dedupe_key: &str,
        payload_json: &Value,
    ) -> Result<Option<AnomalyInstanceRecord>, sqlx::Error> {
        sqlx::query_as::<_, AnomalyInstanceRecord>(
            "UPDATE anomaly_instances
             SET status = 'resolved',
                 resolved_at = NOW(),
                 payload_json = payload_json || $3::jsonb
             WHERE rule_id = $1
               AND status = 'open'
               AND payload_json ->> 'dedupe_key' = $2
             RETURNING id, rule_id, cluster_id, severity, status, started_at, resolved_at, payload_json",
        )
        .bind(rule_id)
        .bind(dedupe_key)
        .bind(payload_json)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn resolve_anomaly_instance(
        &self,
        anomaly_instance_id: Uuid,
        payload_json: &Value,
    ) -> Result<Option<AnomalyInstanceRecord>, sqlx::Error> {
        sqlx::query_as::<_, AnomalyInstanceRecord>(
            "UPDATE anomaly_instances
             SET status = 'resolved',
                 resolved_at = NOW(),
                 payload_json = payload_json || $2::jsonb
             WHERE id = $1
               AND status = 'open'
             RETURNING id, rule_id, cluster_id, severity, status, started_at, resolved_at, payload_json",
        )
        .bind(anomaly_instance_id)
        .bind(payload_json)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn count_open_alerts(&self) -> Result<u64, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM alert_instances WHERE status IN ('active', 'acknowledged')",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(value.max(0) as u64)
    }

    pub async fn count_active_hosts_since(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, sqlx::Error> {
        let value =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agents WHERE last_seen_at >= $1")
                .bind(cutoff)
                .fetch_one(&self.pool)
                .await?;
        Ok(value.max(0) as u64)
    }

    pub async fn count_deployment_jobs_since(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, sqlx::Error> {
        let value = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM deployment_jobs WHERE created_at >= $1",
        )
        .bind(cutoff)
        .fetch_one(&self.pool)
        .await?;
        Ok(value.max(0) as u64)
    }

    pub async fn recent_activity(
        &self,
        limit: u32,
    ) -> Result<Vec<AuditActivityRecord>, sqlx::Error> {
        sqlx::query_as::<_, AuditActivityRecord>(
            "SELECT event_type, entity_type, entity_id, reason, created_at
             FROM runtime_audit_events
             ORDER BY created_at DESC
             LIMIT $1",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
    }
}

fn active_pair_from_row(row: PgRow) -> (AlertInstanceRecord, AlertRuleRecord) {
    (
        AlertInstanceRecord {
            id: row.get("ai_id"),
            rule_id: row.get("ai_rule_id"),
            title: row.get("ai_title"),
            status: row.get("ai_status"),
            severity: row.get("ai_severity"),
            host: row.get("ai_host"),
            service: row.get("ai_service"),
            fingerprint: row.get("ai_fingerprint"),
            payload_json: row.get("ai_payload_json"),
            detection_mode: row.get("ai_detection_mode"),
            correlation_key: row.get("ai_correlation_key"),
            source_signals: row.get("ai_source_signals"),
            triggered_at: row.get("ai_triggered_at"),
            acknowledged_at: row.get("ai_acknowledged_at"),
            resolved_at: row.get("ai_resolved_at"),
            auto_resolved_at: row.get("ai_auto_resolved_at"),
            updated_at: row.get("ai_updated_at"),
            rule_name: Some(row.get("ar_name")),
        },
        AlertRuleRecord {
            id: row.get("ar_id"),
            name: row.get("ar_name"),
            description: row.get("ar_description"),
            status: row.get("ar_status"),
            severity: row.get("ar_severity"),
            scope_type: row.get("ar_scope_type"),
            scope_id: row.get("ar_scope_id"),
            condition_json: row.get("ar_condition_json"),
            created_by: row.get("ar_created_by"),
            updated_by: row.get("ar_updated_by"),
            created_at: row.get("ar_created_at"),
            updated_at: row.get("ar_updated_at"),
        },
    )
}
