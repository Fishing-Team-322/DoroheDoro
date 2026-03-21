use std::collections::HashMap;

use common::{
    proto::runtime::{AuditContext, PagingRequest, PagingResponse},
    AppError, AppResult,
};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    models::{
        paging_response, CredentialProfileModel, HostGroupMemberModel, HostGroupModel, HostModel,
        PolicyModel, PolicyRevisionModel,
    },
    repository::ControlRepository,
};

#[derive(Debug, Clone)]
pub struct AuditInfo {
    pub actor_id: String,
    pub actor_type: String,
    pub request_id: String,
    pub reason: String,
}

impl AuditInfo {
    pub fn from_proto(
        correlation_id: &str,
        audit: Option<AuditContext>,
        default_reason: &str,
    ) -> Self {
        let audit = audit.unwrap_or_default();
        Self {
            actor_id: non_empty_or(audit.actor_id, "system"),
            actor_type: non_empty_or(audit.actor_type, "system"),
            request_id: non_empty_or(audit.request_id, correlation_id),
            reason: non_empty_or(audit.reason, default_reason),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListInput {
    pub limit: u32,
    pub offset: u64,
    pub query: Option<String>,
}

impl ListInput {
    pub fn from_proto(paging: Option<PagingRequest>) -> Self {
        let paging = paging.unwrap_or_default();
        Self {
            limit: if paging.limit == 0 {
                50
            } else {
                paging.limit.min(200)
            },
            offset: paging.offset,
            query: normalize_query(paging.query),
        }
    }

    pub fn paging(&self, total: u64) -> PagingResponse {
        paging_response(self.limit, self.offset, total)
    }
}

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

    pub async fn list_policies(
        &self,
        list: &ListInput,
    ) -> AppResult<(Vec<PolicyModel>, PagingResponse)> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let filtered = self
            .repo
            .list_policies()
            .await
            .map_err(map_db_error)?
            .into_iter()
            .filter(|policy| match &query {
                Some(query) => {
                    policy.name.to_ascii_lowercase().contains(query)
                        || policy.description.to_ascii_lowercase().contains(query)
                }
                None => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_policy(&self, policy_id: Uuid) -> AppResult<PolicyModel> {
        self.repo
            .get_policy(policy_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("policy {policy_id} not found")))
    }

    pub async fn create_policy(
        &self,
        input: PolicyCreateInput,
        audit: &AuditInfo,
    ) -> AppResult<PolicyModel> {
        validate_policy_fields(&input.name, &input.description)?;
        validate_policy_body(&input.body_json)?;
        self.repo
            .create_policy(&input.name, &input.description, &input.body_json, audit)
            .await
            .map_err(map_db_error)
    }

    pub async fn update_policy(
        &self,
        input: PolicyUpdateInput,
        audit: &AuditInfo,
    ) -> AppResult<PolicyModel> {
        validate_policy_fields("", &input.description)?;
        validate_policy_body(&input.body_json)?;
        self.repo
            .update_policy(input.policy_id, &input.description, &input.body_json, audit)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("policy {} not found", input.policy_id)))
    }

    pub async fn list_policy_revisions(
        &self,
        policy_id: Uuid,
        list: &ListInput,
    ) -> AppResult<(Vec<PolicyRevisionModel>, PagingResponse)> {
        let revisions = self
            .repo
            .list_policy_revisions(policy_id)
            .await
            .map_err(map_db_error)?;
        paginate(revisions, list)
    }

    pub async fn list_hosts(&self, list: &ListInput) -> AppResult<(Vec<HostModel>, PagingResponse)> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let filtered = self
            .repo
            .list_hosts()
            .await
            .map_err(map_db_error)?
            .into_iter()
            .filter(|host| match &query {
                Some(query) => {
                    host.hostname.to_ascii_lowercase().contains(query)
                        || host.ip.to_ascii_lowercase().contains(query)
                }
                None => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_host(&self, host_id: Uuid) -> AppResult<HostModel> {
        self.repo
            .get_host(host_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("host {host_id} not found")))
    }

    pub async fn create_host(
        &self,
        input: HostUpsertInput,
        audit: &AuditInfo,
    ) -> AppResult<HostModel> {
        validate_host_input(&input)?;
        let labels_json = labels_to_json(input.labels);
        self.repo
            .create_host(
                &input.hostname,
                &input.ip,
                input.ssh_port as i32,
                &input.remote_user,
                &labels_json,
                audit,
            )
            .await
            .map_err(map_db_error)
    }

    pub async fn update_host(
        &self,
        host_id: Uuid,
        input: HostUpsertInput,
        audit: &AuditInfo,
    ) -> AppResult<HostModel> {
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
                audit,
            )
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("host {host_id} not found")))
    }

    pub async fn list_host_groups(
        &self,
        list: &ListInput,
    ) -> AppResult<(Vec<HostGroupModel>, PagingResponse)> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let filtered = self
            .repo
            .list_host_groups()
            .await
            .map_err(map_db_error)?
            .into_iter()
            .filter(|group| match &query {
                Some(query) => {
                    group.name.to_ascii_lowercase().contains(query)
                        || group.description.to_ascii_lowercase().contains(query)
                }
                None => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_host_group(&self, host_group_id: Uuid) -> AppResult<HostGroupModel> {
        self.repo
            .get_host_group(host_group_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("host group {host_group_id} not found")))
    }

    pub async fn create_host_group(
        &self,
        input: HostGroupInput,
        audit: &AuditInfo,
    ) -> AppResult<HostGroupModel> {
        validate_host_group_input(&input)?;
        self.repo
            .create_host_group(&input.name, &input.description, audit)
            .await
            .map_err(map_db_error)
    }

    pub async fn update_host_group(
        &self,
        host_group_id: Uuid,
        input: HostGroupInput,
        audit: &AuditInfo,
    ) -> AppResult<HostGroupModel> {
        validate_host_group_input(&input)?;
        self.repo
            .update_host_group(host_group_id, &input.name, &input.description, audit)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("host group {host_group_id} not found")))
    }

    pub async fn add_host_to_group(
        &self,
        host_group_id: Uuid,
        host_id: Uuid,
        audit: &AuditInfo,
    ) -> AppResult<HostGroupMemberModel> {
        self.repo
            .add_host_group_member(host_group_id, host_id, audit)
            .await
            .map_err(map_db_error)
    }

    pub async fn remove_host_from_group(
        &self,
        host_group_id: Uuid,
        host_id: Uuid,
        audit: &AuditInfo,
    ) -> AppResult<()> {
        let removed = self
            .repo
            .remove_host_group_member(host_group_id, host_id, audit)
            .await
            .map_err(map_db_error)?;
        if removed {
            Ok(())
        } else {
            Err(AppError::not_found("host membership not found"))
        }
    }

    pub async fn list_credentials(
        &self,
        list: &ListInput,
    ) -> AppResult<(Vec<CredentialProfileModel>, PagingResponse)> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let filtered = self
            .repo
            .list_credentials()
            .await
            .map_err(map_db_error)?
            .into_iter()
            .filter(|profile| match &query {
                Some(query) => {
                    profile.name.to_ascii_lowercase().contains(query)
                        || profile.kind.to_ascii_lowercase().contains(query)
                        || profile.description.to_ascii_lowercase().contains(query)
                        || profile.vault_ref.to_ascii_lowercase().contains(query)
                }
                None => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
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
        audit: &AuditInfo,
    ) -> AppResult<CredentialProfileModel> {
        validate_credentials_input(&input)?;
        self.repo
            .create_credentials(
                &input.name,
                &input.kind,
                &input.description,
                &input.vault_ref,
                audit,
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

    let value: Value = serde_json::from_str(payload)
        .map_err(|error| AppError::invalid_argument(format!("invalid {field} json: {error}")))?;
    validate_policy_body(&value)?;
    Ok(value)
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

fn validate_policy_body(value: &Value) -> AppResult<()> {
    let object = value.as_object().ok_or_else(|| {
        AppError::invalid_argument("policy_body_json must be a JSON object")
    })?;

    let mut has_supported_section = false;

    if let Some(paths) = object.get("paths") {
        has_supported_section = true;
        validate_string_array(paths, "paths")?;
    }

    if let Some(sources) = object.get("sources") {
        has_supported_section = true;
        let entries = sources.as_array().ok_or_else(|| {
            AppError::invalid_argument("policy sources must be an array")
        })?;
        for source in entries {
            if let Some(source_str) = source.as_str() {
                if source_str.trim().is_empty() {
                    return Err(AppError::invalid_argument(
                        "policy sources must not contain empty strings",
                    ));
                }
                continue;
            }

            let Some(source_object) = source.as_object() else {
                return Err(AppError::invalid_argument(
                    "policy sources entries must be strings or objects",
                ));
            };

            let kind = source_object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("file")
                .trim();
            if kind.is_empty() || kind != "file" {
                return Err(AppError::invalid_argument(
                    "policy source type must be `file` when specified",
                ));
            }
            let path = source_object
                .get("path")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::invalid_argument("policy file source path is required"))?;
            let _ = path;
        }
    }

    if !has_supported_section {
        return Err(AppError::invalid_argument(
            "policy must define supported `paths` or `sources` sections",
        ));
    }

    Ok(())
}

fn validate_string_array(value: &Value, field: &str) -> AppResult<()> {
    let items = value.as_array().ok_or_else(|| {
        AppError::invalid_argument(format!("{field} must be an array"))
    })?;
    for item in items {
        let item = item.as_str().map(str::trim).filter(|value| !value.is_empty());
        if item.is_none() {
            return Err(AppError::invalid_argument(format!(
                "{field} must contain non-empty strings"
            )));
        }
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

fn normalize_query(query: String) -> Option<String> {
    let query = query.trim();
    if query.is_empty() {
        None
    } else {
        Some(query.to_string())
    }
}

fn non_empty_or(value: String, default: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn paginate<T: Clone>(items: Vec<T>, list: &ListInput) -> AppResult<(Vec<T>, PagingResponse)> {
    let total = items.len() as u64;
    let start = list.offset.min(total) as usize;
    let end = (start as u64 + list.limit as u64).min(total) as usize;
    Ok((items[start..end].to_vec(), list.paging(total)))
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
