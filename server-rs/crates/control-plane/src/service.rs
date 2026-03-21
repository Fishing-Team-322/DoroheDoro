use std::collections::HashMap;

use common::{AppError, AppResult};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    models::{
        CredentialProfileModel, HostGroupMemberModel, HostGroupModel, HostModel, PolicyModel,
        PolicyRevisionModel,
    },
    repository::ControlRepository,
};

#[derive(Debug, Clone)]
pub struct PolicyCreateInput {
    pub name: String,
    pub description: String,
    pub body_json: Value,
}

#[derive(Debug, Clone)]
pub struct PolicyUpdateInput {
    pub policy_id: Uuid,
    pub description: String,
    pub body_json: Value,
}

#[derive(Debug, Clone)]
pub struct HostUpsertInput {
    pub hostname: String,
    pub ip: String,
    pub ssh_port: u16,
    pub remote_user: String,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct HostGroupInput {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct CredentialProfileInput {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub vault_ref: String,
}

#[derive(Clone)]
pub struct ControlService {
    repo: ControlRepository,
}

impl ControlService {
    pub fn new(repo: ControlRepository) -> Self {
        Self { repo }
    }

    pub async fn list_policies(&self) -> AppResult<Vec<PolicyModel>> {
        self.repo.list_policies().await.map_err(map_db_error)
    }

    pub async fn get_policy(&self, policy_id: Uuid) -> AppResult<PolicyModel> {
        self.repo
            .get_policy(policy_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("policy {policy_id} not found")))
    }

    pub async fn create_policy(&self, input: PolicyCreateInput) -> AppResult<PolicyModel> {
        validate_policy_fields(&input.name, &input.description)?;
        self.repo
            .create_policy(&input.name, &input.description, &input.body_json)
            .await
            .map_err(map_db_error)
    }

    pub async fn update_policy(&self, input: PolicyUpdateInput) -> AppResult<PolicyModel> {
        validate_policy_fields("", &input.description)?;
        self.repo
            .update_policy(input.policy_id, &input.description, &input.body_json)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("policy {} not found", input.policy_id)))
    }

    pub async fn list_policy_revisions(
        &self,
        policy_id: Uuid,
    ) -> AppResult<Vec<PolicyRevisionModel>> {
        self.repo
            .list_policy_revisions(policy_id)
            .await
            .map_err(map_db_error)
    }

    pub async fn list_hosts(&self) -> AppResult<Vec<HostModel>> {
        self.repo.list_hosts().await.map_err(map_db_error)
    }

    pub async fn get_host(&self, host_id: Uuid) -> AppResult<HostModel> {
        self.repo
            .get_host(host_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("host {host_id} not found")))
    }

    pub async fn create_host(&self, input: HostUpsertInput) -> AppResult<HostModel> {
        validate_host_input(&input)?;
        let labels_json = labels_to_json(input.labels);
        self.repo
            .create_host(
                &input.hostname,
                &input.ip,
                input.ssh_port as i32,
                &input.remote_user,
                &labels_json,
            )
            .await
            .map_err(map_db_error)
    }

    pub async fn update_host(&self, host_id: Uuid, input: HostUpsertInput) -> AppResult<HostModel> {
        validate_host_input(&input)?;
        let labels_json = labels_to_json(input.labels);
        self.repo
            .update_host(
                host_id,
                &input.hostname,
                &input.ip,
                input.ssh_port as i32,
                &input.remote_user,
                &labels_json,
            )
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("host {host_id} not found")))
    }

    pub async fn list_host_groups(&self) -> AppResult<Vec<HostGroupModel>> {
        self.repo.list_host_groups().await.map_err(map_db_error)
    }

    pub async fn get_host_group(&self, host_group_id: Uuid) -> AppResult<HostGroupModel> {
        self.repo
            .get_host_group(host_group_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("host group {host_group_id} not found")))
    }

    pub async fn create_host_group(&self, input: HostGroupInput) -> AppResult<HostGroupModel> {
        validate_host_group_input(&input)?;
        self.repo
            .create_host_group(&input.name, &input.description)
            .await
            .map_err(map_db_error)
    }

    pub async fn update_host_group(
        &self,
        host_group_id: Uuid,
        input: HostGroupInput,
    ) -> AppResult<HostGroupModel> {
        validate_host_group_input(&input)?;
        self.repo
            .update_host_group(host_group_id, &input.name, &input.description)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("host group {host_group_id} not found")))
    }

    pub async fn add_host_to_group(
        &self,
        host_group_id: Uuid,
        host_id: Uuid,
    ) -> AppResult<HostGroupMemberModel> {
        self.repo
            .add_host_group_member(host_group_id, host_id)
            .await
            .map_err(map_db_error)
    }

    pub async fn remove_host_from_group(
        &self,
        host_group_id: Uuid,
        host_id: Uuid,
    ) -> AppResult<()> {
        let removed = self
            .repo
            .remove_host_group_member(host_group_id, host_id)
            .await
            .map_err(map_db_error)?;
        if removed {
            Ok(())
        } else {
            Err(AppError::not_found("host membership not found"))
        }
    }

    pub async fn list_credentials(&self) -> AppResult<Vec<CredentialProfileModel>> {
        self.repo.list_credentials().await.map_err(map_db_error)
    }

    pub async fn get_credentials(&self, credentials_id: Uuid) -> AppResult<CredentialProfileModel> {
        self.repo
            .get_credentials(credentials_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| {
                AppError::not_found(format!("credentials profile {credentials_id} not found"))
            })
    }

    pub async fn create_credentials(
        &self,
        input: CredentialProfileInput,
    ) -> AppResult<CredentialProfileModel> {
        validate_credentials_input(&input)?;
        self.repo
            .create_credentials(
                &input.name,
                &input.kind,
                &input.description,
                &input.vault_ref,
            )
            .await
            .map_err(map_db_error)
    }
}

