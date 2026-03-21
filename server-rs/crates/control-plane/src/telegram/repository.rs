use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Clone)]
pub struct TelegramRepository {
    pool: PgPool,
}

impl TelegramRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_matching_routes(
        &self,
        cluster_id: Option<Uuid>,
    ) -> Result<Vec<TelegramDeliveryRoute>, sqlx::Error> {
        sqlx::query_as::<_, TelegramDeliveryRoute>(
            "SELECT
                i.id AS integration_id,
                i.name AS integration_name,
                i.config_json,
                b.id AS binding_id,
                b.scope_type,
                b.scope_id,
                b.event_types_json,
                b.severity_threshold,
                b.created_at AS binding_created_at
             FROM integrations i
             JOIN integration_bindings b ON b.integration_id = i.id
             WHERE i.kind = 'telegram_bot'
               AND i.is_active = TRUE
               AND b.is_active = TRUE
               AND (
                    b.scope_type = 'global'
                    OR ($1::uuid IS NOT NULL AND b.scope_type = 'cluster' AND b.scope_id = $1)
               )
             ORDER BY
                CASE WHEN b.scope_type = 'cluster' THEN 0 ELSE 1 END ASC,
                b.created_at ASC,
                b.id ASC",
        )
        .bind(cluster_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn insert_delivery_if_absent(
        &self,
        delivery: NewTelegramDelivery,
    ) -> Result<Option<TelegramDeliveryRecord>, sqlx::Error> {
        sqlx::query_as::<_, TelegramDeliveryRecord>(
            "INSERT INTO telegram_deliveries (
                id,
                integration_id,
                integration_binding_id,
                notification_id,
                dedup_key,
                event_type,
                cluster_id,
                cluster_name,
                severity,
                title,
                summary,
                entity_kind,
                entity_id,
                details_url,
                telegram_chat_id,
                parse_mode,
                message_text,
                notification_json,
                status,
                attempt_count,
                max_attempts,
                next_attempt_at,
                status_code,
                status_message,
                status_severity,
                source_component,
                suggested_action,
                correlation_id,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                $18, 'queued', 0, $19, $20, 'queued', 'telegram delivery queued', 'info',
                'control-plane.telegram', '', $21, $22, $22
            )
            ON CONFLICT (integration_binding_id, notification_id) DO NOTHING
            RETURNING
                id,
                integration_id,
                integration_binding_id,
                notification_id,
                dedup_key,
                event_type,
                cluster_id,
                cluster_name,
                severity,
                title,
                summary,
                entity_kind,
                entity_id,
                details_url,
                telegram_chat_id,
                parse_mode,
                message_text,
                notification_json,
                status,
                attempt_count,
                max_attempts,
                next_attempt_at,
                last_attempt_at,
                delivered_at,
                dead_lettered_at,
                lease_token,
                lease_expires_at,
                status_code,
                status_message,
                status_severity,
                source_component,
                suggested_action,
                correlation_id,
                telegram_message_id,
                created_at,
                updated_at",
        )
        .bind(delivery.id)
        .bind(delivery.integration_id)
        .bind(delivery.integration_binding_id)
        .bind(delivery.notification_id)
        .bind(delivery.dedup_key)
        .bind(delivery.event_type)
        .bind(delivery.cluster_id)
        .bind(delivery.cluster_name)
        .bind(delivery.severity)
        .bind(delivery.title)
        .bind(delivery.summary)
        .bind(delivery.entity_kind)
        .bind(delivery.entity_id)
        .bind(delivery.details_url)
        .bind(delivery.telegram_chat_id)
        .bind(delivery.parse_mode)
        .bind(delivery.message_text)
        .bind(delivery.notification_json)
        .bind(delivery.max_attempts)
        .bind(delivery.next_attempt_at)
        .bind(delivery.correlation_id)
        .bind(delivery.created_at)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn start_batch(
        &self,
        batch_id: Uuid,
        correlation_id: &str,
        picked_count: u32,
        started_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO telegram_delivery_batches (
                id,
                correlation_id,
                picked_count,
                success_count,
                retryable_failure_count,
                permanent_failure_count,
                dead_letter_count,
                started_at,
                completed_at,
                duration_ms
            )
            VALUES ($1, $2, $3, 0, 0, 0, 0, $4, NULL, 0)",
        )
        .bind(batch_id)
        .bind(correlation_id)
        .bind(i32::try_from(picked_count).unwrap_or(i32::MAX))
        .bind(started_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn finish_batch(
        &self,
        batch_id: Uuid,
        stats: BatchCompletion,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE telegram_delivery_batches
             SET success_count = $2,
                 retryable_failure_count = $3,
                 permanent_failure_count = $4,
                 dead_letter_count = $5,
                 completed_at = $6,
                 duration_ms = $7
             WHERE id = $1",
        )
        .bind(batch_id)
        .bind(stats.success_count)
        .bind(stats.retryable_failure_count)
        .bind(stats.permanent_failure_count)
        .bind(stats.dead_letter_count)
        .bind(stats.completed_at)
        .bind(stats.duration_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn lease_due_deliveries(
        &self,
        limit: u32,
        lease_token: &str,
        lease_for_ms: i64,
    ) -> Result<Vec<TelegramDeliveryRecord>, sqlx::Error> {
        sqlx::query_as::<_, TelegramDeliveryRecord>(
            "WITH candidates AS (
                SELECT id
                FROM telegram_deliveries
                WHERE next_attempt_at <= NOW()
                  AND (
                        status IN ('queued', 'retry_pending')
                        OR (
                            status = 'sending'
                            AND lease_expires_at IS NOT NULL
                            AND lease_expires_at <= NOW()
                        )
                  )
                ORDER BY next_attempt_at ASC, created_at ASC, id ASC
                LIMIT $1
                FOR UPDATE SKIP LOCKED
             )
             UPDATE telegram_deliveries deliveries
             SET status = 'sending',
                 lease_token = $2,
                 lease_expires_at = NOW() + ($3 * INTERVAL '1 millisecond'),
                 updated_at = NOW(),
                 status_code = 'sending',
                 status_message = 'telegram delivery leased for send attempt',
                 status_severity = 'info',
                 source_component = 'control-plane.telegram',
                 suggested_action = ''
             FROM candidates
             WHERE deliveries.id = candidates.id
             RETURNING
                deliveries.id,
                deliveries.integration_id,
                deliveries.integration_binding_id,
                deliveries.notification_id,
                deliveries.dedup_key,
                deliveries.event_type,
                deliveries.cluster_id,
                deliveries.cluster_name,
                deliveries.severity,
                deliveries.title,
                deliveries.summary,
                deliveries.entity_kind,
                deliveries.entity_id,
                deliveries.details_url,
                deliveries.telegram_chat_id,
                deliveries.parse_mode,
                deliveries.message_text,
                deliveries.notification_json,
                deliveries.status,
                deliveries.attempt_count,
                deliveries.max_attempts,
                deliveries.next_attempt_at,
                deliveries.last_attempt_at,
                deliveries.delivered_at,
                deliveries.dead_lettered_at,
                deliveries.lease_token,
                deliveries.lease_expires_at,
                deliveries.status_code,
                deliveries.status_message,
                deliveries.status_severity,
                deliveries.source_component,
                deliveries.suggested_action,
                deliveries.correlation_id,
                deliveries.telegram_message_id,
                deliveries.created_at,
                deliveries.updated_at",
        )
        .bind(i64::from(limit))
        .bind(lease_token)
        .bind(lease_for_ms)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn record_attempt(
        &self,
        attempt: NewTelegramDeliveryAttempt,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO telegram_delivery_attempts (
                id,
                delivery_id,
                batch_id,
                attempt_number,
                classification,
                http_status,
                telegram_error_code,
                retry_after_seconds,
                duration_ms,
                status_code,
                status_message,
                status_severity,
                source_component,
                suggested_action,
                correlation_id,
                created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15, $16
            )
            ON CONFLICT (delivery_id, attempt_number) DO UPDATE
            SET batch_id = COALESCE(EXCLUDED.batch_id, telegram_delivery_attempts.batch_id),
                classification = EXCLUDED.classification,
                http_status = EXCLUDED.http_status,
                telegram_error_code = EXCLUDED.telegram_error_code,
                retry_after_seconds = EXCLUDED.retry_after_seconds,
                duration_ms = EXCLUDED.duration_ms,
                status_code = EXCLUDED.status_code,
                status_message = EXCLUDED.status_message,
                status_severity = EXCLUDED.status_severity,
                source_component = EXCLUDED.source_component,
                suggested_action = EXCLUDED.suggested_action,
                correlation_id = EXCLUDED.correlation_id,
                created_at = EXCLUDED.created_at
            RETURNING id",
        )
        .bind(attempt.id)
        .bind(attempt.delivery_id)
        .bind(attempt.batch_id)
        .bind(attempt.attempt_number)
        .bind(attempt.classification)
        .bind(attempt.http_status)
        .bind(attempt.telegram_error_code)
        .bind(attempt.retry_after_seconds)
        .bind(attempt.duration_ms)
        .bind(attempt.status_code)
        .bind(attempt.status_message)
        .bind(attempt.status_severity)
        .bind(attempt.source_component)
        .bind(attempt.suggested_action)
        .bind(attempt.correlation_id)
        .bind(attempt.created_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn mark_delivery_succeeded(
        &self,
        delivery_id: Uuid,
        attempt_count: u32,
        completed_at: DateTime<Utc>,
        telegram_message_id: &str,
        status: DeliveryStatusUpdate<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE telegram_deliveries
             SET status = 'delivered',
                 attempt_count = $2,
                 last_attempt_at = $3,
                 delivered_at = $3,
                 dead_lettered_at = NULL,
                 lease_token = NULL,
                 lease_expires_at = NULL,
                 status_code = $4,
                 status_message = $5,
                 status_severity = $6,
                 source_component = $7,
                 suggested_action = $8,
                 telegram_message_id = $9,
                 updated_at = $3
             WHERE id = $1",
        )
        .bind(delivery_id)
        .bind(i32::try_from(attempt_count).unwrap_or(i32::MAX))
        .bind(completed_at)
        .bind(status.status_code)
        .bind(status.status_message)
        .bind(status.status_severity)
        .bind(status.source_component)
        .bind(status.suggested_action)
        .bind(telegram_message_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_delivery_retryable_failure(
        &self,
        delivery_id: Uuid,
        attempt_count: u32,
        next_attempt_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
        status: DeliveryStatusUpdate<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE telegram_deliveries
             SET status = 'retry_pending',
                 attempt_count = $2,
                 next_attempt_at = $3,
                 last_attempt_at = $4,
                 lease_token = NULL,
                 lease_expires_at = NULL,
                 status_code = $5,
                 status_message = $6,
                 status_severity = $7,
                 source_component = $8,
                 suggested_action = $9,
                 updated_at = $4
             WHERE id = $1",
        )
        .bind(delivery_id)
        .bind(i32::try_from(attempt_count).unwrap_or(i32::MAX))
        .bind(next_attempt_at)
        .bind(completed_at)
        .bind(status.status_code)
        .bind(status.status_message)
        .bind(status.status_severity)
        .bind(status.source_component)
        .bind(status.suggested_action)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_delivery_dead_letter(
        &self,
        delivery_id: Uuid,
        attempt_count: u32,
        completed_at: DateTime<Utc>,
        status: DeliveryStatusUpdate<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE telegram_deliveries
             SET status = 'dead_letter',
                 attempt_count = $2,
                 last_attempt_at = $3,
                 dead_lettered_at = $3,
                 lease_token = NULL,
                 lease_expires_at = NULL,
                 status_code = $4,
                 status_message = $5,
                 status_severity = $6,
                 source_component = $7,
                 suggested_action = $8,
                 updated_at = $3
             WHERE id = $1",
        )
        .bind(delivery_id)
        .bind(i32::try_from(attempt_count).unwrap_or(i32::MAX))
        .bind(completed_at)
        .bind(status.status_code)
        .bind(status.status_message)
        .bind(status.status_severity)
        .bind(status.source_component)
        .bind(status.suggested_action)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_delivery_target_state(
        &self,
        integration_id: Uuid,
        integration_binding_id: Uuid,
    ) -> Result<Option<TelegramTargetState>, sqlx::Error> {
        sqlx::query_as::<_, TelegramTargetState>(
            "SELECT
                i.id AS integration_id,
                i.name AS integration_name,
                i.is_active AS integration_is_active,
                i.config_json,
                COALESCE(b.is_active, FALSE) AS binding_is_active
             FROM integrations i
             LEFT JOIN integration_bindings b
               ON b.id = $2
              AND b.integration_id = i.id
             WHERE i.id = $1
               AND i.kind = 'telegram_bot'
             LIMIT 1",
        )
        .bind(integration_id)
        .bind(integration_binding_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_telegram_integration(
        &self,
        integration_id: Uuid,
    ) -> Result<Option<TelegramIntegrationState>, sqlx::Error> {
        sqlx::query_as::<_, TelegramIntegrationState>(
            "SELECT
                id AS integration_id,
                name AS integration_name,
                is_active AS integration_is_active,
                config_json
             FROM integrations
             WHERE id = $1
               AND kind = 'telegram_bot'
             LIMIT 1",
        )
        .bind(integration_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_healthcheck_run(
        &self,
        request: NewTelegramHealthcheckRun,
    ) -> Result<TelegramHealthcheckRunRecord, sqlx::Error> {
        sqlx::query_as::<_, TelegramHealthcheckRunRecord>(
            "INSERT INTO telegram_healthcheck_runs (
                id,
                request_id,
                integration_id,
                chat_id_override,
                resolved_chat_id,
                status,
                classification,
                telegram_message_id,
                status_code,
                status_message,
                status_severity,
                source_component,
                suggested_action,
                correlation_id,
                created_at,
                updated_at,
                completed_at
            )
            VALUES (
                $1, $2, $3, $4, '', 'running', '', '', 'healthcheck_requested',
                'telegram healthcheck queued', 'info', 'control-plane.telegram', '', $5, $6, $6, NULL
            )
            ON CONFLICT (request_id) DO UPDATE
            SET updated_at = telegram_healthcheck_runs.updated_at
            RETURNING
                id,
                request_id,
                integration_id,
                chat_id_override,
                resolved_chat_id,
                status,
                classification,
                telegram_message_id,
                status_code,
                status_message,
                status_severity,
                source_component,
                suggested_action,
                correlation_id,
                created_at,
                updated_at,
                completed_at",
        )
        .bind(request.id)
        .bind(request.request_id)
        .bind(request.integration_id)
        .bind(request.chat_id_override)
        .bind(request.correlation_id)
        .bind(request.created_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn complete_healthcheck_run(
        &self,
        healthcheck_run_id: Uuid,
        status: HealthcheckCompletion<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE telegram_healthcheck_runs
             SET resolved_chat_id = $2,
                 status = $3,
                 classification = $4,
                 telegram_message_id = $5,
                 status_code = $6,
                 status_message = $7,
                 status_severity = $8,
                 source_component = $9,
                 suggested_action = $10,
                 updated_at = $11,
                 completed_at = $11
             WHERE id = $1",
        )
        .bind(healthcheck_run_id)
        .bind(status.resolved_chat_id)
        .bind(status.status)
        .bind(status.classification)
        .bind(status.telegram_message_id)
        .bind(status.status_code)
        .bind(status.status_message)
        .bind(status.status_severity)
        .bind(status.source_component)
        .bind(status.suggested_action)
        .bind(status.completed_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct TelegramDeliveryRoute {
    pub integration_id: Uuid,
    pub integration_name: String,
    pub config_json: Value,
    pub binding_id: Uuid,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub event_types_json: Value,
    pub severity_threshold: String,
    pub binding_created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct TelegramDeliveryRecord {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub integration_binding_id: Uuid,
    pub notification_id: String,
    pub dedup_key: String,
    pub event_type: String,
    pub cluster_id: Option<Uuid>,
    pub cluster_name: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub details_url: String,
    pub telegram_chat_id: String,
    pub parse_mode: String,
    pub message_text: String,
    pub notification_json: Value,
    pub status: String,
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub dead_lettered_at: Option<DateTime<Utc>>,
    pub lease_token: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub status_code: String,
    pub status_message: String,
    pub status_severity: String,
    pub source_component: String,
    pub suggested_action: String,
    pub correlation_id: String,
    pub telegram_message_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct TelegramTargetState {
    pub integration_id: Uuid,
    pub integration_name: String,
    pub integration_is_active: bool,
    pub config_json: Value,
    pub binding_is_active: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct TelegramIntegrationState {
    pub integration_id: Uuid,
    pub integration_name: String,
    pub integration_is_active: bool,
    pub config_json: Value,
}

#[derive(Debug, Clone, FromRow)]
pub struct TelegramHealthcheckRunRecord {
    pub id: Uuid,
    pub request_id: String,
    pub integration_id: Uuid,
    pub chat_id_override: String,
    pub resolved_chat_id: String,
    pub status: String,
    pub classification: String,
    pub telegram_message_id: String,
    pub status_code: String,
    pub status_message: String,
    pub status_severity: String,
    pub source_component: String,
    pub suggested_action: String,
    pub correlation_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewTelegramDelivery {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub integration_binding_id: Uuid,
    pub notification_id: String,
    pub dedup_key: String,
    pub event_type: String,
    pub cluster_id: Option<Uuid>,
    pub cluster_name: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub details_url: String,
    pub telegram_chat_id: String,
    pub parse_mode: String,
    pub message_text: String,
    pub notification_json: Value,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub correlation_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewTelegramDeliveryAttempt {
    pub id: Uuid,
    pub delivery_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub attempt_number: i32,
    pub classification: String,
    pub http_status: Option<i32>,
    pub telegram_error_code: String,
    pub retry_after_seconds: Option<i32>,
    pub duration_ms: i64,
    pub status_code: String,
    pub status_message: String,
    pub status_severity: String,
    pub source_component: String,
    pub suggested_action: String,
    pub correlation_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeliveryStatusUpdate<'a> {
    pub status_code: &'a str,
    pub status_message: &'a str,
    pub status_severity: &'a str,
    pub source_component: &'a str,
    pub suggested_action: &'a str,
}

#[derive(Debug, Clone)]
pub struct BatchCompletion {
    pub success_count: i32,
    pub retryable_failure_count: i32,
    pub permanent_failure_count: i32,
    pub dead_letter_count: i32,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: i64,
}

#[derive(Debug, Clone)]
pub struct NewTelegramHealthcheckRun {
    pub id: Uuid,
    pub request_id: String,
    pub integration_id: Uuid,
    pub chat_id_override: String,
    pub correlation_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct HealthcheckCompletion<'a> {
    pub resolved_chat_id: &'a str,
    pub status: &'a str,
    pub classification: &'a str,
    pub telegram_message_id: &'a str,
    pub status_code: &'a str,
    pub status_message: &'a str,
    pub status_severity: &'a str,
    pub source_component: &'a str,
    pub suggested_action: &'a str,
    pub completed_at: DateTime<Utc>,
}
