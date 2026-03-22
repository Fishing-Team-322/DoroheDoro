use std::collections::HashMap;

use common::{
    json::AuditAppendEvent,
    proto::runtime::{AuditContext, PagingRequest, PagingResponse},
    AppError, AppResult,
};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    models::{
        paging_response, AnomalyInstanceModel, AnomalyRuleModel, AuditEventModel,
        ClusterDetailsModel, ClusterModel, CredentialProfileModel, HostGroupMemberModel,
        HostGroupModel, HostModel, IntegrationBindingModel, IntegrationModel, PermissionModel,
        PolicyModel, PolicyRevisionModel, RoleBindingModel, RoleModel, TicketCommentModel,
        TicketDetailsModel, TicketModel,
    },
    repository::{ControlRepository, PermissionDefinition},
    telegram::{
        normalize_binding_event_types, normalize_binding_scope, normalize_delivery_severity,
        normalize_telegram_config, sanitize_integration_model, TELEGRAM_INTEGRATION_KIND,
    },
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

#[derive(Debug, Clone)]
pub struct ClusterCreateInput {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub is_active: bool,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub struct ClusterUpdateInput {
    pub cluster_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub is_active: bool,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub struct ClusterListFilter {
    pub host_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ClusterHostMutationInput {
    pub cluster_id: Uuid,
    pub host_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RoleCreateInput {
    pub name: String,
    pub slug: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct RoleUpdateInput {
    pub role_id: Uuid,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct RoleBindingInput {
    pub user_id: String,
    pub role_id: Uuid,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct IntegrationInput {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub config: Value,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct IntegrationUpdateInput {
    pub integration_id: Uuid,
    pub name: String,
    pub description: String,
    pub config: Value,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct IntegrationBindingInput {
    pub integration_id: Uuid,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub event_types: Value,
    pub severity_threshold: String,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct TicketCreateInput {
    pub title: String,
    pub description: String,
    pub cluster_id: Uuid,
    pub source_type: String,
    pub source_id: Option<String>,
    pub severity: String,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct TicketAssignInput {
    pub ticket_id: Uuid,
    pub assignee_user_id: String,
}

#[derive(Debug, Clone)]
pub struct TicketStatusChangeInput {
    pub ticket_id: Uuid,
    pub status: String,
    pub resolution: String,
}

#[derive(Debug, Clone)]
pub struct TicketCommentInput {
    pub ticket_id: Uuid,
    pub body: String,
    pub author_user_id: String,
}

#[derive(Debug, Clone)]
pub struct TicketListFilter {
    pub cluster_id: Option<Uuid>,
    pub status: Option<String>,
    pub severity: Option<String>,
    pub assignee_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoleBindingListFilter {
    pub user_id: Option<String>,
    pub role_id: Option<Uuid>,
    pub scope_type: Option<String>,
    pub scope_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct AnomalyRuleInput {
    pub name: String,
    pub kind: String,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub config: Value,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct AnomalyRuleUpdateInput {
    pub anomaly_rule_id: Uuid,
    pub name: String,
    pub config: Value,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct AnomalyRuleListFilter {
    pub scope_type: Option<String>,
    pub scope_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct AnomalyInstanceFilter {
    pub anomaly_rule_id: Option<Uuid>,
    pub cluster_id: Option<Uuid>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditEventListFilter {
    pub event_type: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub actor_id: Option<String>,
}

const ROLE_SCOPE_GLOBAL: &str = "global";
const ROLE_SCOPE_CLUSTER: &str = "cluster";
const INTEGRATION_KIND_TELEGRAM: &str = "telegram_bot";
const INTEGRATION_KIND_SLACK: &str = "slack_webhook";
const INTEGRATION_KIND_WEBHOOK: &str = "generic_webhook";
const ALLOWED_INTEGRATION_KINDS: &[&str] = &[
    INTEGRATION_KIND_TELEGRAM,
    INTEGRATION_KIND_SLACK,
    INTEGRATION_KIND_WEBHOOK,
];
const TICKET_STATUSES: &[&str] = &["open", "triage", "in_progress", "resolved", "closed"];
const TICKET_SEVERITIES: &[&str] = &["info", "low", "medium", "high", "critical"];
const ANOMALY_KINDS: &[&str] = &["threshold", "baseline", "new_pattern"];

const PERMISSION_DEFINITIONS: &[PermissionDefinition<'static>] = &[
    PermissionDefinition {
        code: "logs.view",
        description: "View log search results",
    },
    PermissionDefinition {
        code: "agents.view",
        description: "View registered agents",
    },
    PermissionDefinition {
        code: "agents.manage",
        description: "Manage agent lifecycle",
    },
    PermissionDefinition {
        code: "policies.view",
        description: "View policies",
    },
    PermissionDefinition {
        code: "policies.manage",
        description: "Manage policies and revisions",
    },
    PermissionDefinition {
        code: "deployments.view",
        description: "View deployment jobs",
    },
    PermissionDefinition {
        code: "deployments.manage",
        description: "Trigger and manage deployments",
    },
    PermissionDefinition {
        code: "clusters.view",
        description: "View clusters and membership",
    },
    PermissionDefinition {
        code: "clusters.manage",
        description: "Manage clusters and membership",
    },
    PermissionDefinition {
        code: "integrations.view",
        description: "View integrations",
    },
    PermissionDefinition {
        code: "integrations.manage",
        description: "Manage integrations and bindings",
    },
    PermissionDefinition {
        code: "tickets.view",
        description: "View tickets",
    },
    PermissionDefinition {
        code: "tickets.manage",
        description: "Manage ticket lifecycle",
    },
    PermissionDefinition {
        code: "tickets.assign",
        description: "Assign tickets to users",
    },
    PermissionDefinition {
        code: "tickets.comment",
        description: "Comment on tickets",
    },
    PermissionDefinition {
        code: "tickets.close",
        description: "Close tickets",
    },
    PermissionDefinition {
        code: "roles.view",
        description: "View roles and permissions",
    },
    PermissionDefinition {
        code: "roles.manage",
        description: "Manage roles and bindings",
    },
    PermissionDefinition {
        code: "alerts.view",
        description: "View alerts and anomaly instances",
    },
    PermissionDefinition {
        code: "alerts.manage",
        description: "Manage alert/anomaly rules",
    },
];

#[derive(Clone)]
pub struct ControlService {
    repo: ControlRepository,
}

impl ControlService {
    pub fn new(repo: ControlRepository) -> Self {
        Self { repo }
    }

    pub async fn bootstrap(&self) -> AppResult<()> {
        self.repo
            .ensure_permission_catalog(PERMISSION_DEFINITIONS)
            .await
            .map_err(map_db_error)
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

    pub async fn list_hosts(
        &self,
        list: &ListInput,
    ) -> AppResult<(Vec<HostModel>, PagingResponse)> {
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

    pub async fn list_runtime_audit_events(
        &self,
        list: &ListInput,
        filter: &AuditEventListFilter,
    ) -> AppResult<(Vec<AuditEventModel>, PagingResponse)> {
        let (items, total) = self
            .repo
            .list_runtime_audit_events(
                filter.event_type.as_deref(),
                filter.entity_type.as_deref(),
                filter.entity_id.as_deref(),
                filter.actor_id.as_deref(),
                list.limit,
                list.offset,
            )
            .await
            .map_err(map_db_error)?;
        Ok((items, paging_response(list.limit, list.offset, total)))
    }

    pub async fn append_runtime_audit_event(
        &self,
        event: AuditAppendEvent,
    ) -> AppResult<AuditEventModel> {
        if event.event_type.trim().is_empty() {
            return Err(AppError::invalid_argument("event_type is required"));
        }
        if event.entity_type.trim().is_empty() {
            return Err(AppError::invalid_argument("entity_type is required"));
        }
        if event.entity_id.trim().is_empty() {
            return Err(AppError::invalid_argument("entity_id is required"));
        }
        if event.actor_id.trim().is_empty() {
            return Err(AppError::invalid_argument("actor_id is required"));
        }
        if event.request_id.trim().is_empty() {
            return Err(AppError::invalid_argument("request_id is required"));
        }
        self.repo
            .append_runtime_audit_event(&event)
            .await
            .map_err(map_db_error)
    }
}

impl ControlService {
    pub async fn list_clusters(
        &self,
        list: &ListInput,
        filter: &ClusterListFilter,
    ) -> AppResult<(Vec<ClusterModel>, PagingResponse)> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let clusters = self
            .repo
            .list_clusters(filter.host_id)
            .await
            .map_err(map_db_error)?;
        let filtered = clusters
            .into_iter()
            .filter(|cluster| match &query {
                Some(q) => {
                    cluster.name.to_ascii_lowercase().contains(q)
                        || cluster.slug.to_ascii_lowercase().contains(q)
                        || cluster.description.to_ascii_lowercase().contains(q)
                }
                None => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_cluster(&self, cluster_id: Uuid) -> AppResult<ClusterDetailsModel> {
        self.repo
            .get_cluster_details(cluster_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("cluster {cluster_id} not found")))
    }

    pub async fn create_cluster(
        &self,
        input: ClusterCreateInput,
        audit: &AuditInfo,
    ) -> AppResult<ClusterDetailsModel> {
        validate_cluster_fields(&input.name, &input.slug)?;
        self.repo
            .create_cluster(
                &input.name,
                &input.slug,
                &input.description,
                input.is_active,
                &input.metadata,
                audit,
            )
            .await
            .map_err(map_db_error)
    }

    pub async fn update_cluster(
        &self,
        input: ClusterUpdateInput,
        audit: &AuditInfo,
    ) -> AppResult<ClusterDetailsModel> {
        validate_cluster_fields(&input.name, &input.slug)?;
        self.repo
            .update_cluster(
                input.cluster_id,
                &input.name,
                &input.slug,
                &input.description,
                input.is_active,
                &input.metadata,
                audit,
            )
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("cluster {} not found", input.cluster_id)))
    }

    pub async fn add_host_to_cluster(
        &self,
        input: ClusterHostMutationInput,
        audit: &AuditInfo,
    ) -> AppResult<ClusterDetailsModel> {
        self.repo
            .add_host_to_cluster(input.cluster_id, input.host_id, audit)
            .await
            .map_err(map_db_error)?;
        self.get_cluster(input.cluster_id).await
    }

    pub async fn remove_host_from_cluster(
        &self,
        input: ClusterHostMutationInput,
        audit: &AuditInfo,
    ) -> AppResult<ClusterDetailsModel> {
        let removed = self
            .repo
            .remove_host_from_cluster(input.cluster_id, input.host_id, audit)
            .await
            .map_err(map_db_error)?;
        if !removed {
            return Err(AppError::not_found("cluster host membership not found"));
        }
        self.get_cluster(input.cluster_id).await
    }

    pub async fn list_roles(
        &self,
        list: &ListInput,
    ) -> AppResult<(Vec<RoleModel>, PagingResponse)> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let roles = self.repo.list_roles().await.map_err(map_db_error)?;
        let filtered = roles
            .into_iter()
            .filter(|role| match &query {
                Some(q) => {
                    role.name.to_ascii_lowercase().contains(q)
                        || role.slug.to_ascii_lowercase().contains(q)
                        || role.description.to_ascii_lowercase().contains(q)
                }
                None => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_role(&self, role_id: Uuid) -> AppResult<RoleModel> {
        self.repo
            .get_role(role_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("role {role_id} not found")))
    }

    pub async fn create_role(
        &self,
        input: RoleCreateInput,
        audit: &AuditInfo,
    ) -> AppResult<RoleModel> {
        validate_role_fields(&input.name, &input.slug)?;
        self.repo
            .create_role(&input.name, &input.slug, &input.description, audit)
            .await
            .map_err(map_db_error)
    }

    pub async fn update_role(
        &self,
        input: RoleUpdateInput,
        audit: &AuditInfo,
    ) -> AppResult<RoleModel> {
        if input.name.trim().is_empty() {
            return Err(AppError::invalid_argument("role name cannot be empty"));
        }
        self.repo
            .update_role(input.role_id, &input.name, &input.description, audit)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("role {} not found", input.role_id)))
    }

    pub async fn get_role_permissions(
        &self,
        role_id: Uuid,
    ) -> AppResult<(RoleModel, Vec<PermissionModel>)> {
        let role = self.get_role(role_id).await?;
        let permissions = self
            .repo
            .get_role_permissions(role_id)
            .await
            .map_err(map_db_error)?;
        Ok((role, permissions))
    }

    pub async fn set_role_permissions(
        &self,
        role_id: Uuid,
        permission_codes: Vec<String>,
        audit: &AuditInfo,
    ) -> AppResult<(RoleModel, Vec<PermissionModel>)> {
        self.repo
            .ensure_permission_catalog(PERMISSION_DEFINITIONS)
            .await
            .map_err(map_db_error)?;
        let permissions = self
            .repo
            .permissions_by_codes(&permission_codes)
            .await
            .map_err(map_db_error)?;
        if permissions.len() != permission_codes.len() {
            return Err(AppError::invalid_argument(
                "one or more permission codes are invalid",
            ));
        }
        let ids = permissions.iter().map(|perm| perm.id).collect::<Vec<_>>();
        let new_permissions = self
            .repo
            .set_role_permissions(role_id, &ids, audit)
            .await
            .map_err(map_db_error)?;
        let role = self.get_role(role_id).await?;
        Ok((role, new_permissions))
    }

    pub async fn list_role_bindings(
        &self,
        list: &ListInput,
        filter: &RoleBindingListFilter,
    ) -> AppResult<(Vec<RoleBindingModel>, PagingResponse)> {
        let bindings = self.repo.list_role_bindings().await.map_err(map_db_error)?;
        let filtered = bindings
            .into_iter()
            .filter(|binding| match &filter.user_id {
                Some(user_id) if binding.user_id != *user_id => false,
                _ => true,
            })
            .filter(|binding| match filter.role_id {
                Some(role_id) if binding.role_id != role_id => false,
                _ => true,
            })
            .filter(|binding| match &filter.scope_type {
                Some(scope_type) if binding.scope_type != *scope_type => false,
                _ => true,
            })
            .filter(|binding| match filter.scope_id {
                Some(scope_id) if binding.scope_id != Some(scope_id) => false,
                _ => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn create_role_binding(
        &self,
        input: RoleBindingInput,
        audit: &AuditInfo,
    ) -> AppResult<RoleBindingModel> {
        validate_scope(&input.scope_type, input.scope_id)?;
        if input.user_id.trim().is_empty() {
            return Err(AppError::invalid_argument("user_id is required"));
        }
        self.repo
            .create_role_binding(
                &input.user_id,
                input.role_id,
                &input.scope_type,
                input.scope_id,
                audit,
            )
            .await
            .map_err(map_db_error)
    }

    pub async fn delete_role_binding(
        &self,
        role_binding_id: Uuid,
        audit: &AuditInfo,
    ) -> AppResult<()> {
        let deleted = self
            .repo
            .delete_role_binding(role_binding_id, audit)
            .await
            .map_err(map_db_error)?;
        if deleted {
            Ok(())
        } else {
            Err(AppError::not_found("role binding not found"))
        }
    }

    pub async fn list_integrations(
        &self,
        list: &ListInput,
    ) -> AppResult<(Vec<IntegrationModel>, PagingResponse)> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let integrations = self
            .repo
            .list_integrations()
            .await
            .map_err(map_db_error)?
            .into_iter()
            .map(sanitize_integration_model)
            .collect::<Vec<_>>();
        let filtered = integrations
            .into_iter()
            .filter(|integration| match &query {
                Some(q) => {
                    integration.name.to_ascii_lowercase().contains(q)
                        || integration.kind.to_ascii_lowercase().contains(q)
                        || integration.description.to_ascii_lowercase().contains(q)
                }
                None => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_integration(
        &self,
        integration_id: Uuid,
    ) -> AppResult<(IntegrationModel, Vec<IntegrationBindingModel>)> {
        let integration = self
            .repo
            .get_integration(integration_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("integration {integration_id} not found")))
            .map(sanitize_integration_model)?;
        let bindings = self
            .repo
            .list_integration_bindings(integration_id)
            .await
            .map_err(map_db_error)?;
        Ok((integration, bindings))
    }

    pub async fn create_integration(
        &self,
        input: IntegrationInput,
        audit: &AuditInfo,
    ) -> AppResult<IntegrationModel> {
        validate_integration_kind(&input.kind)?;
        validate_json_object(&input.config, "config_json")?;
        let normalized_config = normalize_telegram_config(&input.kind, &input.name, &input.config)?;
        let integration = self
            .repo
            .create_integration(
                &input.name,
                &input.kind,
                &input.description,
                &normalized_config,
                input.is_active,
                audit,
            )
            .await
            .map_err(map_db_error)?;
        Ok(sanitize_integration_model(integration))
    }

    pub async fn update_integration(
        &self,
        input: IntegrationUpdateInput,
        audit: &AuditInfo,
    ) -> AppResult<IntegrationModel> {
        validate_json_object(&input.config, "config_json")?;
        let existing = self
            .repo
            .get_integration(input.integration_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| {
                AppError::not_found(format!("integration {} not found", input.integration_id))
            })?;
        let normalized_config =
            normalize_telegram_config(&existing.kind, &input.name, &input.config)?;
        let integration = self
            .repo
            .update_integration(
                input.integration_id,
                &input.name,
                &input.description,
                &normalized_config,
                input.is_active,
                audit,
            )
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| {
                AppError::not_found(format!("integration {} not found", input.integration_id))
            })?;
        Ok(sanitize_integration_model(integration))
    }

    pub async fn bind_integration(
        &self,
        input: IntegrationBindingInput,
        audit: &AuditInfo,
    ) -> AppResult<IntegrationBindingModel> {
        let integration = self
            .repo
            .get_integration(input.integration_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| {
                AppError::not_found(format!("integration {} not found", input.integration_id))
            })?;
        let scope_type = normalize_binding_scope(&input.scope_type, input.scope_id)?;
        let severity_threshold = normalize_delivery_severity(&input.severity_threshold)?;
        let event_types = if integration.kind == TELEGRAM_INTEGRATION_KIND {
            normalize_binding_event_types(&integration.kind, &input.event_types)?
        } else {
            validate_event_types(&input.event_types)?;
            input.event_types.clone()
        };
        self.repo
            .create_integration_binding(
                input.integration_id,
                &scope_type,
                input.scope_id,
                &event_types,
                &severity_threshold,
                input.is_active,
                audit,
            )
            .await
            .map_err(map_db_error)
    }

    pub async fn unbind_integration(
        &self,
        integration_binding_id: Uuid,
        audit: &AuditInfo,
    ) -> AppResult<()> {
        let deleted = self
            .repo
            .delete_integration_binding(integration_binding_id, audit)
            .await
            .map_err(map_db_error)?;
        if deleted {
            Ok(())
        } else {
            Err(AppError::not_found("integration binding not found"))
        }
    }

    pub async fn list_tickets(
        &self,
        list: &ListInput,
        filter: &TicketListFilter,
    ) -> AppResult<(Vec<TicketModel>, PagingResponse)> {
        let tickets = self.repo.list_tickets().await.map_err(map_db_error)?;
        let filtered = tickets
            .into_iter()
            .filter(|ticket| match filter.cluster_id {
                Some(cluster_id) if ticket.cluster_id != cluster_id => false,
                _ => true,
            })
            .filter(|ticket| match &filter.status {
                Some(status) if ticket.status != *status => false,
                _ => true,
            })
            .filter(|ticket| match &filter.severity {
                Some(severity) if ticket.severity != *severity => false,
                _ => true,
            })
            .filter(|ticket| match &filter.assignee_user_id {
                Some(assignee) => ticket
                    .assignee_user_id
                    .as_ref()
                    .map(|value| value == assignee)
                    .unwrap_or(false),
                None => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_ticket(&self, ticket_id: Uuid) -> AppResult<TicketDetailsModel> {
        self.repo
            .get_ticket_details(ticket_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("ticket {ticket_id} not found")))
    }

    pub async fn create_ticket(
        &self,
        input: TicketCreateInput,
        audit: &AuditInfo,
    ) -> AppResult<TicketDetailsModel> {
        validate_ticket_input(&input)?;
        let ticket_key = generate_ticket_key();
        self.repo
            .create_ticket(
                &ticket_key,
                &input.title,
                &input.description,
                input.cluster_id,
                &input.source_type,
                input.source_id.as_deref(),
                &input.severity,
                "open",
                &input.created_by,
                audit,
            )
            .await
            .map_err(map_db_error)
    }

    pub async fn assign_ticket(
        &self,
        input: TicketAssignInput,
        audit: &AuditInfo,
    ) -> AppResult<TicketModel> {
        if input.assignee_user_id.trim().is_empty() {
            return Err(AppError::invalid_argument("assignee_user_id is required"));
        }
        self.repo
            .assign_ticket(input.ticket_id, &input.assignee_user_id, audit)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("ticket {} not found", input.ticket_id)))
    }

    pub async fn unassign_ticket(
        &self,
        ticket_id: Uuid,
        audit: &AuditInfo,
    ) -> AppResult<TicketModel> {
        self.repo
            .unassign_ticket(ticket_id, audit)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("ticket {ticket_id} not found")))
    }

    pub async fn comment_ticket(
        &self,
        input: TicketCommentInput,
        audit: &AuditInfo,
    ) -> AppResult<TicketCommentModel> {
        if input.body.trim().is_empty() {
            return Err(AppError::invalid_argument("comment body is required"));
        }
        if input.author_user_id.trim().is_empty() {
            return Err(AppError::invalid_argument("author_user_id is required"));
        }
        self.repo
            .add_ticket_comment(input.ticket_id, &input.author_user_id, &input.body, audit)
            .await
            .map_err(map_db_error)
    }

    pub async fn change_ticket_status(
        &self,
        input: TicketStatusChangeInput,
        audit: &AuditInfo,
    ) -> AppResult<TicketModel> {
        validate_ticket_status(&input.status)?;
        self.repo
            .change_ticket_status(input.ticket_id, &input.status, &input.resolution, audit)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("ticket {} not found", input.ticket_id)))
    }

    pub async fn close_ticket(
        &self,
        ticket_id: Uuid,
        resolution: &str,
        audit: &AuditInfo,
    ) -> AppResult<TicketModel> {
        self.repo
            .close_ticket(ticket_id, resolution, audit)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("ticket {ticket_id} not found")))
    }

    pub async fn list_anomaly_rules(
        &self,
        list: &ListInput,
        filter: &AnomalyRuleListFilter,
    ) -> AppResult<(Vec<AnomalyRuleModel>, PagingResponse)> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let rules = self.repo.list_anomaly_rules().await.map_err(map_db_error)?;
        let filtered = rules
            .into_iter()
            .filter(|rule| match &query {
                Some(q) => rule.name.to_ascii_lowercase().contains(q),
                None => true,
            })
            .filter(|rule| match &filter.scope_type {
                Some(scope) if &rule.scope_type != scope => false,
                _ => true,
            })
            .filter(|rule| match filter.scope_id {
                Some(scope_id) if rule.scope_id != Some(scope_id) => false,
                _ => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_anomaly_rule(&self, rule_id: Uuid) -> AppResult<AnomalyRuleModel> {
        self.repo
            .get_anomaly_rule(rule_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("anomaly rule {rule_id} not found")))
    }

    pub async fn create_anomaly_rule(
        &self,
        input: AnomalyRuleInput,
        audit: &AuditInfo,
    ) -> AppResult<AnomalyRuleModel> {
        validate_anomaly_kind(&input.kind)?;
        validate_scope(&input.scope_type, input.scope_id)?;
        validate_json_object(&input.config, "config_json")?;
        self.repo
            .create_anomaly_rule(
                &input.name,
                &input.kind,
                &input.scope_type,
                input.scope_id,
                &input.config,
                input.is_active,
                audit,
            )
            .await
            .map_err(map_db_error)
    }

    pub async fn update_anomaly_rule(
        &self,
        input: AnomalyRuleUpdateInput,
        audit: &AuditInfo,
    ) -> AppResult<AnomalyRuleModel> {
        validate_json_object(&input.config, "config_json")?;
        self.repo
            .update_anomaly_rule(
                input.anomaly_rule_id,
                &input.name,
                &input.config,
                input.is_active,
                audit,
            )
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| {
                AppError::not_found(format!("anomaly rule {} not found", input.anomaly_rule_id))
            })
    }

    pub async fn list_anomaly_instances(
        &self,
        list: &ListInput,
        filter: &AnomalyInstanceFilter,
    ) -> AppResult<(Vec<AnomalyInstanceModel>, PagingResponse)> {
        let instances = self
            .repo
            .list_anomaly_instances()
            .await
            .map_err(map_db_error)?;
        let filtered = instances
            .into_iter()
            .filter(|instance| match filter.anomaly_rule_id {
                Some(rule_id) if instance.rule_id != rule_id => false,
                _ => true,
            })
            .filter(|instance| match filter.cluster_id {
                Some(cluster_id) if instance.cluster_id != Some(cluster_id) => false,
                _ => true,
            })
            .filter(|instance| match &filter.status {
                Some(status) if &instance.status != status => false,
                _ => true,
            })
            .collect::<Vec<_>>();
        paginate(filtered, list)
    }

    pub async fn get_anomaly_instance(
        &self,
        anomaly_instance_id: Uuid,
    ) -> AppResult<AnomalyInstanceModel> {
        self.repo
            .get_anomaly_instance(anomaly_instance_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "anomaly instance {} not found",
                    anomaly_instance_id
                ))
            })
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
    let object = value
        .as_object()
        .ok_or_else(|| AppError::invalid_argument("policy_body_json must be a JSON object"))?;

    let mut has_supported_section = false;

    if let Some(paths) = object.get("paths") {
        has_supported_section = true;
        validate_string_array(paths, "paths")?;
    }

    if let Some(sources) = object.get("sources") {
        has_supported_section = true;
        let entries = sources
            .as_array()
            .ok_or_else(|| AppError::invalid_argument("policy sources must be an array"))?;
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
    let items = value
        .as_array()
        .ok_or_else(|| AppError::invalid_argument(format!("{field} must be an array")))?;
    for item in items {
        let item = item
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty());
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

fn validate_cluster_fields(name: &str, slug: &str) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_argument("cluster name is required"));
    }
    if slug.trim().is_empty() {
        return Err(AppError::invalid_argument("cluster slug is required"));
    }
    if !slug
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(AppError::invalid_argument(
            "cluster slug must be lowercase alphanumeric with hyphens",
        ));
    }
    Ok(())
}

fn validate_role_fields(name: &str, slug: &str) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_argument("role name is required"));
    }
    if slug.trim().is_empty() {
        return Err(AppError::invalid_argument("role slug is required"));
    }
    if !slug
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(AppError::invalid_argument(
            "role slug must be lowercase alphanumeric with hyphens",
        ));
    }
    Ok(())
}

fn validate_scope(scope_type: &str, scope_id: Option<Uuid>) -> AppResult<()> {
    match scope_type {
        ROLE_SCOPE_GLOBAL if scope_id.is_some() => Err(AppError::invalid_argument(
            "global scope must not include scope_id",
        )),
        ROLE_SCOPE_CLUSTER if scope_id.is_none() => Err(AppError::invalid_argument(
            "cluster scope requires scope_id",
        )),
        ROLE_SCOPE_GLOBAL | ROLE_SCOPE_CLUSTER => Ok(()),
        _ => Err(AppError::invalid_argument("unsupported scope_type")),
    }
}

fn validate_integration_kind(kind: &str) -> AppResult<()> {
    if ALLOWED_INTEGRATION_KINDS
        .iter()
        .any(|allowed| *allowed == kind)
    {
        Ok(())
    } else {
        Err(AppError::invalid_argument(format!(
            "unsupported integration kind {kind}"
        )))
    }
}

fn validate_json_object(value: &Value, field: &str) -> AppResult<()> {
    if value.as_object().is_none() {
        return Err(AppError::invalid_argument(format!(
            "{field} must be a JSON object"
        )));
    }
    Ok(())
}

fn validate_event_types(value: &Value) -> AppResult<()> {
    let array = value
        .as_array()
        .ok_or_else(|| AppError::invalid_argument("event_types_json must be a JSON array"))?;
    for entry in array {
        if entry
            .as_str()
            .map(|item| item.trim().is_empty())
            .unwrap_or(true)
        {
            return Err(AppError::invalid_argument(
                "event_types_json must contain non-empty strings",
            ));
        }
    }
    Ok(())
}

fn validate_severity(severity: &str) -> AppResult<()> {
    if TICKET_SEVERITIES.iter().any(|level| *level == severity) {
        Ok(())
    } else {
        Err(AppError::invalid_argument(format!(
            "unsupported severity {severity}"
        )))
    }
}

fn validate_ticket_input(input: &TicketCreateInput) -> AppResult<()> {
    if input.title.trim().is_empty() {
        return Err(AppError::invalid_argument("ticket title is required"));
    }
    if input.description.trim().is_empty() {
        return Err(AppError::invalid_argument("ticket description is required"));
    }
    validate_severity(&input.severity)?;
    if input.source_type.trim().is_empty() {
        return Err(AppError::invalid_argument("ticket source_type is required"));
    }
    if input.created_by.trim().is_empty() {
        return Err(AppError::invalid_argument("created_by is required"));
    }
    Ok(())
}

fn validate_ticket_status(status: &str) -> AppResult<()> {
    if TICKET_STATUSES.iter().any(|value| *value == status) {
        Ok(())
    } else {
        Err(AppError::invalid_argument(format!(
            "unsupported ticket status {status}"
        )))
    }
}

fn validate_anomaly_kind(kind: &str) -> AppResult<()> {
    if ANOMALY_KINDS.iter().any(|value| *value == kind) {
        Ok(())
    } else {
        Err(AppError::invalid_argument(format!(
            "unsupported anomaly kind {kind}"
        )))
    }
}

fn generate_ticket_key() -> String {
    let raw = Uuid::new_v4().simple().to_string();
    format!("TC-{}", &raw[..8])
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