pub fn parse_policy_json(field: &str, payload: &str) -> AppResult<Value> {
    if payload.trim().is_empty() {
        return Err(AppError::invalid_argument(format!(
            "{field} cannot be empty"
        )));
    }

    serde_json::from_str(payload)
        .map_err(|error| AppError::invalid_argument(format!("invalid {field} json: {error}")))
}

fn validate_policy_fields(name: &str, description: &str) -> AppResult<()> {
    if !name.is_empty() && name.trim().is_empty() {
        return Err(AppError::invalid_argument("policy name cannot be blank"));
    }
    if description.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "policy description cannot be blank",
        ));
    }
    Ok(())
}

fn validate_host_input(input: &HostUpsertInput) -> AppResult<()> {
    if input.hostname.trim().is_empty() {
        return Err(AppError::invalid_argument("hostname is required"));
    }
    if input.ip.trim().is_empty() {
        return Err(AppError::invalid_argument("ip is required"));
    }
    if input.ssh_port == 0 {
        return Err(AppError::invalid_argument(
            "ssh_port must be greater than 0",
        ));
    }
    if input.remote_user.trim().is_empty() {
        return Err(AppError::invalid_argument("remote_user is required"));
    }
    Ok(())
}

fn validate_host_group_input(input: &HostGroupInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::invalid_argument("host group name is required"));
    }
    Ok(())
}

fn validate_credentials_input(input: &CredentialProfileInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "credentials profile name is required",
        ));
    }
    if input.kind.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "credentials profile kind is required",
        ));
    }
    if input.vault_ref.trim().is_empty() {
        return Err(AppError::invalid_argument("vault reference is required"));
    }
    Ok(())
}

fn labels_to_json(labels: HashMap<String, String>) -> Value {
    let mut map = Map::new();
    for (k, v) in labels {
        map.insert(k, Value::String(v));
    }
    Value::Object(map)
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
