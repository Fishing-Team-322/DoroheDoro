use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use common::proto::{
    control::{CredentialProfileMetadata, Host, HostGroup, HostGroupMember, Policy, PolicyRevision},
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
