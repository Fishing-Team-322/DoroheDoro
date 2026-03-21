use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use common::proto::{
    control::{
        AnomalyInstance, AnomalyRule, Cluster, ClusterAgentBinding, ClusterDetails,
        ClusterHostBinding, CredentialProfileMetadata, Host, HostGroup, HostGroupMember,
        Integration, IntegrationBinding, Permission as PermissionProto, Policy, PolicyRevision,
        Role as RoleProto, RoleBinding as RoleBindingProto, Ticket as TicketProto,
        TicketComment as TicketCommentProto, TicketDetails, TicketEvent as TicketEventProto,
    },
    runtime,
};
use serde_json::Value;
use uuid::Uuid;

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn json_to_map(value: Value) -> BTreeMap<String, String> {
    match value {
        Value::Object(map) => map
            .into_iter()
            .map(|(key, val)| (key, json_value_to_string(val)))
            .collect(),
        _ => BTreeMap::new(),
    }
}

fn json_value_to_string(value: Value) -> String {
    match value {
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct PolicyModel {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub latest_revision_id: Uuid,
    pub latest_revision: String,
    pub policy_body_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
    pub update_reason: String,
}

impl PolicyModel {
    pub fn into_proto(self) -> Policy {
        Policy {
            policy_id: self.id.to_string(),
            name: self.name,
            description: self.description,
            is_active: self.is_active,
            latest_revision_id: self.latest_revision_id.to_string(),
            latest_revision: self.latest_revision,
            policy_body_json: self.policy_body_json.to_string(),
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
            update_reason: self.update_reason,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolicyRevisionModel {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub revision: String,
    pub body_json: Value,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub reason: String,
    pub request_id: String,
}

impl PolicyRevisionModel {
    pub fn into_proto(self) -> PolicyRevision {
        PolicyRevision {
            policy_revision_id: self.id.to_string(),
            policy_id: self.policy_id.to_string(),
            revision: self.revision,
            policy_body_json: self.body_json.to_string(),
            created_at: format_ts(self.created_at),
            created_by: self.created_by,
            reason: self.reason,
            request_id: self.request_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostModel {
    pub id: Uuid,
    pub hostname: String,
    pub ip: String,
    pub ssh_port: i32,
    pub remote_user: String,
    pub labels_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
    pub update_reason: String,
}

impl HostModel {
    pub fn into_proto(self) -> Host {
        Host {
            host_id: self.id.to_string(),
            hostname: self.hostname,
            ip: self.ip,
            ssh_port: self.ssh_port as u32,
            remote_user: self.remote_user,
            labels: json_to_map(self.labels_json),
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
            update_reason: self.update_reason,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostGroupMemberModel {
    pub id: Uuid,
    pub host_group_id: Uuid,
    pub host_id: Uuid,
    pub hostname: Option<String>,
}

impl HostGroupMemberModel {
    pub fn into_proto(self) -> HostGroupMember {
        HostGroupMember {
            host_group_member_id: self.id.to_string(),
            host_group_id: self.host_group_id.to_string(),
            host_id: self.host_id.to_string(),
            hostname: self.hostname.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostGroupModel {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
    pub update_reason: String,
    pub members: Vec<HostGroupMemberModel>,
}

impl HostGroupModel {
    pub fn into_proto(mut self) -> HostGroup {
        HostGroup {
            host_group_id: self.id.to_string(),
            name: self.name,
            description: self.description,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
            update_reason: self.update_reason,
            members: self
                .members
                .drain(..)
                .map(HostGroupMemberModel::into_proto)
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CredentialProfileModel {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub description: String,
    pub vault_ref: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
    pub update_reason: String,
}

impl CredentialProfileModel {
    pub fn into_proto(self) -> CredentialProfileMetadata {
        CredentialProfileMetadata {
            credentials_profile_id: self.id.to_string(),
            name: self.name,
            kind: self.kind,
            description: self.description,
            vault_ref: self.vault_ref,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
            update_reason: self.update_reason,
        }
    }
}

pub fn paging_response(limit: u32, offset: u64, total: u64) -> runtime::PagingResponse {
    runtime::PagingResponse {
        limit,
        offset,
        total,
    }
}

#[derive(Debug, Clone)]
pub struct ClusterModel {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
    pub metadata_json: Value,
    pub host_count: i64,
    pub agent_count: i64,
}

impl ClusterModel {
    pub fn into_proto(self) -> Cluster {
        Cluster {
            cluster_id: self.id.to_string(),
            name: self.name,
            slug: self.slug,
            description: self.description,
            is_active: self.is_active,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
            metadata_json: self.metadata_json.to_string(),
            host_count: clamp_count(self.host_count),
            agent_count: clamp_count(self.agent_count),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClusterHostBindingModel {
    pub id: Uuid,
    pub cluster_id: Uuid,
    pub host_id: Uuid,
    pub hostname: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ClusterHostBindingModel {
    pub fn into_proto(self) -> ClusterHostBinding {
        ClusterHostBinding {
            cluster_host_id: self.id.to_string(),
            host_id: self.host_id.to_string(),
            hostname: self.hostname.unwrap_or_default(),
            created_at: format_ts(self.created_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClusterAgentBindingModel {
    pub id: Uuid,
    pub cluster_id: Uuid,
    pub agent_id: String,
    pub created_at: DateTime<Utc>,
}

impl ClusterAgentBindingModel {
    pub fn into_proto(self) -> ClusterAgentBinding {
        ClusterAgentBinding {
            cluster_agent_id: self.id.to_string(),
            agent_id: self.agent_id,
            created_at: format_ts(self.created_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClusterDetailsModel {
    pub cluster: ClusterModel,
    pub hosts: Vec<ClusterHostBindingModel>,
    pub agents: Vec<ClusterAgentBindingModel>,
}

impl ClusterDetailsModel {
    pub fn into_proto(self) -> ClusterDetails {
        ClusterDetails {
            cluster: Some(self.cluster.into_proto()),
            hosts: self.hosts.into_iter().map(|h| h.into_proto()).collect(),
            agents: self.agents.into_iter().map(|a| a.into_proto()).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoleModel {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
}

impl RoleModel {
    pub fn into_proto(self) -> RoleProto {
        RoleProto {
            role_id: self.id.to_string(),
            name: self.name,
            slug: self.slug,
            description: self.description,
            is_system: self.is_system,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PermissionModel {
    pub id: Uuid,
    pub code: String,
    pub description: String,
}

impl PermissionModel {
    pub fn into_proto(self) -> PermissionProto {
        PermissionProto {
            permission_id: self.id.to_string(),
            code: self.code,
            description: self.description,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoleBindingModel {
    pub id: Uuid,
    pub user_id: String,
    pub role_id: Uuid,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl RoleBindingModel {
    pub fn into_proto(self) -> RoleBindingProto {
        RoleBindingProto {
            role_binding_id: self.id.to_string(),
            user_id: self.user_id,
            role_id: self.role_id.to_string(),
            scope_type: self.scope_type,
            scope_id: self.scope_id.map(|id| id.to_string()).unwrap_or_default(),
            created_at: format_ts(self.created_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntegrationModel {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub description: String,
    pub config_json: Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
}

impl IntegrationModel {
    pub fn into_proto(self) -> Integration {
        Integration {
            integration_id: self.id.to_string(),
            name: self.name,
            kind: self.kind,
            description: self.description,
            config_json: self.config_json.to_string(),
            is_active: self.is_active,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntegrationBindingModel {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub event_types_json: Value,
    pub severity_threshold: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl IntegrationBindingModel {
    pub fn into_proto(self) -> IntegrationBinding {
        IntegrationBinding {
            integration_binding_id: self.id.to_string(),
            integration_id: self.integration_id.to_string(),
            scope_type: self.scope_type,
            scope_id: self.scope_id.map(|id| id.to_string()).unwrap_or_default(),
            event_types_json: self.event_types_json.to_string(),
            severity_threshold: self.severity_threshold,
            is_active: self.is_active,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TicketModel {
    pub id: Uuid,
    pub ticket_key: String,
    pub title: String,
    pub description: String,
    pub cluster_id: Uuid,
    pub cluster_name: Option<String>,
    pub source_type: String,
    pub source_id: Option<String>,
    pub severity: String,
    pub status: String,
    pub assignee_user_id: Option<String>,
    pub created_by: String,
    pub resolution: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl TicketModel {
    pub fn into_proto(self) -> TicketProto {
        TicketProto {
            ticket_id: self.id.to_string(),
            ticket_key: self.ticket_key,
            title: self.title,
            description: self.description,
            cluster_id: self.cluster_id.to_string(),
            cluster_name: self.cluster_name.unwrap_or_default(),
            source_type: self.source_type,
            source_id: self.source_id.unwrap_or_default(),
            severity: self.severity,
            status: self.status,
            assignee_user_id: self.assignee_user_id.unwrap_or_default(),
            created_by: self.created_by,
            resolution: self.resolution,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            resolved_at: format_opt_ts(self.resolved_at),
            closed_at: format_opt_ts(self.closed_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TicketCommentModel {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub author_user_id: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl TicketCommentModel {
    pub fn into_proto(self) -> TicketCommentProto {
        TicketCommentProto {
            ticket_comment_id: self.id.to_string(),
            ticket_id: self.ticket_id.to_string(),
            author_user_id: self.author_user_id,
            body: self.body,
            created_at: format_ts(self.created_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TicketEventModel {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub event_type: String,
    pub payload_json: Value,
    pub created_at: DateTime<Utc>,
}

impl TicketEventModel {
    pub fn into_proto(self) -> TicketEventProto {
        TicketEventProto {
            ticket_event_id: self.id.to_string(),
            ticket_id: self.ticket_id.to_string(),
            event_type: self.event_type,
            payload_json: self.payload_json.to_string(),
            created_at: format_ts(self.created_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TicketDetailsModel {
    pub ticket: TicketModel,
    pub comments: Vec<TicketCommentModel>,
    pub events: Vec<TicketEventModel>,
}

impl TicketDetailsModel {
    pub fn into_proto(self) -> TicketDetails {
        TicketDetails {
            ticket: Some(self.ticket.into_proto()),
            comments: self.comments.into_iter().map(|c| c.into_proto()).collect(),
            events: self.events.into_iter().map(|e| e.into_proto()).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnomalyRuleModel {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub config_json: Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
}

impl AnomalyRuleModel {
    pub fn into_proto(self) -> AnomalyRule {
        AnomalyRule {
            anomaly_rule_id: self.id.to_string(),
            name: self.name,
            kind: self.kind,
            scope_type: self.scope_type,
            scope_id: self.scope_id.map(|id| id.to_string()).unwrap_or_default(),
            config_json: self.config_json.to_string(),
            is_active: self.is_active,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
            created_by: self.created_by,
            updated_by: self.updated_by,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnomalyInstanceModel {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub severity: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub payload_json: Value,
}

impl AnomalyInstanceModel {
    pub fn into_proto(self) -> AnomalyInstance {
        AnomalyInstance {
            anomaly_instance_id: self.id.to_string(),
            anomaly_rule_id: self.rule_id.to_string(),
            cluster_id: self.cluster_id.map(|id| id.to_string()).unwrap_or_default(),
            severity: self.severity,
            status: self.status,
            started_at: format_ts(self.started_at),
            resolved_at: format_opt_ts(self.resolved_at),
            payload_json: self.payload_json.to_string(),
        }
    }
}

fn format_opt_ts(ts: Option<DateTime<Utc>>) -> String {
    ts.map(format_ts).unwrap_or_default()
}

fn clamp_count(value: i64) -> u32 {
    if value <= 0 {
        0
    } else if value >= u32::MAX as i64 {
        u32::MAX
    } else {
        value as u32
    }
}
