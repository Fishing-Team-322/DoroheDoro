use std::collections::HashMap;

use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{postgres::PgRow, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    models::{
        AnomalyInstanceModel, AnomalyRuleModel, ClusterAgentBindingModel, ClusterDetailsModel,
        ClusterHostBindingModel, ClusterModel, CredentialProfileModel, HostGroupMemberModel,
        HostGroupModel, HostModel, IntegrationBindingModel, IntegrationModel, PermissionModel,
        PolicyModel, PolicyRevisionModel, RoleBindingModel, RoleModel, TicketCommentModel,
        TicketDetailsModel, TicketEventModel, TicketModel,
    },
    service::AuditInfo,
};

#[derive(Clone)]
pub struct ControlRepository {
    pool: PgPool,
}

impl ControlRepository {
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

    pub async fn list_policies(&self) -> Result<Vec<PolicyModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT
                p.id,
                p.name,
                p.description,
                p.is_active,
                p.created_at,
                p.updated_at,
                p.created_by,
                p.updated_by,
                p.update_reason,
                pr.id AS latest_revision_id,
                pr.revision AS latest_revision,
                pr.body_json AS policy_body_json
             FROM policies p
             JOIN LATERAL (
                SELECT id, revision, body_json
                FROM policy_revisions
                WHERE policy_id = p.id
                ORDER BY created_at DESC
                LIMIT 1
             ) pr ON TRUE
             ORDER BY p.name ASC, p.id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(policy_from_row).collect())
    }

    pub async fn get_policy(&self, policy_id: Uuid) -> Result<Option<PolicyModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT
                p.id,
                p.name,
                p.description,
                p.is_active,
                p.created_at,
                p.updated_at,
                p.created_by,
                p.updated_by,
                p.update_reason,
                pr.id AS latest_revision_id,
                pr.revision AS latest_revision,
                pr.body_json AS policy_body_json
             FROM policies p
             JOIN LATERAL (
                SELECT id, revision, body_json
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
        .await?;

        Ok(row.map(policy_from_row))
    }

    pub async fn create_policy(
        &self,
        name: &str,
        description: &str,
        body_json: &Value,
        audit: &AuditInfo,
    ) -> Result<PolicyModel, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let policy_id = Uuid::new_v4();
        let revision_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO policies (
                id, name, description, is_active, created_at, updated_at,
                created_by, updated_by, request_id, update_reason
            )
            VALUES ($1, $2, $3, TRUE, $4, $4, $5, $5, $6, $7)",
        )
        .bind(policy_id)
        .bind(name)
        .bind(description)
        .bind(now)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO policy_revisions (
                id, policy_id, revision, body_json, created_at,
                created_by, request_id, reason
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(revision_id)
        .bind(policy_id)
        .bind("rev-1")
        .bind(body_json)
        .bind(now)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .execute(&mut *tx)
        .await?;

        self.record_audit_event(
            &mut tx,
            "policy",
            "policy.create",
            policy_id,
            audit,
            json!({ "name": name, "policy_revision_id": revision_id }),
        )
        .await?;

        tx.commit().await?;
        self.get_policy(policy_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn update_policy(
        &self,
        policy_id: Uuid,
        description: &str,
        body_json: &Value,
        audit: &AuditInfo,
    ) -> Result<Option<PolicyModel>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE policies
             SET description = $2,
                 updated_at = $3,
                 updated_by = $4,
                 request_id = $5,
                 update_reason = $6
             WHERE id = $1",
        )
        .bind(policy_id)
        .bind(description)
        .bind(now)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(None);
        }

        let revision_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM policy_revisions WHERE policy_id = $1")
                .bind(policy_id)
                .fetch_one(&mut *tx)
                .await?;
        let revision_label = format!("rev-{}", revision_count + 1);
        let revision_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO policy_revisions (
                id, policy_id, revision, body_json, created_at,
                created_by, request_id, reason
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(revision_id)
        .bind(policy_id)
        .bind(&revision_label)
        .bind(body_json)
        .bind(now)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .execute(&mut *tx)
        .await?;

        self.record_audit_event(
            &mut tx,
            "policy",
            "policy.update",
            policy_id,
            audit,
            json!({ "policy_revision_id": revision_id, "revision": revision_label }),
        )
        .await?;

        tx.commit().await?;
        self.get_policy(policy_id).await
    }

    pub async fn list_policy_revisions(
        &self,
        policy_id: Uuid,
    ) -> Result<Vec<PolicyRevisionModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, policy_id, revision, body_json, created_at, created_by, reason, request_id
             FROM policy_revisions
             WHERE policy_id = $1
             ORDER BY created_at DESC, revision DESC",
        )
        .bind(policy_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(policy_revision_from_row).collect())
    }

    pub async fn list_hosts(&self) -> Result<Vec<HostModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at,
                    created_by, updated_by, update_reason
             FROM hosts
             ORDER BY hostname ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(host_from_row).collect())
    }

    pub async fn get_host(&self, host_id: Uuid) -> Result<Option<HostModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at,
                    created_by, updated_by, update_reason
             FROM hosts WHERE id = $1 LIMIT 1",
        )
        .bind(host_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(host_from_row))
    }

    pub async fn create_host(
        &self,
        hostname: &str,
        ip: &str,
        ssh_port: i32,
        remote_user: &str,
        labels_json: &Value,
        audit: &AuditInfo,
    ) -> Result<HostModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO hosts (
                id, hostname, ip, ssh_port, remote_user, labels_json,
                created_at, updated_at, created_by, updated_by, request_id, update_reason
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), $7, $7, $8, $9)
            RETURNING id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at,
                      created_by, updated_by, update_reason",
        )
        .bind(Uuid::new_v4())
        .bind(hostname)
        .bind(ip)
        .bind(ssh_port)
        .bind(remote_user)
        .bind(labels_json)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .fetch_one(&self.pool)
        .await?;

        let host = host_from_row(row);
        self.record_audit_event_direct(
            "host",
            "host.create",
            host.id,
            audit,
            json!({ "hostname": host.hostname, "ip": host.ip }),
        )
        .await?;
        Ok(host)
    }

    pub async fn update_host(
        &self,
        host_id: Uuid,
        hostname: &str,
        ip: &str,
        ssh_port: i32,
        remote_user: &str,
        labels_json: &Value,
        audit: &AuditInfo,
    ) -> Result<Option<HostModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE hosts
             SET hostname = $2,
                 ip = $3,
                 ssh_port = $4,
                 remote_user = $5,
                 labels_json = $6,
                 updated_at = NOW(),
                 updated_by = $7,
                 request_id = $8,
                 update_reason = $9
             WHERE id = $1
             RETURNING id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at,
                       created_by, updated_by, update_reason",
        )
        .bind(host_id)
        .bind(hostname)
        .bind(ip)
        .bind(ssh_port)
        .bind(remote_user)
        .bind(labels_json)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let host = host_from_row(row);
        self.record_audit_event_direct(
            "host",
            "host.update",
            host.id,
            audit,
            json!({ "hostname": host.hostname, "ip": host.ip }),
        )
        .await?;
        Ok(Some(host))
    }

    pub async fn list_host_groups(&self) -> Result<Vec<HostGroupModel>, sqlx::Error> {
        let group_rows = sqlx::query(
            "SELECT id, name, description, created_at, updated_at, created_by, updated_by, update_reason
             FROM host_groups
             ORDER BY name ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut groups: HashMap<Uuid, HostGroupModel> = group_rows
            .into_iter()
            .map(|row| {
                let id: Uuid = row.get("id");
                (
                    id,
                    HostGroupModel {
                        id,
                        name: row.get("name"),
                        description: row.get("description"),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                        created_by: row.get("created_by"),
                        updated_by: row.get("updated_by"),
                        update_reason: row.get("update_reason"),
                        members: Vec::new(),
                    },
                )
            })
            .collect();

        if groups.is_empty() {
            return Ok(Vec::new());
        }

        let member_rows = sqlx::query(
            "SELECT hgm.id,
                    hgm.host_group_id,
                    hgm.host_id,
                    hosts.hostname
             FROM host_group_members hgm
             LEFT JOIN hosts ON hosts.id = hgm.host_id",
        )
        .fetch_all(&self.pool)
        .await?;

        for row in member_rows {
            let host_group_id: Uuid = row.get("host_group_id");
            if let Some(group) = groups.get_mut(&host_group_id) {
                group.members.push(HostGroupMemberModel {
                    id: row.get("id"),
                    host_group_id,
                    host_id: row.get("host_id"),
                    hostname: row.get::<Option<String>, _>("hostname"),
                });
            }
        }

        let mut groups = groups.into_values().collect::<Vec<_>>();
        groups.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(groups)
    }

    pub async fn get_host_group(
        &self,
        host_group_id: Uuid,
    ) -> Result<Option<HostGroupModel>, sqlx::Error> {
        let group_row = sqlx::query(
            "SELECT id, name, description, created_at, updated_at, created_by, updated_by, update_reason
             FROM host_groups WHERE id = $1",
        )
        .bind(host_group_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = group_row else {
            return Ok(None);
        };

        let mut group = HostGroupModel {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            created_by: row.get("created_by"),
            updated_by: row.get("updated_by"),
            update_reason: row.get("update_reason"),
            members: Vec::new(),
        };

        let member_rows = sqlx::query(
            "SELECT hgm.id,
                    hgm.host_group_id,
                    hgm.host_id,
                    hosts.hostname
             FROM host_group_members hgm
             LEFT JOIN hosts ON hosts.id = hgm.host_id
             WHERE hgm.host_group_id = $1",
        )
        .bind(host_group_id)
        .fetch_all(&self.pool)
        .await?;

        for row in member_rows {
            group.members.push(HostGroupMemberModel {
                id: row.get("id"),
                host_group_id,
                host_id: row.get("host_id"),
                hostname: row.get::<Option<String>, _>("hostname"),
            });
        }

        Ok(Some(group))
    }

    pub async fn create_host_group(
        &self,
        name: &str,
        description: &str,
        audit: &AuditInfo,
    ) -> Result<HostGroupModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO host_groups (
                id, name, description, created_at, updated_at,
                created_by, updated_by, request_id, update_reason
            )
            VALUES ($1, $2, $3, NOW(), NOW(), $4, $4, $5, $6)
            RETURNING id, name, description, created_at, updated_at, created_by, updated_by, update_reason",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(description)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .fetch_one(&self.pool)
        .await?;

        let group = HostGroupModel {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            created_by: row.get("created_by"),
            updated_by: row.get("updated_by"),
            update_reason: row.get("update_reason"),
            members: Vec::new(),
        };

        self.record_audit_event_direct(
            "host_group",
            "host_group.create",
            group.id,
            audit,
            json!({ "name": group.name }),
        )
        .await?;

        Ok(group)
    }

    pub async fn update_host_group(
        &self,
        host_group_id: Uuid,
        name: &str,
        description: &str,
        audit: &AuditInfo,
    ) -> Result<Option<HostGroupModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE host_groups
             SET name = $2,
                 description = $3,
                 updated_at = NOW(),
                 updated_by = $4,
                 request_id = $5,
                 update_reason = $6
             WHERE id = $1
             RETURNING id, name, description, created_at, updated_at, created_by, updated_by, update_reason",
        )
        .bind(host_group_id)
        .bind(name)
        .bind(description)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .fetch_optional(&self.pool)
        .await?;

        let Some(_row) = row else {
            return Ok(None);
        };

        self.record_audit_event_direct(
            "host_group",
            "host_group.update",
            host_group_id,
            audit,
            json!({ "name": name }),
        )
        .await?;

        self.get_host_group(host_group_id).await
    }

    pub async fn add_host_group_member(
        &self,
        host_group_id: Uuid,
        host_id: Uuid,
        audit: &AuditInfo,
    ) -> Result<HostGroupMemberModel, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query(
            "WITH inserted AS (
                 INSERT INTO host_group_members (id, host_group_id, host_id, created_at)
                 VALUES ($1, $2, $3, NOW())
                 RETURNING id, host_group_id, host_id
             )
             SELECT inserted.id,
                    inserted.host_group_id,
                    inserted.host_id,
                    hosts.hostname
             FROM inserted
             LEFT JOIN hosts ON hosts.id = inserted.host_id",
        )
        .bind(Uuid::new_v4())
        .bind(host_group_id)
        .bind(host_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE host_groups
             SET updated_at = NOW(),
                 updated_by = $2,
                 request_id = $3,
                 update_reason = $4
             WHERE id = $1",
        )
        .bind(host_group_id)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .execute(&mut *tx)
        .await?;

        self.record_audit_event(
            &mut tx,
            "host_group",
            "host_group.member.add",
            host_group_id,
            audit,
            json!({ "host_id": host_id }),
        )
        .await?;

        tx.commit().await?;

        Ok(HostGroupMemberModel {
            id: row.get("id"),
            host_group_id: row.get("host_group_id"),
            host_id: row.get("host_id"),
            hostname: row.get("hostname"),
        })
    }

    pub async fn remove_host_group_member(
        &self,
        host_group_id: Uuid,
        host_id: Uuid,
        audit: &AuditInfo,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result =
            sqlx::query("DELETE FROM host_group_members WHERE host_group_id = $1 AND host_id = $2")
                .bind(host_group_id)
                .bind(host_id)
                .execute(&mut *tx)
                .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query(
            "UPDATE host_groups
             SET updated_at = NOW(),
                 updated_by = $2,
                 request_id = $3,
                 update_reason = $4
             WHERE id = $1",
        )
        .bind(host_group_id)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .execute(&mut *tx)
        .await?;

        self.record_audit_event(
            &mut tx,
            "host_group",
            "host_group.member.remove",
            host_group_id,
            audit,
            json!({ "host_id": host_id }),
        )
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn list_credentials(&self) -> Result<Vec<CredentialProfileModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, name, kind, description, vault_ref, created_at, updated_at,
                    created_by, updated_by, update_reason
             FROM credentials_profiles_metadata
             ORDER BY name ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(credential_from_row).collect())
    }

    pub async fn get_credentials(
        &self,
        credentials_id: Uuid,
    ) -> Result<Option<CredentialProfileModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, kind, description, vault_ref, created_at, updated_at,
                    created_by, updated_by, update_reason
             FROM credentials_profiles_metadata
             WHERE id = $1",
        )
        .bind(credentials_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(credential_from_row))
    }

    pub async fn create_credentials(
        &self,
        name: &str,
        kind: &str,
        description: &str,
        vault_ref: &str,
        audit: &AuditInfo,
    ) -> Result<CredentialProfileModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO credentials_profiles_metadata (
                id, name, kind, description, vault_ref, created_at, updated_at,
                created_by, updated_by, request_id, update_reason
            )
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW(), $6, $6, $7, $8)
             RETURNING id, name, kind, description, vault_ref, created_at, updated_at,
                       created_by, updated_by, update_reason",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(kind)
        .bind(description)
        .bind(vault_ref)
        .bind(&audit.actor_id)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .fetch_one(&self.pool)
        .await?;

        let profile = credential_from_row(row);
        self.record_audit_event_direct(
            "credentials_profile",
            "credentials.create",
            profile.id,
            audit,
            json!({ "name": profile.name, "kind": profile.kind, "vault_ref": profile.vault_ref }),
        )
        .await?;
        Ok(profile)
    }

    pub async fn list_clusters(
        &self,
        host_filter: Option<Uuid>,
    ) -> Result<Vec<ClusterModel>, sqlx::Error> {
        const BASE_SELECT: &str = "
            SELECT
                c.id,
                c.name,
                c.slug,
                c.description,
                c.is_active,
                c.created_at,
                c.updated_at,
                c.created_by,
                c.updated_by,
                COALESCE(cm.metadata_json, '{}'::jsonb) AS metadata_json,
                (SELECT COUNT(*) FROM cluster_hosts ch WHERE ch.cluster_id = c.id) AS host_count,
                (SELECT COUNT(*) FROM cluster_agents ca WHERE ca.cluster_id = c.id) AS agent_count
            FROM clusters c
            LEFT JOIN cluster_metadata cm ON cm.cluster_id = c.id
        ";

        let rows = if let Some(host_id) = host_filter {
            sqlx::query(&format!(
                "{BASE_SELECT}
                 WHERE c.id IN (
                    SELECT cluster_id FROM cluster_hosts WHERE host_id = $1
                 )
                 ORDER BY c.name ASC, c.id ASC"
            ))
            .bind(host_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(&format!(
                "{BASE_SELECT}
                 ORDER BY c.name ASC, c.id ASC"
            ))
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(cluster_from_row).collect())
    }

    pub async fn get_cluster_details(
        &self,
        cluster_id: Uuid,
    ) -> Result<Option<ClusterDetailsModel>, sqlx::Error> {
        let Some(cluster) = self.get_cluster_model(cluster_id).await? else {
            return Ok(None);
        };
        let hosts = self.list_cluster_hosts(cluster_id).await?;
        let agents = self.list_cluster_agents(cluster_id).await?;
        Ok(Some(ClusterDetailsModel {
            cluster,
            hosts,
            agents,
        }))
    }

    pub async fn create_cluster(
        &self,
        name: &str,
        slug: &str,
        description: &str,
        is_active: bool,
        metadata_json: &Value,
        audit: &AuditInfo,
    ) -> Result<ClusterDetailsModel, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let cluster_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO clusters (
                id, name, slug, description, is_active,
                created_at, updated_at, created_by, updated_by
            )
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW(), $6, $6)",
        )
        .bind(cluster_id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(is_active)
        .bind(&audit.actor_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO cluster_metadata (cluster_id, metadata_json, updated_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (cluster_id)
             DO UPDATE SET metadata_json = EXCLUDED.metadata_json, updated_at = NOW()",
        )
        .bind(cluster_id)
        .bind(metadata_json)
        .execute(&mut *tx)
        .await?;

        self.record_audit_event(
            &mut tx,
            "cluster",
            "cluster.create",
            cluster_id,
            audit,
            json!({ "slug": slug }),
        )
        .await?;

        tx.commit().await?;

        self.get_cluster_details(cluster_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn update_cluster(
        &self,
        cluster_id: Uuid,
        name: &str,
        slug: &str,
        description: &str,
        is_active: bool,
        metadata_json: &Value,
        audit: &AuditInfo,
    ) -> Result<Option<ClusterDetailsModel>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE clusters
             SET name = $2,
                 slug = $3,
                 description = $4,
                 is_active = $5,
                 updated_at = NOW(),
                 updated_by = $6
             WHERE id = $1",
        )
        .bind(cluster_id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(is_active)
        .bind(&audit.actor_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(None);
        }

        sqlx::query(
            "INSERT INTO cluster_metadata (cluster_id, metadata_json, updated_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (cluster_id)
             DO UPDATE SET metadata_json = EXCLUDED.metadata_json, updated_at = NOW()",
        )
        .bind(cluster_id)
        .bind(metadata_json)
        .execute(&mut *tx)
        .await?;

        self.record_audit_event(
            &mut tx,
            "cluster",
            "cluster.update",
            cluster_id,
            audit,
            json!({ "slug": slug }),
        )
        .await?;

        tx.commit().await?;
        self.get_cluster_details(cluster_id).await
    }

    pub async fn add_host_to_cluster(
        &self,
        cluster_id: Uuid,
        host_id: Uuid,
        audit: &AuditInfo,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "INSERT INTO cluster_hosts (id, cluster_id, host_id, created_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (cluster_id, host_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(cluster_id)
        .bind(host_id)
        .execute(&mut *tx)
        .await?;

        let inserted = result.rows_affected() > 0;
        if inserted {
            self.record_audit_event(
                &mut tx,
                "cluster",
                "cluster.host.add",
                cluster_id,
                audit,
                json!({ "host_id": host_id }),
            )
            .await?;
        }

        tx.commit().await?;
        Ok(inserted)
    }

    pub async fn remove_host_from_cluster(
        &self,
        cluster_id: Uuid,
        host_id: Uuid,
        audit: &AuditInfo,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result =
            sqlx::query("DELETE FROM cluster_hosts WHERE cluster_id = $1 AND host_id = $2")
                .bind(cluster_id)
                .bind(host_id)
                .execute(&mut *tx)
                .await?;

        let removed = result.rows_affected() > 0;
        if removed {
            self.record_audit_event(
                &mut tx,
                "cluster",
                "cluster.host.remove",
                cluster_id,
                audit,
                json!({ "host_id": host_id }),
            )
            .await?;
        }

        tx.commit().await?;
        Ok(removed)
    }

    pub async fn list_cluster_hosts(
        &self,
        cluster_id: Uuid,
    ) -> Result<Vec<ClusterHostBindingModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT ch.id,
                    ch.cluster_id,
                    ch.host_id,
                    ch.created_at,
                    hosts.hostname
             FROM cluster_hosts ch
             LEFT JOIN hosts ON hosts.id = ch.host_id
             WHERE ch.cluster_id = $1
             ORDER BY hosts.hostname ASC NULLS LAST, ch.created_at ASC",
        )
        .bind(cluster_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(cluster_host_from_row).collect())
    }

    pub async fn list_cluster_agents(
        &self,
        cluster_id: Uuid,
    ) -> Result<Vec<ClusterAgentBindingModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT ca.id,
                    ca.cluster_id,
                    ca.agent_id,
                    ca.created_at
             FROM cluster_agents ca
             WHERE ca.cluster_id = $1
             ORDER BY ca.created_at ASC",
        )
        .bind(cluster_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(cluster_agent_from_row).collect())
    }

    async fn get_cluster_model(
        &self,
        cluster_id: Uuid,
    ) -> Result<Option<ClusterModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT
                c.id,
                c.name,
                c.slug,
                c.description,
                c.is_active,
                c.created_at,
                c.updated_at,
                c.created_by,
                c.updated_by,
                COALESCE(cm.metadata_json, '{}'::jsonb) AS metadata_json,
                (SELECT COUNT(*) FROM cluster_hosts ch WHERE ch.cluster_id = c.id) AS host_count,
                (SELECT COUNT(*) FROM cluster_agents ca WHERE ca.cluster_id = c.id) AS agent_count
             FROM clusters c
             LEFT JOIN cluster_metadata cm ON cm.cluster_id = c.id
             WHERE c.id = $1
             LIMIT 1",
        )
        .bind(cluster_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(cluster_from_row))
    }

    pub async fn list_roles(&self) -> Result<Vec<RoleModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, name, slug, description, is_system,
                    created_at, updated_at, created_by, updated_by
             FROM roles
             ORDER BY name ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(role_from_row).collect())
    }

    pub async fn get_role(&self, role_id: Uuid) -> Result<Option<RoleModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, slug, description, is_system,
                    created_at, updated_at, created_by, updated_by
             FROM roles
             WHERE id = $1",
        )
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(role_from_row))
    }

    pub async fn create_role(
        &self,
        name: &str,
        slug: &str,
        description: &str,
        audit: &AuditInfo,
    ) -> Result<RoleModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO roles (
                id, name, slug, description, is_system,
                created_at, updated_at, created_by, updated_by
            )
            VALUES ($1, $2, $3, $4, FALSE, NOW(), NOW(), $5, $5)
            RETURNING id, name, slug, description, is_system,
                      created_at, updated_at, created_by, updated_by",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(&audit.actor_id)
        .fetch_one(&self.pool)
        .await?;

        let role = role_from_row(row);
        self.record_audit_event_direct(
            "role",
            "role.create",
            role.id,
            audit,
            json!({ "slug": role.slug }),
        )
        .await?;
        Ok(role)
    }

    pub async fn update_role(
        &self,
        role_id: Uuid,
        name: &str,
        description: &str,
        audit: &AuditInfo,
    ) -> Result<Option<RoleModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE roles
             SET name = $2,
                 description = $3,
                 updated_at = NOW(),
                 updated_by = $4
             WHERE id = $1
             RETURNING id, name, slug, description, is_system,
                       created_at, updated_at, created_by, updated_by",
        )
        .bind(role_id)
        .bind(name)
        .bind(description)
        .bind(&audit.actor_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let role = role_from_row(row);
        self.record_audit_event_direct(
            "role",
            "role.update",
            role_id,
            audit,
            json!({ "name": name }),
        )
        .await?;
        Ok(Some(role))
    }

    pub async fn ensure_permission_catalog(
        &self,
        definitions: &[PermissionDefinition<'_>],
    ) -> Result<(), sqlx::Error> {
        for definition in definitions {
            sqlx::query(
                "INSERT INTO permissions (id, code, description)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (code)
                 DO UPDATE SET description = EXCLUDED.description",
            )
            .bind(Uuid::new_v4())
            .bind(definition.code)
            .bind(definition.description)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn list_permissions(&self) -> Result<Vec<PermissionModel>, sqlx::Error> {
        let rows = sqlx::query("SELECT id, code, description FROM permissions ORDER BY code ASC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(permission_from_row).collect())
    }

    pub async fn permissions_by_codes(
        &self,
        codes: &[String],
    ) -> Result<Vec<PermissionModel>, sqlx::Error> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let code_params: Vec<String> = codes.iter().cloned().collect();
        let rows = sqlx::query(
            "SELECT id, code, description
             FROM permissions
             WHERE code = ANY($1)",
        )
        .bind(&code_params)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(permission_from_row).collect())
    }

    pub async fn get_role_permissions(
        &self,
        role_id: Uuid,
    ) -> Result<Vec<PermissionModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT p.id, p.code, p.description
             FROM role_permissions rp
             JOIN permissions p ON p.id = rp.permission_id
             WHERE rp.role_id = $1
             ORDER BY p.code ASC",
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(permission_from_row).collect())
    }

    pub async fn set_role_permissions(
        &self,
        role_id: Uuid,
        permission_ids: &[Uuid],
        audit: &AuditInfo,
    ) -> Result<Vec<PermissionModel>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
            .bind(role_id)
            .execute(&mut *tx)
            .await?;

        for permission_id in permission_ids {
            sqlx::query(
                "INSERT INTO role_permissions (id, role_id, permission_id, created_at)
                 VALUES ($1, $2, $3, NOW())",
            )
            .bind(Uuid::new_v4())
            .bind(role_id)
            .bind(permission_id)
            .execute(&mut *tx)
            .await?;
        }

        self.record_audit_event(
            &mut tx,
            "role",
            "role.permissions.update",
            role_id,
            audit,
            json!({ "permission_count": permission_ids.len() }),
        )
        .await?;

        tx.commit().await?;
        self.get_role_permissions(role_id).await
    }

    pub async fn list_role_bindings(&self) -> Result<Vec<RoleBindingModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, user_id, role_id, scope_type, scope_id, created_at
             FROM user_role_bindings
             ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(role_binding_from_row).collect())
    }

    pub async fn create_role_binding(
        &self,
        user_id: &str,
        role_id: Uuid,
        scope_type: &str,
        scope_id: Option<Uuid>,
        audit: &AuditInfo,
    ) -> Result<RoleBindingModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO user_role_bindings (
                id, user_id, role_id, scope_type, scope_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, NOW())
            RETURNING id, user_id, role_id, scope_type, scope_id, created_at",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(role_id)
        .bind(scope_type)
        .bind(scope_id)
        .fetch_one(&self.pool)
        .await?;

        let binding = role_binding_from_row(row);
        self.record_audit_event_direct(
            "role_binding",
            "role.binding.create",
            binding.id,
            audit,
            json!({ "user_id": binding.user_id, "role_id": binding.role_id }),
        )
        .await?;
        Ok(binding)
    }

    pub async fn delete_role_binding(
        &self,
        role_binding_id: Uuid,
        audit: &AuditInfo,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM user_role_bindings WHERE id = $1")
            .bind(role_binding_id)
            .execute(&self.pool)
            .await?;
        let deleted = result.rows_affected() > 0;
        if deleted {
            self.record_audit_event_direct(
                "role_binding",
                "role.binding.delete",
                role_binding_id,
                audit,
                Value::Null,
            )
            .await?;
        }
        Ok(deleted)
    }

    pub async fn list_integrations(&self) -> Result<Vec<IntegrationModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, name, kind, description, config_json, is_active,
                    created_at, updated_at, created_by, updated_by
             FROM integrations
             ORDER BY name ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(integration_from_row).collect())
    }

    pub async fn get_integration(
        &self,
        integration_id: Uuid,
    ) -> Result<Option<IntegrationModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, kind, description, config_json, is_active,
                    created_at, updated_at, created_by, updated_by
             FROM integrations
             WHERE id = $1",
        )
        .bind(integration_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(integration_from_row))
    }

    pub async fn create_integration(
        &self,
        name: &str,
        kind: &str,
        description: &str,
        config_json: &Value,
        is_active: bool,
        audit: &AuditInfo,
    ) -> Result<IntegrationModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO integrations (
                id, name, kind, description, config_json, is_active,
                created_at, updated_at, created_by, updated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), $7, $7)
            RETURNING id, name, kind, description, config_json, is_active,
                      created_at, updated_at, created_by, updated_by",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(kind)
        .bind(description)
        .bind(config_json)
        .bind(is_active)
        .bind(&audit.actor_id)
        .fetch_one(&self.pool)
        .await?;

        let integration = integration_from_row(row);
        self.record_audit_event_direct(
            "integration",
            "integration.create",
            integration.id,
            audit,
            json!({ "kind": integration.kind }),
        )
        .await?;
        Ok(integration)
    }

    pub async fn update_integration(
        &self,
        integration_id: Uuid,
        name: &str,
        description: &str,
        config_json: &Value,
        is_active: bool,
        audit: &AuditInfo,
    ) -> Result<Option<IntegrationModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE integrations
             SET name = $2,
                 description = $3,
                 config_json = $4,
                 is_active = $5,
                 updated_at = NOW(),
                 updated_by = $6
             WHERE id = $1
             RETURNING id, name, kind, description, config_json, is_active,
                       created_at, updated_at, created_by, updated_by",
        )
        .bind(integration_id)
        .bind(name)
        .bind(description)
        .bind(config_json)
        .bind(is_active)
        .bind(&audit.actor_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let integration = integration_from_row(row);
        self.record_audit_event_direct(
            "integration",
            "integration.update",
            integration_id,
            audit,
            json!({ "name": name }),
        )
        .await?;
        Ok(Some(integration))
    }

    pub async fn list_integration_bindings(
        &self,
        integration_id: Uuid,
    ) -> Result<Vec<IntegrationBindingModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, integration_id, scope_type, scope_id, event_types_json,
                    severity_threshold, is_active, created_at, updated_at
             FROM integration_bindings
             WHERE integration_id = $1
             ORDER BY created_at DESC",
        )
        .bind(integration_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(integration_binding_from_row).collect())
    }

    pub async fn create_integration_binding(
        &self,
        integration_id: Uuid,
        scope_type: &str,
        scope_id: Option<Uuid>,
        event_types_json: &Value,
        severity_threshold: &str,
        is_active: bool,
        audit: &AuditInfo,
    ) -> Result<IntegrationBindingModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO integration_bindings (
                id, integration_id, scope_type, scope_id, event_types_json,
                severity_threshold, is_active, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            RETURNING id, integration_id, scope_type, scope_id, event_types_json,
                      severity_threshold, is_active, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(integration_id)
        .bind(scope_type)
        .bind(scope_id)
        .bind(event_types_json)
        .bind(severity_threshold)
        .bind(is_active)
        .fetch_one(&self.pool)
        .await?;

        let binding = integration_binding_from_row(row);
        self.record_audit_event_direct(
            "integration_binding",
            "integration.bind",
            binding.id,
            audit,
            json!({ "integration_id": integration_id }),
        )
        .await?;
        Ok(binding)
    }

    pub async fn delete_integration_binding(
        &self,
        integration_binding_id: Uuid,
        audit: &AuditInfo,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM integration_bindings WHERE id = $1")
            .bind(integration_binding_id)
            .execute(&self.pool)
            .await?;
        let deleted = result.rows_affected() > 0;
        if deleted {
            self.record_audit_event_direct(
                "integration_binding",
                "integration.unbind",
                integration_binding_id,
                audit,
                Value::Null,
            )
            .await?;
        }
        Ok(deleted)
    }

    pub async fn list_tickets(&self) -> Result<Vec<TicketModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT t.id,
                    t.ticket_key,
                    t.title,
                    t.description,
                    t.cluster_id,
                    clusters.name AS cluster_name,
                    t.source_type,
                    t.source_id,
                    t.severity,
                    t.status,
                    t.assignee_user_id,
                    t.created_by,
                    t.resolution,
                    t.created_at,
                    t.updated_at,
                    t.resolved_at,
                    t.closed_at
             FROM tickets t
             LEFT JOIN clusters ON clusters.id = t.cluster_id
             ORDER BY t.created_at DESC, t.id DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(ticket_from_row).collect())
    }

    pub async fn get_ticket_details(
        &self,
        ticket_id: Uuid,
    ) -> Result<Option<TicketDetailsModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT t.id,
                    t.ticket_key,
                    t.title,
                    t.description,
                    t.cluster_id,
                    clusters.name AS cluster_name,
                    t.source_type,
                    t.source_id,
                    t.severity,
                    t.status,
                    t.assignee_user_id,
                    t.created_by,
                    t.resolution,
                    t.created_at,
                    t.updated_at,
                    t.resolved_at,
                    t.closed_at
             FROM tickets t
             LEFT JOIN clusters ON clusters.id = t.cluster_id
             WHERE t.id = $1",
        )
        .bind(ticket_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let ticket = ticket_from_row(row);
        let comments = self.list_ticket_comments(ticket_id).await?;
        let events = self.list_ticket_events(ticket_id).await?;
        Ok(Some(TicketDetailsModel {
            ticket,
            comments,
            events,
        }))
    }

    pub async fn create_ticket(
        &self,
        ticket_key: &str,
        title: &str,
        description: &str,
        cluster_id: Uuid,
        source_type: &str,
        source_id: Option<&str>,
        severity: &str,
        status: &str,
        created_by: &str,
        audit: &AuditInfo,
    ) -> Result<TicketDetailsModel, sqlx::Error> {
        let ticket_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tickets (
                id, ticket_key, title, description, cluster_id, source_type, source_id,
                severity, status, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())",
        )
        .bind(ticket_id)
        .bind(ticket_key)
        .bind(title)
        .bind(description)
        .bind(cluster_id)
        .bind(source_type)
        .bind(source_id)
        .bind(severity)
        .bind(status)
        .bind(created_by)
        .execute(&self.pool)
        .await?;

        self.record_audit_event_direct(
            "ticket",
            "ticket.create",
            ticket_id,
            audit,
            json!({ "ticket_key": ticket_key, "severity": severity }),
        )
        .await?;
        self.get_ticket_details(ticket_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn assign_ticket(
        &self,
        ticket_id: Uuid,
        assignee_user_id: &str,
        audit: &AuditInfo,
    ) -> Result<Option<TicketModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE tickets
             SET assignee_user_id = $2,
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, ticket_key, title, description, cluster_id, source_type, source_id,
                       severity, status, assignee_user_id, created_by, resolution,
                       created_at, updated_at, resolved_at, closed_at,
                       (SELECT name FROM clusters WHERE id = cluster_id) AS cluster_name",
        )
        .bind(ticket_id)
        .bind(assignee_user_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let ticket = ticket_from_row(row);
        self.record_audit_event_direct(
            "ticket",
            "ticket.assign",
            ticket_id,
            audit,
            json!({ "assignee_user_id": assignee_user_id }),
        )
        .await?;
        Ok(Some(ticket))
    }

    pub async fn unassign_ticket(
        &self,
        ticket_id: Uuid,
        audit: &AuditInfo,
    ) -> Result<Option<TicketModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE tickets
             SET assignee_user_id = NULL,
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, ticket_key, title, description, cluster_id, source_type, source_id,
                       severity, status, assignee_user_id, created_by, resolution,
                       created_at, updated_at, resolved_at, closed_at,
                       (SELECT name FROM clusters WHERE id = cluster_id) AS cluster_name",
        )
        .bind(ticket_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let ticket = ticket_from_row(row);
        self.record_audit_event_direct("ticket", "ticket.unassign", ticket_id, audit, Value::Null)
            .await?;
        Ok(Some(ticket))
    }

    pub async fn add_ticket_comment(
        &self,
        ticket_id: Uuid,
        author_user_id: &str,
        body: &str,
        audit: &AuditInfo,
    ) -> Result<TicketCommentModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO ticket_comments (id, ticket_id, author_user_id, body, created_at)
             VALUES ($1, $2, $3, $4, NOW())
             RETURNING id, ticket_id, author_user_id, body, created_at",
        )
        .bind(Uuid::new_v4())
        .bind(ticket_id)
        .bind(author_user_id)
        .bind(body)
        .fetch_one(&self.pool)
        .await?;

        let comment = ticket_comment_from_row(row);
        self.record_audit_event_direct(
            "ticket",
            "ticket.comment.add",
            ticket_id,
            audit,
            json!({ "comment_id": comment.id }),
        )
        .await?;
        Ok(comment)
    }

    pub async fn add_ticket_event(
        &self,
        ticket_id: Uuid,
        event_type: &str,
        payload_json: &Value,
    ) -> Result<TicketEventModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO ticket_events (id, ticket_id, event_type, payload_json, created_at)
             VALUES ($1, $2, $3, $4, NOW())
             RETURNING id, ticket_id, event_type, payload_json, created_at",
        )
        .bind(Uuid::new_v4())
        .bind(ticket_id)
        .bind(event_type)
        .bind(payload_json)
        .fetch_one(&self.pool)
        .await?;
        Ok(ticket_event_from_row(row))
    }

    pub async fn change_ticket_status(
        &self,
        ticket_id: Uuid,
        status: &str,
        resolution: &str,
        audit: &AuditInfo,
    ) -> Result<Option<TicketModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE tickets
             SET status = $2,
                 resolution = $3,
                 updated_at = NOW(),
                 resolved_at = CASE WHEN $2 = 'resolved' THEN NOW() ELSE resolved_at END
             WHERE id = $1
             RETURNING id, ticket_key, title, description, cluster_id, source_type, source_id,
                       severity, status, assignee_user_id, created_by, resolution,
                       created_at, updated_at, resolved_at, closed_at,
                       (SELECT name FROM clusters WHERE id = cluster_id) AS cluster_name",
        )
        .bind(ticket_id)
        .bind(status)
        .bind(resolution)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let ticket = ticket_from_row(row);
        self.record_audit_event_direct(
            "ticket",
            "ticket.status.change",
            ticket_id,
            audit,
            json!({ "status": status }),
        )
        .await?;
        Ok(Some(ticket))
    }

    pub async fn close_ticket(
        &self,
        ticket_id: Uuid,
        resolution: &str,
        audit: &AuditInfo,
    ) -> Result<Option<TicketModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE tickets
             SET status = 'closed',
                 resolution = $2,
                 closed_at = NOW(),
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, ticket_key, title, description, cluster_id, source_type, source_id,
                       severity, status, assignee_user_id, created_by, resolution,
                       created_at, updated_at, resolved_at, closed_at,
                       (SELECT name FROM clusters WHERE id = cluster_id) AS cluster_name",
        )
        .bind(ticket_id)
        .bind(resolution)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let ticket = ticket_from_row(row);
        self.record_audit_event_direct(
            "ticket",
            "ticket.close",
            ticket_id,
            audit,
            json!({ "resolution": resolution }),
        )
        .await?;
        Ok(Some(ticket))
    }

    async fn list_ticket_comments(
        &self,
        ticket_id: Uuid,
    ) -> Result<Vec<TicketCommentModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, ticket_id, author_user_id, body, created_at
             FROM ticket_comments
             WHERE ticket_id = $1
             ORDER BY created_at ASC",
        )
        .bind(ticket_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(ticket_comment_from_row).collect())
    }

    async fn list_ticket_events(
        &self,
        ticket_id: Uuid,
    ) -> Result<Vec<TicketEventModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, ticket_id, event_type, payload_json, created_at
             FROM ticket_events
             WHERE ticket_id = $1
             ORDER BY created_at ASC",
        )
        .bind(ticket_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(ticket_event_from_row).collect())
    }

    pub async fn list_anomaly_rules(&self) -> Result<Vec<AnomalyRuleModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, name, kind, scope_type, scope_id, config_json, is_active,
                    created_at, updated_at, created_by, updated_by
             FROM anomaly_rules
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(anomaly_rule_from_row).collect())
    }

    pub async fn get_anomaly_rule(
        &self,
        anomaly_rule_id: Uuid,
    ) -> Result<Option<AnomalyRuleModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, kind, scope_type, scope_id, config_json, is_active,
                    created_at, updated_at, created_by, updated_by
             FROM anomaly_rules
             WHERE id = $1",
        )
        .bind(anomaly_rule_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(anomaly_rule_from_row))
    }

    pub async fn create_anomaly_rule(
        &self,
        name: &str,
        kind: &str,
        scope_type: &str,
        scope_id: Option<Uuid>,
        config_json: &Value,
        is_active: bool,
        audit: &AuditInfo,
    ) -> Result<AnomalyRuleModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO anomaly_rules (
                id, name, kind, scope_type, scope_id, config_json, is_active,
                created_at, updated_at, created_by, updated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW(), $8, $8)
            RETURNING id, name, kind, scope_type, scope_id, config_json, is_active,
                      created_at, updated_at, created_by, updated_by",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(kind)
        .bind(scope_type)
        .bind(scope_id)
        .bind(config_json)
        .bind(is_active)
        .bind(&audit.actor_id)
        .fetch_one(&self.pool)
        .await?;

        let rule = anomaly_rule_from_row(row);
        self.record_audit_event_direct(
            "anomaly_rule",
            "anomaly.rule.create",
            rule.id,
            audit,
            json!({ "kind": rule.kind }),
        )
        .await?;
        Ok(rule)
    }

    pub async fn update_anomaly_rule(
        &self,
        anomaly_rule_id: Uuid,
        name: &str,
        config_json: &Value,
        is_active: bool,
        audit: &AuditInfo,
    ) -> Result<Option<AnomalyRuleModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE anomaly_rules
             SET name = $2,
                 config_json = $3,
                 is_active = $4,
                 updated_at = NOW(),
                 updated_by = $5
             WHERE id = $1
             RETURNING id, name, kind, scope_type, scope_id, config_json, is_active,
                       created_at, updated_at, created_by, updated_by",
        )
        .bind(anomaly_rule_id)
        .bind(name)
        .bind(config_json)
        .bind(is_active)
        .bind(&audit.actor_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let rule = anomaly_rule_from_row(row);
        self.record_audit_event_direct(
            "anomaly_rule",
            "anomaly.rule.update",
            anomaly_rule_id,
            audit,
            json!({ "name": name }),
        )
        .await?;
        Ok(Some(rule))
    }

    pub async fn list_anomaly_instances(&self) -> Result<Vec<AnomalyInstanceModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, rule_id, cluster_id, severity, status, started_at, resolved_at, payload_json
             FROM anomaly_instances
             ORDER BY started_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(anomaly_instance_from_row).collect())
    }

    pub async fn get_anomaly_instance(
        &self,
        anomaly_instance_id: Uuid,
    ) -> Result<Option<AnomalyInstanceModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, rule_id, cluster_id, severity, status, started_at, resolved_at, payload_json
             FROM anomaly_instances
             WHERE id = $1",
        )
        .bind(anomaly_instance_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(anomaly_instance_from_row))
    }

    async fn record_audit_event(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        entity_type: &str,
        action: &str,
        entity_id: Uuid,
        audit: &AuditInfo,
        payload_json: Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO control_audit_events (
                id, entity_type, entity_id, action, actor_id, actor_type,
                request_id, reason, payload_json, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(entity_type)
        .bind(entity_id.to_string())
        .bind(action)
        .bind(&audit.actor_id)
        .bind(&audit.actor_type)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .bind(payload_json)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn record_audit_event_direct(
        &self,
        entity_type: &str,
        action: &str,
        entity_id: Uuid,
        audit: &AuditInfo,
        payload_json: Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO control_audit_events (
                id, entity_type, entity_id, action, actor_id, actor_type,
                request_id, reason, payload_json, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(entity_type)
        .bind(entity_id.to_string())
        .bind(action)
        .bind(&audit.actor_id)
        .bind(&audit.actor_type)
        .bind(&audit.request_id)
        .bind(&audit.reason)
        .bind(payload_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct PermissionDefinition<'a> {
    pub code: &'a str,
    pub description: &'a str,
}

fn policy_from_row(row: PgRow) -> PolicyModel {
    PolicyModel {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        is_active: row.get("is_active"),
        latest_revision_id: row.get("latest_revision_id"),
        latest_revision: row.get("latest_revision"),
        policy_body_json: row.get("policy_body_json"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: row.get("updated_by"),
        update_reason: row.get("update_reason"),
    }
}

fn policy_revision_from_row(row: PgRow) -> PolicyRevisionModel {
    PolicyRevisionModel {
        id: row.get("id"),
        policy_id: row.get("policy_id"),
        revision: row.get("revision"),
        body_json: row.get("body_json"),
        created_at: row.get("created_at"),
        created_by: row.get("created_by"),
        reason: row.get("reason"),
        request_id: row.get("request_id"),
    }
}

fn host_from_row(row: PgRow) -> HostModel {
    HostModel {
        id: row.get("id"),
        hostname: row.get("hostname"),
        ip: row.get("ip"),
        ssh_port: row.get("ssh_port"),
        remote_user: row.get("remote_user"),
        labels_json: row.get("labels_json"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: row.get("updated_by"),
        update_reason: row.get("update_reason"),
    }
}

fn credential_from_row(row: PgRow) -> CredentialProfileModel {
    CredentialProfileModel {
        id: row.get("id"),
        name: row.get("name"),
        kind: row.get("kind"),
        description: row.get("description"),
        vault_ref: row.get("vault_ref"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: row.get("updated_by"),
        update_reason: row.get("update_reason"),
    }
}

fn cluster_from_row(row: PgRow) -> ClusterModel {
    ClusterModel {
        id: row.get("id"),
        name: row.get("name"),
        slug: row.get("slug"),
        description: row.get("description"),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: row.get("updated_by"),
        metadata_json: row.get("metadata_json"),
        host_count: row.get::<i64, _>("host_count"),
        agent_count: row.get::<i64, _>("agent_count"),
    }
}

fn cluster_host_from_row(row: PgRow) -> ClusterHostBindingModel {
    ClusterHostBindingModel {
        id: row.get("id"),
        cluster_id: row.get("cluster_id"),
        host_id: row.get("host_id"),
        hostname: row.get::<Option<String>, _>("hostname"),
        created_at: row.get("created_at"),
    }
}

fn cluster_agent_from_row(row: PgRow) -> ClusterAgentBindingModel {
    ClusterAgentBindingModel {
        id: row.get("id"),
        cluster_id: row.get("cluster_id"),
        agent_id: row.get("agent_id"),
        created_at: row.get("created_at"),
    }
}

fn role_from_row(row: PgRow) -> RoleModel {
    RoleModel {
        id: row.get("id"),
        name: row.get("name"),
        slug: row.get("slug"),
        description: row.get("description"),
        is_system: row.get("is_system"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: row.get("updated_by"),
    }
}

fn permission_from_row(row: PgRow) -> PermissionModel {
    PermissionModel {
        id: row.get("id"),
        code: row.get("code"),
        description: row.get("description"),
    }
}

fn role_binding_from_row(row: PgRow) -> RoleBindingModel {
    RoleBindingModel {
        id: row.get("id"),
        user_id: row.get("user_id"),
        role_id: row.get("role_id"),
        scope_type: row.get("scope_type"),
        scope_id: row.get::<Option<Uuid>, _>("scope_id"),
        created_at: row.get("created_at"),
    }
}

fn integration_from_row(row: PgRow) -> IntegrationModel {
    IntegrationModel {
        id: row.get("id"),
        name: row.get("name"),
        kind: row.get("kind"),
        description: row.get("description"),
        config_json: row.get("config_json"),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: row.get("updated_by"),
    }
}

fn integration_binding_from_row(row: PgRow) -> IntegrationBindingModel {
    IntegrationBindingModel {
        id: row.get("id"),
        integration_id: row.get("integration_id"),
        scope_type: row.get("scope_type"),
        scope_id: row.get::<Option<Uuid>, _>("scope_id"),
        event_types_json: row.get("event_types_json"),
        severity_threshold: row.get("severity_threshold"),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn ticket_from_row(row: PgRow) -> TicketModel {
    TicketModel {
        id: row.get("id"),
        ticket_key: row.get("ticket_key"),
        title: row.get("title"),
        description: row.get("description"),
        cluster_id: row.get("cluster_id"),
        cluster_name: row.get::<Option<String>, _>("cluster_name"),
        source_type: row.get("source_type"),
        source_id: row.get::<Option<String>, _>("source_id"),
        severity: row.get("severity"),
        status: row.get("status"),
        assignee_user_id: row.get::<Option<String>, _>("assignee_user_id"),
        created_by: row.get("created_by"),
        resolution: row.get("resolution"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        resolved_at: row.get("resolved_at"),
        closed_at: row.get("closed_at"),
    }
}

fn ticket_comment_from_row(row: PgRow) -> TicketCommentModel {
    TicketCommentModel {
        id: row.get("id"),
        ticket_id: row.get("ticket_id"),
        author_user_id: row.get("author_user_id"),
        body: row.get("body"),
        created_at: row.get("created_at"),
    }
}

fn ticket_event_from_row(row: PgRow) -> TicketEventModel {
    TicketEventModel {
        id: row.get("id"),
        ticket_id: row.get("ticket_id"),
        event_type: row.get("event_type"),
        payload_json: row.get("payload_json"),
        created_at: row.get("created_at"),
    }
}

fn anomaly_rule_from_row(row: PgRow) -> AnomalyRuleModel {
    AnomalyRuleModel {
        id: row.get("id"),
        name: row.get("name"),
        kind: row.get("kind"),
        scope_type: row.get("scope_type"),
        scope_id: row.get::<Option<Uuid>, _>("scope_id"),
        config_json: row.get("config_json"),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        created_by: row.get("created_by"),
        updated_by: row.get("updated_by"),
    }
}

fn anomaly_instance_from_row(row: PgRow) -> AnomalyInstanceModel {
    AnomalyInstanceModel {
        id: row.get("id"),
        rule_id: row.get("rule_id"),
        cluster_id: row.get::<Option<Uuid>, _>("cluster_id"),
        severity: row.get("severity"),
        status: row.get("status"),
        started_at: row.get("started_at"),
        resolved_at: row.get("resolved_at"),
        payload_json: row.get("payload_json"),
    }
}
