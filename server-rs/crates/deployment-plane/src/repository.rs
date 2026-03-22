use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{postgres::PgRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::models::{
    DeploymentAttemptRecord, DeploymentJobPayload, DeploymentJobRecord, DeploymentJobStatus,
    DeploymentJobType, DeploymentJobView, DeploymentSnapshot, DeploymentStepRecord,
    DeploymentStepStatus, DeploymentTargetRecord, DeploymentTargetStatus, ExecutorKind,
    JobSummaryData, ListJobsFilter, RetryStrategy, RunningAttempt,
};

#[derive(Clone)]
pub struct DeploymentRepository {
    pool: PgPool,
}

impl DeploymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
    }

    pub async fn create_job(
        &self,
        payload: &DeploymentJobPayload,
        executor_kind: ExecutorKind,
    ) -> Result<RunningAttempt, sqlx::Error> {
        let job_id = Uuid::new_v4();
        let attempt_id = Uuid::new_v4();
        let now = Utc::now();
        let payload_json = serde_json::to_value(payload).unwrap_or_else(|_| json!({}));
        let mut summary = JobSummaryData::default();
        summary.current_phase = "queued".to_string();
        summary.total_targets = payload.snapshot.targets.len() as u32;
        summary.pending_targets = payload.snapshot.targets.len() as u32;
        summary.attempt_count = 1;
        summary.current_attempt_id = Some(attempt_id);

        let summary_json = serde_json::to_value(&summary).unwrap_or_else(|_| json!({}));

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO deployment_jobs (
                id,
                job_type,
                status,
                requested_by,
                policy_id,
                policy_revision_id,
                credential_profile_id,
                executor_kind,
                payload_json,
                summary_json,
                created_at,
                started_at,
                finished_at,
                updated_at
             )
             VALUES ($1, $2, 'queued', $3, $4, $5, $6, $7, $8, $9, $10, NULL, NULL, $10)",
        )
        .bind(job_id)
        .bind(payload.request.job_type.as_str())
        .bind(&payload.request.requested_by)
        .bind(payload.snapshot.policy.policy_id)
        .bind(payload.snapshot.policy.policy_revision_id)
        .bind(payload.snapshot.credentials.credential_profile_id)
        .bind(executor_kind.as_str())
        .bind(&payload_json)
        .bind(&summary_json)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO deployment_attempts (
                id,
                deployment_job_id,
                attempt_no,
                status,
                triggered_by,
                reason,
                created_at,
                started_at,
                finished_at,
                updated_at
             )
             VALUES ($1, $2, 1, 'queued', $3, 'initial deployment request', $4, NULL, NULL, $4)",
        )
        .bind(attempt_id)
        .bind(job_id)
        .bind(&payload.request.requested_by)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let mut target_records = Vec::new();
        for target in &payload.snapshot.targets {
            let target_id = Uuid::new_v4();
            let bootstrap_payload = json!({
                "token_id": target.bootstrap.token_id,
                "bootstrap_token": target.bootstrap.bootstrap_token,
                "expires_at": target.bootstrap.expires_at,
                "bootstrap_yaml": target.bootstrap.bootstrap_yaml,
            });
            let artifact_payload =
                serde_json::to_value(&target.artifact).unwrap_or_else(|_| json!({}));

            sqlx::query(
                "INSERT INTO deployment_targets (
                    id,
                    deployment_job_id,
                    deployment_attempt_id,
                    host_id,
                    hostname_snapshot,
                    status,
                    bootstrap_payload_json,
                    artifact_payload_json,
                    rendered_vars_json,
                    error_message,
                    created_at,
                    started_at,
                    finished_at,
                    updated_at
                 )
                 VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $8, '', $9, NULL, NULL, $9)",
            )
            .bind(target_id)
            .bind(job_id)
            .bind(attempt_id)
            .bind(target.host.host_id)
            .bind(&target.host.hostname)
            .bind(&bootstrap_payload)
            .bind(&artifact_payload)
            .bind(&target.rendered_vars)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            target_records.push(DeploymentTargetRecord {
                id: target_id,
                deployment_job_id: job_id,
                deployment_attempt_id: attempt_id,
                host_id: target.host.host_id,
                hostname_snapshot: target.host.hostname.clone(),
                status: DeploymentTargetStatus::Pending,
                bootstrap_payload_json: bootstrap_payload,
                artifact_payload_json: artifact_payload,
                rendered_vars_json: target.rendered_vars.clone(),
                error_message: String::new(),
                created_at: now,
                started_at: None,
                finished_at: None,
                updated_at: now,
            });
        }

        tx.commit().await?;

        let job = self
            .get_job(job_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;
        let attempt = self
            .get_latest_attempt(job_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        Ok(RunningAttempt {
            job,
            attempt,
            snapshot: payload.snapshot.clone(),
            targets: target_records,
        })
    }

    pub async fn create_retry_attempt(
        &self,
        job_id: Uuid,
        snapshot: &DeploymentSnapshot,
        triggered_by: &str,
        reason: &str,
        strategy: RetryStrategy,
        previous_targets: &[DeploymentTargetRecord],
    ) -> Result<RunningAttempt, sqlx::Error> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;
        let attempt_no: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(attempt_no), 0) + 1
             FROM deployment_attempts
             WHERE deployment_job_id = $1",
        )
        .bind(job_id)
        .fetch_one(&mut *tx)
        .await?;
        let attempt_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO deployment_attempts (
                id,
                deployment_job_id,
                attempt_no,
                status,
                triggered_by,
                reason,
                created_at,
                started_at,
                finished_at,
                updated_at
             )
             VALUES ($1, $2, $3, 'queued', $4, $5, $6, NULL, NULL, $6)",
        )
        .bind(attempt_id)
        .bind(job_id)
        .bind(attempt_no)
        .bind(triggered_by)
        .bind(reason)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let retry_host_ids: Vec<Uuid> = match strategy {
            RetryStrategy::All => previous_targets
                .iter()
                .map(|target| target.host_id)
                .collect(),
            RetryStrategy::FailedOnly => previous_targets
                .iter()
                .filter(|target| target.status == DeploymentTargetStatus::Failed)
                .map(|target| target.host_id)
                .collect(),
        };

        let retry_target_host_ids = if retry_host_ids.is_empty() {
            previous_targets
                .iter()
                .map(|target| target.host_id)
                .collect::<Vec<_>>()
        } else {
            retry_host_ids
        };

        let mut target_records = Vec::new();
        for target in &snapshot.targets {
            if !retry_target_host_ids.contains(&target.host.host_id) {
                continue;
            }

            let target_id = Uuid::new_v4();
            let bootstrap_payload = json!({
                "token_id": target.bootstrap.token_id,
                "bootstrap_token": target.bootstrap.bootstrap_token,
                "expires_at": target.bootstrap.expires_at,
                "bootstrap_yaml": target.bootstrap.bootstrap_yaml,
            });
            let artifact_payload =
                serde_json::to_value(&target.artifact).unwrap_or_else(|_| json!({}));

            sqlx::query(
                "INSERT INTO deployment_targets (
                    id,
                    deployment_job_id,
                    deployment_attempt_id,
                    host_id,
                    hostname_snapshot,
                    status,
                    bootstrap_payload_json,
                    artifact_payload_json,
                    rendered_vars_json,
                    error_message,
                    created_at,
                    started_at,
                    finished_at,
                    updated_at
                 )
                 VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $8, '', $9, NULL, NULL, $9)",
            )
            .bind(target_id)
            .bind(job_id)
            .bind(attempt_id)
            .bind(target.host.host_id)
            .bind(&target.host.hostname)
            .bind(&bootstrap_payload)
            .bind(&artifact_payload)
            .bind(&target.rendered_vars)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            target_records.push(DeploymentTargetRecord {
                id: target_id,
                deployment_job_id: job_id,
                deployment_attempt_id: attempt_id,
                host_id: target.host.host_id,
                hostname_snapshot: target.host.hostname.clone(),
                status: DeploymentTargetStatus::Pending,
                bootstrap_payload_json: bootstrap_payload,
                artifact_payload_json: artifact_payload,
                rendered_vars_json: target.rendered_vars.clone(),
                error_message: String::new(),
                created_at: now,
                started_at: None,
                finished_at: None,
                updated_at: now,
            });
        }

        let mut summary = JobSummaryData::default();
        summary.current_phase = "queued".to_string();
        summary.total_targets = target_records.len() as u32;
        summary.pending_targets = target_records.len() as u32;
        summary.running_targets = 0;
        summary.succeeded_targets = 0;
        summary.failed_targets = 0;
        summary.cancelled_targets = 0;
        summary.attempt_count = attempt_no as u32;
        summary.current_attempt_id = Some(attempt_id);

        sqlx::query(
            "UPDATE deployment_jobs
             SET status = 'queued',
                 started_at = NULL,
                 finished_at = NULL,
                 updated_at = $2,
                 summary_json = $3
             WHERE id = $1",
        )
        .bind(job_id)
        .bind(now)
        .bind(serde_json::to_value(&summary).unwrap_or_else(|_| json!({})))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let job = self
            .get_job(job_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;
        let attempt = self
            .get_latest_attempt(job_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        Ok(RunningAttempt {
            job,
            attempt,
            snapshot: snapshot.clone(),
            targets: target_records,
        })
    }

    pub async fn get_job_view(
        &self,
        job_id: Uuid,
    ) -> Result<Option<DeploymentJobView>, sqlx::Error> {
        let Some(job) = self.get_job(job_id).await? else {
            return Ok(None);
        };
        let attempts = self.list_attempts(job_id).await?;
        let current_attempt_id = job
            .summary_data()
            .current_attempt_id
            .or_else(|| attempts.first().map(|attempt| attempt.id));
        let (targets, steps) = if let Some(attempt_id) = current_attempt_id {
            (
                self.list_targets_for_attempt(attempt_id).await?,
                self.list_steps_for_attempt(attempt_id).await?,
            )
        } else {
            (Vec::new(), Vec::new())
        };

        Ok(Some(DeploymentJobView {
            job,
            attempts,
            targets,
            steps,
        }))
    }

    pub async fn get_job(&self, job_id: Uuid) -> Result<Option<DeploymentJobRecord>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT
                id,
                job_type,
                status,
                requested_by,
                policy_id,
                policy_revision_id,
                credential_profile_id,
                executor_kind,
                payload_json,
                summary_json,
                created_at,
                started_at,
                finished_at,
                updated_at
             FROM deployment_jobs
             WHERE id = $1
             LIMIT 1",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(job_from_row))
    }

    pub async fn get_latest_attempt(
        &self,
        job_id: Uuid,
    ) -> Result<Option<DeploymentAttemptRecord>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT
                id,
                deployment_job_id,
                attempt_no,
                status,
                triggered_by,
                reason,
                created_at,
                started_at,
                finished_at,
                updated_at
             FROM deployment_attempts
             WHERE deployment_job_id = $1
             ORDER BY attempt_no DESC
             LIMIT 1",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(attempt_from_row))
    }

    pub async fn list_attempts(
        &self,
        job_id: Uuid,
    ) -> Result<Vec<DeploymentAttemptRecord>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT
                id,
                deployment_job_id,
                attempt_no,
                status,
                triggered_by,
                reason,
                created_at,
                started_at,
                finished_at,
                updated_at
             FROM deployment_attempts
             WHERE deployment_job_id = $1
             ORDER BY attempt_no DESC",
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(attempt_from_row).collect())
    }

    pub async fn list_targets_for_attempt(
        &self,
        attempt_id: Uuid,
    ) -> Result<Vec<DeploymentTargetRecord>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT
                id,
                deployment_job_id,
                deployment_attempt_id,
                host_id,
                hostname_snapshot,
                status,
                bootstrap_payload_json,
                artifact_payload_json,
                rendered_vars_json,
                error_message,
                created_at,
                started_at,
                finished_at,
                updated_at
             FROM deployment_targets
             WHERE deployment_attempt_id = $1
             ORDER BY hostname_snapshot ASC",
        )
        .bind(attempt_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(target_from_row).collect())
    }

    pub async fn list_steps_for_attempt(
        &self,
        attempt_id: Uuid,
    ) -> Result<Vec<DeploymentStepRecord>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT
                id,
                deployment_job_id,
                deployment_attempt_id,
                deployment_target_id,
                step_name,
                status,
                message,
                payload_json,
                created_at,
                updated_at
             FROM deployment_steps
             WHERE deployment_attempt_id = $1
             ORDER BY created_at ASC",
        )
        .bind(attempt_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(step_from_row).collect())
    }

    pub async fn list_jobs(
        &self,
        filter: &ListJobsFilter,
    ) -> Result<(Vec<DeploymentJobRecord>, u64), sqlx::Error> {
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id, job_type, status, requested_by, policy_id, policy_revision_id, credential_profile_id, executor_kind, payload_json, summary_json, created_at, started_at, finished_at, updated_at FROM deployment_jobs WHERE 1=1",
        );
        append_filter_clauses(&mut builder, filter);
        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(filter.limit as i64);
        builder.push(" OFFSET ");
        builder.push_bind(filter.offset as i64);

        let rows = builder.build().fetch_all(&self.pool).await?;
        let jobs = rows.into_iter().map(job_from_row).collect();

        let mut count_builder = QueryBuilder::<Postgres>::new(
            "SELECT COUNT(*) as count FROM deployment_jobs WHERE 1=1",
        );
        append_filter_clauses(&mut count_builder, filter);
        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await?;

        Ok((jobs, total.max(0) as u64))
    }

    pub async fn load_job_payload(
        &self,
        job_id: Uuid,
    ) -> Result<Option<DeploymentJobPayload>, sqlx::Error> {
        let payload = sqlx::query_scalar::<_, Value>(
            "SELECT payload_json
             FROM deployment_jobs
             WHERE id = $1
             LIMIT 1",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(payload.and_then(|value| serde_json::from_value(value).ok()))
    }

    pub async fn mark_attempt_running(
        &self,
        job_id: Uuid,
        attempt_id: Uuid,
        current_phase: &str,
    ) -> Result<DeploymentJobRecord, sqlx::Error> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE deployment_attempts
             SET status = 'running',
                 started_at = COALESCE(started_at, $2),
                 updated_at = $2
             WHERE id = $1",
        )
        .bind(attempt_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.refresh_job_summary(
            job_id,
            Some(attempt_id),
            current_phase,
            DeploymentJobStatus::Running,
        )
        .await
    }

    pub async fn mark_target_running(&self, target_id: Uuid) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE deployment_targets
             SET status = 'running',
                 started_at = COALESCE(started_at, $2),
                 updated_at = $2
             WHERE id = $1",
        )
        .bind(target_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn complete_target(
        &self,
        target_id: Uuid,
        status: DeploymentTargetStatus,
        error_message: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE deployment_targets
             SET status = $2,
                 error_message = $3,
                 finished_at = $4,
                 updated_at = $4
             WHERE id = $1",
        )
        .bind(target_id)
        .bind(status.as_str())
        .bind(error_message.unwrap_or(""))
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_step(
        &self,
        job_id: Uuid,
        attempt_id: Uuid,
        target_id: Option<Uuid>,
        step_name: &str,
        status: DeploymentStepStatus,
        message: &str,
        payload_json: &Value,
    ) -> Result<DeploymentStepRecord, sqlx::Error> {
        let now = Utc::now();
        let row = sqlx::query(
            "INSERT INTO deployment_steps (
                id,
                deployment_job_id,
                deployment_attempt_id,
                deployment_target_id,
                step_name,
                status,
                message,
                payload_json,
                created_at,
                updated_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
             RETURNING
                id,
                deployment_job_id,
                deployment_attempt_id,
                deployment_target_id,
                step_name,
                status,
                message,
                payload_json,
                created_at,
                updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(job_id)
        .bind(attempt_id)
        .bind(target_id)
        .bind(step_name)
        .bind(status.as_str())
        .bind(message)
        .bind(payload_json)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(step_from_row(row))
    }

    pub async fn refresh_job_summary(
        &self,
        job_id: Uuid,
        attempt_id: Option<Uuid>,
        current_phase: &str,
        job_status: DeploymentJobStatus,
    ) -> Result<DeploymentJobRecord, sqlx::Error> {
        let now = Utc::now();
        let summary_json = self
            .build_summary_json(job_id, attempt_id, current_phase)
            .await?;

        let started_at_expr = if job_status == DeploymentJobStatus::Running {
            Some(now)
        } else {
            None
        };
        let finished_at_expr = if job_status.is_terminal() {
            Some(now)
        } else {
            None
        };

        sqlx::query(
            "UPDATE deployment_jobs
             SET status = $2,
                 started_at = COALESCE(started_at, $3),
                 finished_at = CASE
                     WHEN $4 IS NULL THEN NULL
                     ELSE $4
                 END,
                 updated_at = $5,
                 summary_json = $6
             WHERE id = $1",
        )
        .bind(job_id)
        .bind(job_status.as_str())
        .bind(started_at_expr)
        .bind(finished_at_expr)
        .bind(now)
        .bind(summary_json)
        .execute(&self.pool)
        .await?;

        self.get_job(job_id).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn finalize_attempt(
        &self,
        job_id: Uuid,
        attempt_id: Uuid,
        job_status: DeploymentJobStatus,
        current_phase: &str,
    ) -> Result<DeploymentJobRecord, sqlx::Error> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE deployment_attempts
             SET status = $2,
                 started_at = COALESCE(started_at, $3),
                 finished_at = $3,
                 updated_at = $3
             WHERE id = $1",
        )
        .bind(attempt_id)
        .bind(job_status.as_str())
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.refresh_job_summary(job_id, Some(attempt_id), current_phase, job_status)
            .await
    }

    pub async fn force_cancel_job(
        &self,
        job_id: Uuid,
        reason: &str,
    ) -> Result<Option<DeploymentJobRecord>, sqlx::Error> {
        let Some(attempt) = self.get_latest_attempt(job_id).await? else {
            return Ok(None);
        };
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "UPDATE deployment_targets
             SET status = CASE
                    WHEN status IN ('succeeded', 'failed', 'cancelled') THEN status
                    ELSE 'cancelled'
                 END,
                 error_message = CASE
                    WHEN status IN ('succeeded', 'failed', 'cancelled') THEN error_message
                    ELSE $2
                 END,
                 finished_at = CASE
                    WHEN status IN ('succeeded', 'failed', 'cancelled') THEN finished_at
                    ELSE $3
                 END,
                 updated_at = $3
             WHERE deployment_attempt_id = $1",
        )
        .bind(attempt.id)
        .bind(reason)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE deployment_attempts
             SET status = 'cancelled',
                 finished_at = $2,
                 updated_at = $2
             WHERE id = $1",
        )
        .bind(attempt.id)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO deployment_steps (
                id,
                deployment_job_id,
                deployment_attempt_id,
                deployment_target_id,
                step_name,
                status,
                message,
                payload_json,
                created_at,
                updated_at
             )
             VALUES ($1, $2, $3, NULL, 'job.cancelled', 'skipped', $4, '{}'::jsonb, $5, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(job_id)
        .bind(attempt.id)
        .bind(reason)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let summary_json = self
            .build_summary_json_within_tx(&mut tx, job_id, Some(attempt.id), "cancelled")
            .await?;
        sqlx::query(
            "UPDATE deployment_jobs
             SET status = 'cancelled',
                 finished_at = $2,
                 updated_at = $2,
                 summary_json = $3
             WHERE id = $1",
        )
        .bind(job_id)
        .bind(now)
        .bind(summary_json)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_job(job_id).await
    }

    pub async fn reconcile_stale_attempts(&self) -> Result<Vec<Uuid>, sqlx::Error> {
        let stale_attempt_rows = sqlx::query(
            "SELECT id, deployment_job_id
             FROM deployment_attempts
             WHERE status IN ('queued', 'running')",
        )
        .fetch_all(&self.pool)
        .await?;

        if stale_attempt_rows.is_empty() {
            return Ok(Vec::new());
        }

        let now = Utc::now();
        let mut tx = self.pool.begin().await?;
        let mut reconciled_jobs = Vec::new();

        for row in stale_attempt_rows {
            let attempt_id: Uuid = row.get("id");
            let job_id: Uuid = row.get("deployment_job_id");
            reconciled_jobs.push(job_id);

            sqlx::query(
                "UPDATE deployment_targets
                 SET status = 'failed',
                     error_message = COALESCE(NULLIF(error_message, ''), 'deployment-plane restarted before the attempt completed'),
                     finished_at = COALESCE(finished_at, $2),
                     updated_at = $2
                 WHERE deployment_attempt_id = $1
                   AND status IN ('pending', 'running')",
            )
            .bind(attempt_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "UPDATE deployment_attempts
                 SET status = 'failed',
                     finished_at = COALESCE(finished_at, $2),
                     updated_at = $2
                 WHERE id = $1",
            )
            .bind(attempt_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO deployment_steps (
                    id,
                    deployment_job_id,
                    deployment_attempt_id,
                    deployment_target_id,
                    step_name,
                    status,
                    message,
                    payload_json,
                    created_at,
                    updated_at
                 )
                 VALUES ($1, $2, $3, NULL, 'reconcile.after_restart', 'failed', $4, '{}'::jsonb, $5, $5)",
            )
            .bind(Uuid::new_v4())
            .bind(job_id)
            .bind(attempt_id)
            .bind("deployment-plane restarted before the attempt completed")
            .bind(now)
            .execute(&mut *tx)
            .await?;

            let summary_json = self
                .build_summary_json_within_tx(&mut tx, job_id, Some(attempt_id), "failed")
                .await?;
            sqlx::query(
                "UPDATE deployment_jobs
                 SET status = 'failed',
                     finished_at = COALESCE(finished_at, $2),
                     updated_at = $2,
                     summary_json = $3
                 WHERE id = $1",
            )
            .bind(job_id)
            .bind(now)
            .bind(summary_json)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        reconciled_jobs.sort();
        reconciled_jobs.dedup();
        Ok(reconciled_jobs)
    }

    async fn build_summary_json(
        &self,
        job_id: Uuid,
        attempt_id: Option<Uuid>,
        current_phase: &str,
    ) -> Result<Value, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let summary = self
            .build_summary_json_within_tx(&mut tx, job_id, attempt_id, current_phase)
            .await?;
        tx.rollback().await?;
        Ok(summary)
    }

    async fn build_summary_json_within_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        job_id: Uuid,
        attempt_id: Option<Uuid>,
        current_phase: &str,
    ) -> Result<Value, sqlx::Error> {
        let attempt_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM deployment_attempts
             WHERE deployment_job_id = $1",
        )
        .bind(job_id)
        .fetch_one(&mut **tx)
        .await?;

        let current_attempt_id = match attempt_id {
            Some(attempt_id) => Some(attempt_id),
            None => {
                sqlx::query_scalar(
                    "SELECT id
                 FROM deployment_attempts
                 WHERE deployment_job_id = $1
                 ORDER BY attempt_no DESC
                 LIMIT 1",
                )
                .bind(job_id)
                .fetch_optional(&mut **tx)
                .await?
            }
        };

        let mut summary = JobSummaryData {
            current_phase: current_phase.to_string(),
            attempt_count: attempt_count.max(0) as u32,
            current_attempt_id,
            ..JobSummaryData::default()
        };

        if let Some(attempt_id) = current_attempt_id {
            let rows = sqlx::query(
                "SELECT status, COUNT(*) as count
                 FROM deployment_targets
                 WHERE deployment_attempt_id = $1
                 GROUP BY status",
            )
            .bind(attempt_id)
            .fetch_all(&mut **tx)
            .await?;

            for row in rows {
                let status: String = row.get("status");
                let count: i64 = row.get("count");
                match status.as_str() {
                    "pending" => summary.pending_targets = count.max(0) as u32,
                    "running" => summary.running_targets = count.max(0) as u32,
                    "succeeded" => summary.succeeded_targets = count.max(0) as u32,
                    "failed" => summary.failed_targets = count.max(0) as u32,
                    "cancelled" => summary.cancelled_targets = count.max(0) as u32,
                    _ => {}
                }
            }
            summary.total_targets = summary.pending_targets
                + summary.running_targets
                + summary.succeeded_targets
                + summary.failed_targets
                + summary.cancelled_targets;
        }

        Ok(serde_json::to_value(summary).unwrap_or_else(|_| json!({})))
    }
}

