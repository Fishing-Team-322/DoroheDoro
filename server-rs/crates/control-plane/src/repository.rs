use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use crate::models::{
    CredentialProfileModel, HostGroupMemberModel, HostGroupModel, HostModel, PolicyModel,
    PolicyRevisionModel,
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
             ORDER BY p.created_at ASC",
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
    ) -> Result<PolicyModel, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let policy_id = Uuid::new_v4();
        let revision_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO policies (id, name, description, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, TRUE, $4, $4)",
        )
        .bind(policy_id)
        .bind(name)
        .bind(description)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO policy_revisions (id, policy_id, revision, body_json, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(revision_id)
        .bind(policy_id)
        .bind("rev-1")
        .bind(body_json)
        .bind(now)
        .execute(&mut *tx)
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
    ) -> Result<Option<PolicyModel>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        sqlx::query(
            "UPDATE policies
             SET description = $2, updated_at = $3
             WHERE id = $1",
        )
        .bind(policy_id)
        .bind(description)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let revision_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM policy_revisions WHERE policy_id = $1")
                .bind(policy_id)
                .fetch_one(&mut *tx)
                .await?;
        let revision_label = format!("rev-{}", revision_count + 1);

        sqlx::query(
            "INSERT INTO policy_revisions (id, policy_id, revision, body_json, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(policy_id)
        .bind(&revision_label)
        .bind(body_json)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_policy(policy_id).await
    }

    pub async fn list_policy_revisions(
        &self,
        policy_id: Uuid,
    ) -> Result<Vec<PolicyRevisionModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, policy_id, revision, body_json, created_at
             FROM policy_revisions
             WHERE policy_id = $1
             ORDER BY created_at DESC",
        )
        .bind(policy_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(policy_revision_from_row).collect())
    }

    pub async fn list_hosts(&self) -> Result<Vec<HostModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at
             FROM hosts
             ORDER BY hostname ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(host_from_row).collect())
    }

    pub async fn get_host(&self, host_id: Uuid) -> Result<Option<HostModel>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at
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
    ) -> Result<HostModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO hosts (id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
             RETURNING id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(hostname)
        .bind(ip)
        .bind(ssh_port)
        .bind(remote_user)
        .bind(labels_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(host_from_row(row))
    }

    pub async fn update_host(
        &self,
        host_id: Uuid,
        hostname: &str,
        ip: &str,
        ssh_port: i32,
        remote_user: &str,
        labels_json: &Value,
    ) -> Result<Option<HostModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE hosts
             SET hostname = $2,
                 ip = $3,
                 ssh_port = $4,
                 remote_user = $5,
                 labels_json = $6,
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, hostname, ip, ssh_port, remote_user, labels_json, created_at, updated_at",
        )
        .bind(host_id)
        .bind(hostname)
        .bind(ip)
        .bind(ssh_port)
        .bind(remote_user)
        .bind(labels_json)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(host_from_row))
    }

    pub async fn list_host_groups(&self) -> Result<Vec<HostGroupModel>, sqlx::Error> {
        let group_rows = sqlx::query(
            "SELECT id, name, description, created_at, updated_at
             FROM host_groups
             ORDER BY name ASC",
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

        Ok(groups.into_values().collect())
    }

    pub async fn get_host_group(
        &self,
        host_group_id: Uuid,
    ) -> Result<Option<HostGroupModel>, sqlx::Error> {
        let group_row = sqlx::query(
            "SELECT id, name, description, created_at, updated_at
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
    ) -> Result<HostGroupModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO host_groups (id, name, description, created_at, updated_at)
             VALUES ($1, $2, $3, NOW(), NOW())
             RETURNING id, name, description, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(description)
        .fetch_one(&self.pool)
        .await?;

        Ok(HostGroupModel {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            members: Vec::new(),
        })
    }

    pub async fn update_host_group(
        &self,
        host_group_id: Uuid,
        name: &str,
        description: &str,
    ) -> Result<Option<HostGroupModel>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE host_groups
             SET name = $2,
                 description = $3,
                 updated_at = NOW()
             WHERE id = $1
             RETURNING id, name, description, created_at, updated_at",
        )
        .bind(host_group_id)
        .bind(name)
        .bind(description)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let mut group = HostGroupModel {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
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

    pub async fn add_host_group_member(
        &self,
        host_group_id: Uuid,
        host_id: Uuid,
    ) -> Result<HostGroupMemberModel, sqlx::Error> {
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
        .fetch_one(&self.pool)
        .await?;

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
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM host_group_members WHERE host_group_id = $1 AND host_id = $2")
                .bind(host_group_id)
                .bind(host_id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_credentials(&self) -> Result<Vec<CredentialProfileModel>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, name, kind, description, vault_ref, created_at, updated_at
             FROM credentials_profiles_metadata
             ORDER BY created_at DESC",
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
            "SELECT id, name, kind, description, vault_ref, created_at, updated_at
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
    ) -> Result<CredentialProfileModel, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO credentials_profiles_metadata (id, name, kind, description, vault_ref, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
             RETURNING id, name, kind, description, vault_ref, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(kind)
        .bind(description)
        .bind(vault_ref)
        .fetch_one(&self.pool)
        .await?;

        Ok(credential_from_row(row))
    }
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
    }
}

fn policy_revision_from_row(row: PgRow) -> PolicyRevisionModel {
    PolicyRevisionModel {
        id: row.get("id"),
        policy_id: row.get("policy_id"),
        revision: row.get("revision"),
        body_json: row.get("body_json"),
        created_at: row.get("created_at"),
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
    }
}
