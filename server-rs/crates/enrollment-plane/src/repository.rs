use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

const DEFAULT_POLICY_NAME: &str = "Default Policy";
const DEFAULT_POLICY_DESCRIPTION: &str = "Dev bootstrap policy for enrollment-plane";
const DEFAULT_POLICY_REVISION: &str = "rev-1";

#[derive(Clone)]
pub struct EnrollmentRepository {
    pool: PgPool,
}

impl EnrollmentRepository {
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

    pub async fn ensure_dev_bootstrap(
        &self,
        token_hash: &str,
        policy_body_json: &Value,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        let policy_id =
            match sqlx::query_scalar::<_, Uuid>("SELECT id FROM policies WHERE name = $1 LIMIT 1")
                .bind(DEFAULT_POLICY_NAME)
                .fetch_optional(&mut *tx)
                .await?
            {
                Some(policy_id) => {
                    sqlx::query(
                        "UPDATE policies
                     SET is_active = TRUE, updated_at = $2
                     WHERE id = $1",
                    )
                    .bind(policy_id)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
                    policy_id
                }
                None => {
                    let policy_id = Uuid::new_v4();
                    sqlx::query(
                    "INSERT INTO policies (id, name, description, is_active, created_at, updated_at)
                     VALUES ($1, $2, $3, TRUE, $4, $4)",
                )
                .bind(policy_id)
                .bind(DEFAULT_POLICY_NAME)
                .bind(DEFAULT_POLICY_DESCRIPTION)
                .bind(now)
                .execute(&mut *tx)
                .await?;
                    policy_id
                }
            };

        let revision_id = match sqlx::query_scalar::<_, Uuid>(
            "SELECT id
             FROM policy_revisions
             WHERE policy_id = $1 AND revision = $2
             LIMIT 1",
        )
        .bind(policy_id)
        .bind(DEFAULT_POLICY_REVISION)
        .fetch_optional(&mut *tx)
        .await?
        {
            Some(revision_id) => revision_id,
            None => {
                let revision_id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO policy_revisions (id, policy_id, revision, body_json, created_at)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(revision_id)
                .bind(policy_id)
                .bind(DEFAULT_POLICY_REVISION)
                .bind(policy_body_json)
                .bind(now)
                .execute(&mut *tx)
                .await?;
                revision_id
            }
        };

        let token_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1
                FROM enrollment_tokens
                WHERE token_hash = $1
            )",
        )
        .bind(token_hash)
        .fetch_one(&mut *tx)
        .await?;

        if !token_exists {
            sqlx::query(
                "INSERT INTO enrollment_tokens (
                    id,
                    token_hash,
                    policy_id,
                    policy_revision_id,
                    expires_at,
                    used_at,
                    created_at,
                    revoked_at
                 )
                 VALUES ($1, $2, $3, $4, $5, NULL, $6, NULL)",
            )
            .bind(Uuid::new_v4())
            .bind(token_hash)
            .bind(policy_id)
            .bind(revision_id)
            .bind(now + Duration::days(3650))
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await
    }

    pub async fn token_policy(
        &self,
        token_hash: &str,
    ) -> Result<Option<PolicySnapshot>, sqlx::Error> {
        sqlx::query_as::<_, PolicySnapshot>(
            "SELECT
                et.policy_id AS policy_id,
                et.policy_revision_id AS policy_revision_id,
                pr.revision AS policy_revision,
                pr.body_json AS policy_body_json
             FROM enrollment_tokens et
             JOIN policy_revisions pr ON pr.id = et.policy_revision_id
             WHERE et.token_hash = $1
               AND et.revoked_at IS NULL
               AND (et.expires_at IS NULL OR et.expires_at > NOW())
             LIMIT 1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn mark_token_used(
        &self,
        token_hash: &str,
        used_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE enrollment_tokens
             SET used_at = COALESCE(used_at, $2)
             WHERE token_hash = $1",
        )
        .bind(token_hash)
        .bind(used_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn policy_revision_matches(
        &self,
        policy_id: Uuid,
        policy_revision_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1
                FROM policy_revisions
                WHERE id = $1 AND policy_id = $2
            )",
        )
        .bind(policy_revision_id)
        .bind(policy_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn issue_bootstrap_token(
        &self,
        token_hash: &str,
        policy_id: Uuid,
        policy_revision_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> Result<IssuedBootstrapToken, sqlx::Error> {
        let created_at = Utc::now();
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO enrollment_tokens (
                id,
                token_hash,
                policy_id,
                policy_revision_id,
                expires_at,
                used_at,
                created_at,
                revoked_at
            )
            VALUES ($1, $2, $3, $4, $5, NULL, $6, NULL)",
        )
        .bind(id)
        .bind(token_hash)
        .bind(policy_id)
        .bind(policy_revision_id)
        .bind(expires_at)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(IssuedBootstrapToken {
            id,
            policy_id,
            policy_revision_id,
            expires_at,
            created_at,
        })
    }

    pub async fn agent_exists(&self, agent_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1
                FROM agents
                WHERE agent_id = $1
            )",
        )
        .bind(agent_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn create_or_update_enrollment(
        &self,
        agent_id: &str,
        hostname: &str,
        version: Option<&str>,
        metadata_json: &Value,
        policy: &PolicySnapshot,
        seen_at: DateTime<Utc>,
    ) -> Result<AgentRecord, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let agent = sqlx::query_as::<_, AgentRecord>(
            "INSERT INTO agents (
                id,
                agent_id,
                hostname,
                status,
                version,
                metadata_json,
                first_seen_at,
                last_seen_at,
                created_at,
                updated_at
             ) VALUES ($1, $2, $3, 'enrolled', $4, $5, $6, $6, $6, $6)
             ON CONFLICT (agent_id) DO UPDATE
             SET hostname = EXCLUDED.hostname,
                 status = 'enrolled',
                 version = COALESCE(NULLIF(EXCLUDED.version, ''), agents.version),
                 metadata_json = CASE
                     WHEN EXCLUDED.metadata_json = '{}'::jsonb THEN agents.metadata_json
                     ELSE agents.metadata_json || EXCLUDED.metadata_json
                 END,
                 last_seen_at = EXCLUDED.last_seen_at,
                 updated_at = EXCLUDED.updated_at
             RETURNING
                agent_id,
                hostname,
                status,
                version,
                metadata_json,
                first_seen_at,
                last_seen_at",
        )
        .bind(Uuid::new_v4())
        .bind(agent_id)
        .bind(hostname)
        .bind(version.unwrap_or_default())
        .bind(metadata_json)
        .bind(seen_at)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO agent_policy_bindings (id, agent_id, policy_id, policy_revision_id, assigned_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(agent_id)
        .bind(policy.policy_id)
        .bind(policy.policy_revision_id)
        .bind(seen_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(agent)
    }

    pub async fn fetch_policy_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Option<PolicySnapshot>, sqlx::Error> {
        sqlx::query_as::<_, PolicySnapshot>(
            "SELECT
                apb.policy_id AS policy_id,
                pr.id AS policy_revision_id,
                pr.revision AS policy_revision,
                pr.body_json AS policy_body_json
             FROM agent_policy_bindings apb
             JOIN policy_revisions pr ON pr.id = apb.policy_revision_id
             WHERE apb.agent_id = $1
             ORDER BY apb.assigned_at DESC
             LIMIT 1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn record_heartbeat(
        &self,
        agent_id: &str,
        hostname: Option<&str>,
        version: Option<&str>,
        status: Option<&str>,
        metadata_json: &Value,
        seen_at: DateTime<Utc>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE agents
             SET hostname = COALESCE(NULLIF($2, ''), hostname),
                 version = COALESCE(NULLIF($3, ''), version),
                 status = COALESCE(NULLIF($4, ''), status),
                 metadata_json = CASE
                     WHEN $5::jsonb = '{}'::jsonb THEN metadata_json
                     ELSE metadata_json || $5::jsonb
                 END,
                 last_seen_at = $6,
                 updated_at = $6
             WHERE agent_id = $1",
        )
        .bind(agent_id)
        .bind(hostname)
        .bind(version)
        .bind(status)
        .bind(metadata_json)
        .bind(seen_at)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn insert_diagnostics(
        &self,
        agent_id: &str,
        payload_json: &Value,
        created_at: DateTime<Utc>,
    ) -> Result<bool, sqlx::Error> {
        if !self.agent_exists(agent_id).await? {
            return Ok(false);
        }

        sqlx::query(
            "INSERT INTO agent_diagnostics (id, agent_id, payload_json, created_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(agent_id)
        .bind(payload_json)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(true)
    }

    pub async fn list_agents(&self) -> Result<Vec<AgentRecord>, sqlx::Error> {
        sqlx::query_as::<_, AgentRecord>(
            "SELECT
                agent_id,
                hostname,
                status,
                version,
                metadata_json,
                first_seen_at,
                last_seen_at
             FROM agents
             ORDER BY last_seen_at DESC, agent_id ASC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_agent(&self, agent_id: &str) -> Result<Option<AgentRecord>, sqlx::Error> {
        sqlx::query_as::<_, AgentRecord>(
            "SELECT
                agent_id,
                hostname,
                status,
                version,
                metadata_json,
                first_seen_at,
                last_seen_at
             FROM agents
             WHERE agent_id = $1
             LIMIT 1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn latest_diagnostics(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDiagnosticsRecord>, sqlx::Error> {
        sqlx::query_as::<_, AgentDiagnosticsRecord>(
            "SELECT
                agent_id,
                payload_json,
                created_at
             FROM agent_diagnostics
             WHERE agent_id = $1
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_policies(&self) -> Result<Vec<PolicyRecord>, sqlx::Error> {
        sqlx::query_as::<_, PolicyRecord>(
            "SELECT
                p.id AS policy_id,
                p.name,
                p.description,
                p.is_active,
                p.created_at,
                p.updated_at,
                pr.revision AS latest_revision,
                pr.body_json AS latest_body_json
             FROM policies p
             LEFT JOIN LATERAL (
                SELECT revision, body_json
                FROM policy_revisions
                WHERE policy_id = p.id
                ORDER BY created_at DESC
                LIMIT 1
             ) pr ON TRUE
             ORDER BY p.updated_at DESC, p.name ASC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_policy(&self, policy_id: Uuid) -> Result<Option<PolicyRecord>, sqlx::Error> {
        sqlx::query_as::<_, PolicyRecord>(
            "SELECT
                p.id AS policy_id,
                p.name,
                p.description,
                p.is_active,
                p.created_at,
                p.updated_at,
                pr.revision AS latest_revision,
                pr.body_json AS latest_body_json
             FROM policies p
             LEFT JOIN LATERAL (
                SELECT revision, body_json
                FROM policy_revisions
                WHERE policy_id = p.id
                ORDER BY created_at DESC
                LIMIT 1
             ) pr ON TRUE
             WHERE p.id = $1
             LIMIT 1",
        )
        .bind(policy_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_policy_revisions(
        &self,
        policy_id: Uuid,
    ) -> Result<Vec<PolicyRevisionRecord>, sqlx::Error> {
        sqlx::query_as::<_, PolicyRevisionRecord>(
            "SELECT
                id AS policy_revision_id,
                revision,
                body_json,
                created_at
             FROM policy_revisions
             WHERE policy_id = $1
             ORDER BY created_at DESC, revision DESC",
        )
        .bind(policy_id)
        .fetch_all(&self.pool)
        .await
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AgentRecord {
    pub agent_id: String,
    pub hostname: String,
    pub status: String,
    pub version: Option<String>,
    pub metadata_json: Value,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AgentDiagnosticsRecord {
    pub agent_id: String,
    pub payload_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PolicySnapshot {
    pub policy_id: Uuid,
    pub policy_revision_id: Uuid,
    pub policy_revision: String,
    pub policy_body_json: Value,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PolicyRecord {
    pub policy_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub latest_revision: Option<String>,
    pub latest_body_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PolicyRevisionRecord {
    pub policy_revision_id: Uuid,
    pub revision: String,
    pub body_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct IssuedBootstrapToken {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub policy_revision_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