fn append_filter_clauses<'a>(builder: &mut QueryBuilder<'a, Postgres>, filter: &'a ListJobsFilter) {
    if let Some(status) = filter.status {
        builder.push(" AND status = ");
        builder.push_bind(status.as_str());
    }
    if let Some(job_type) = filter.job_type {
        builder.push(" AND job_type = ");
        builder.push_bind(job_type.as_str());
    }
    if let Some(requested_by) = &filter.requested_by {
        builder.push(" AND requested_by = ");
        builder.push_bind(requested_by);
    }
    if let Some(created_after) = filter.created_after {
        builder.push(" AND created_at >= ");
        builder.push_bind(created_after);
    }
    if let Some(created_before) = filter.created_before {
        builder.push(" AND created_at <= ");
        builder.push_bind(created_before);
    }
}

fn job_from_row(row: PgRow) -> DeploymentJobRecord {
    DeploymentJobRecord {
        id: row.get("id"),
        job_type: DeploymentJobType::from_str(row.get::<String, _>("job_type").as_str())
            .expect("valid job_type"),
        status: DeploymentJobStatus::from_str(row.get::<String, _>("status").as_str())
            .expect("valid job status"),
        requested_by: row.get("requested_by"),
        policy_id: row.get("policy_id"),
        policy_revision_id: row.get("policy_revision_id"),
        credential_profile_id: row.get("credential_profile_id"),
        executor_kind: ExecutorKind::from_str(row.get::<String, _>("executor_kind").as_str())
            .expect("valid executor kind"),
        payload_json: row.get("payload_json"),
        summary_json: row.get("summary_json"),
        created_at: row.get("created_at"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        updated_at: row.get("updated_at"),
    }
}

fn attempt_from_row(row: PgRow) -> DeploymentAttemptRecord {
    DeploymentAttemptRecord {
        id: row.get("id"),
        deployment_job_id: row.get("deployment_job_id"),
        attempt_no: row.get("attempt_no"),
        status: DeploymentJobStatus::from_str(row.get::<String, _>("status").as_str())
            .expect("valid attempt status"),
        triggered_by: row.get("triggered_by"),
        reason: row.get("reason"),
        created_at: row.get("created_at"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        updated_at: row.get("updated_at"),
    }
}

fn target_from_row(row: PgRow) -> DeploymentTargetRecord {
    DeploymentTargetRecord {
        id: row.get("id"),
        deployment_job_id: row.get("deployment_job_id"),
        deployment_attempt_id: row.get("deployment_attempt_id"),
        host_id: row.get("host_id"),
        hostname_snapshot: row.get("hostname_snapshot"),
        status: DeploymentTargetStatus::from_str(row.get::<String, _>("status").as_str())
            .expect("valid target status"),
        bootstrap_payload_json: row.get("bootstrap_payload_json"),
        artifact_payload_json: row.get("artifact_payload_json"),
        rendered_vars_json: row.get("rendered_vars_json"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        updated_at: row.get("updated_at"),
    }
}

fn step_from_row(row: PgRow) -> DeploymentStepRecord {
    DeploymentStepRecord {
        id: row.get("id"),
        deployment_job_id: row.get("deployment_job_id"),
        deployment_attempt_id: row.get("deployment_attempt_id"),
        deployment_target_id: row.get("deployment_target_id"),
        step_name: row.get("step_name"),
        status: DeploymentStepStatus::from_str(row.get::<String, _>("status").as_str())
            .expect("valid step status"),
        message: row.get("message"),
        payload_json: row.get("payload_json"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
