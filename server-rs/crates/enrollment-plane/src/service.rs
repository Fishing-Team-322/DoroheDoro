use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use common::{
    proto::{
        agent::{
            AgentDetail, AgentPolicyBinding, AgentSummary, DiagnosticsPayload, DiagnosticsSnapshot,
            EnrollRequest, EnrollResponse, FetchPolicyRequest, FetchPolicyResponse,
            GetAgentPolicyResponse, HeartbeatPayload, IssueBootstrapTokenRequest,
            IssueBootstrapTokenResponse, ListAgentsResponse,
        },
        runtime::{PagingRequest, PagingResponse},
    },
    AppError, AppResult,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::repository::{AgentRecord, EnrollmentRepository, PolicyRecord, PolicyRevisionRecord};

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
            query: optional_query(paging.query),
        }
    }

    pub fn paging(&self, total: u64) -> PagingResponse {
        PagingResponse {
            limit: self.limit,
            offset: self.offset,
            total,
        }
    }
}

#[derive(Clone)]
pub struct EnrollmentService {
    repo: EnrollmentRepository,
}

impl EnrollmentService {
    pub fn new(repo: EnrollmentRepository) -> Self {
        Self { repo }
    }

    pub async fn bootstrap_defaults(&self, bootstrap_token: &str) -> AppResult<()> {
        let token_hash = hash_token(bootstrap_token);
        self.repo
            .ensure_dev_bootstrap(&token_hash, &default_policy_body())
            .await
            .map_err(|error| AppError::internal(format!("bootstrap defaults: {error}")))
    }

    pub async fn enroll(&self, request: EnrollRequest) -> AppResult<EnrollResponse> {
        if request.bootstrap_token.trim().is_empty() {
            return Err(AppError::invalid_argument("bootstrap_token is required"));
        }

        let token_hash = hash_token(&request.bootstrap_token);
        let policy = self
            .repo
            .token_policy(&token_hash)
            .await
            .map_err(|error| AppError::internal(format!("load policy by token: {error}")))?
            .ok_or_else(|| AppError::unauthenticated("invalid bootstrap token"))?;

        let existing_agent_id = request.existing_agent_id.trim();
        let chosen_agent_id = if !existing_agent_id.is_empty()
            && self
                .repo
                .agent_exists(existing_agent_id)
                .await
                .map_err(|error| AppError::internal(format!("load agent identity: {error}")))?
        {
            existing_agent_id.to_string()
        } else {
            format!("agent-{}", Uuid::new_v4())
        };

        let seen_at = Utc::now();
        let hostname = normalize_hostname(&request.hostname);
        let metadata_json = map_to_json(&request.metadata)?;

        let agent = self
            .repo
            .create_or_update_enrollment(
                &chosen_agent_id,
                &hostname,
                optional_non_empty(&request.version),
                &metadata_json,
                &policy,
                seen_at,
            )
            .await
            .map_err(|error| AppError::internal(format!("persist enrollment: {error}")))?;

        self.repo
            .mark_token_used(&token_hash, seen_at)
            .await
            .map_err(|error| AppError::internal(format!("mark token used: {error}")))?;

        Ok(EnrollResponse {
            agent_id: agent.agent_id,
            policy_id: policy.policy_id.to_string(),
            policy_revision: policy.policy_revision,
            policy_body_json: policy.policy_body_json.to_string(),
            status: "enrolled".to_string(),
            responded_at_unix_ms: seen_at.timestamp_millis(),
        })
    }

    pub async fn fetch_policy(
        &self,
        request: FetchPolicyRequest,
    ) -> AppResult<FetchPolicyResponse> {
        if request.agent_id.trim().is_empty() {
            return Err(AppError::invalid_argument("agent_id is required"));
        }

        let policy = self
            .repo
            .fetch_policy_for_agent(request.agent_id.trim())
            .await
            .map_err(|error| AppError::internal(format!("fetch policy: {error}")))?
            .ok_or_else(|| AppError::not_found("agent or policy binding not found"))?;

        Ok(FetchPolicyResponse {
            agent_id: request.agent_id,
            policy_id: policy.policy_id.to_string(),
            policy_revision: policy.policy_revision,
            policy_body_json: policy.policy_body_json.to_string(),
            status: "ok".to_string(),
            responded_at_unix_ms: Utc::now().timestamp_millis(),
        })
    }

    pub async fn record_heartbeat(&self, payload: HeartbeatPayload) -> AppResult<()> {
        if payload.agent_id.trim().is_empty() {
            return Err(AppError::invalid_argument("agent_id is required"));
        }

        let seen_at = timestamp_or_now(payload.sent_at_unix_ms);
        let metadata_json = map_to_json(&payload.host_metadata)?;
        let updated = self
            .repo
            .record_heartbeat(
                payload.agent_id.trim(),
                optional_non_empty(&payload.hostname),
                optional_non_empty(&payload.version),
                optional_non_empty(&payload.status),
                &metadata_json,
                seen_at,
            )
            .await
            .map_err(|error| AppError::internal(format!("persist heartbeat: {error}")))?;

        if !updated {
            return Err(AppError::not_found("agent not found"));
        }

        Ok(())
    }

    pub async fn record_diagnostics(&self, payload: DiagnosticsPayload) -> AppResult<()> {
        if payload.agent_id.trim().is_empty() {
            return Err(AppError::invalid_argument("agent_id is required"));
        }

        let created_at = timestamp_or_now(payload.sent_at_unix_ms);
        let json_payload = if payload.payload_json.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str::<Value>(&payload.payload_json).map_err(|error| {
                AppError::invalid_argument(format!("payload_json must be valid JSON: {error}"))
            })?
        };

        let inserted = self
            .repo
            .insert_diagnostics(payload.agent_id.trim(), &json_payload, created_at)
            .await
            .map_err(|error| AppError::internal(format!("persist diagnostics: {error}")))?;

        if !inserted {
            return Err(AppError::not_found("agent not found"));
        }

        Ok(())
    }

    pub async fn list_agents(&self, list: &ListInput) -> AppResult<ListAgentsResponse> {
        let query = list.query.as_deref().map(str::to_ascii_lowercase);
        let filtered = self
            .repo
            .list_agents()
            .await
            .map_err(|error| AppError::internal(format!("list agents: {error}")))?
            .into_iter()
            .filter(|agent| match &query {
                Some(query) => agent_matches_query(agent, query),
                None => true,
            })
            .collect::<Vec<_>>();
        let total = filtered.len() as u64;
        let agents = paginate(filtered, list)?
            .into_iter()
            .map(|agent| {
                let effective_policy = binding_from_agent(&agent);
                AgentSummary {
                    agent_id: agent.agent_id,
                    hostname: agent.hostname,
                    version: agent.version.unwrap_or_default(),
                    status: agent.status,
                    last_seen_at: format_ts(agent.last_seen_at),
                    effective_policy,
                }
            })
            .collect();

        Ok(ListAgentsResponse {
            agents,
            paging: Some(list.paging(total)),
        })
    }

    pub async fn get_agent(&self, agent_id: &str) -> AppResult<AgentDetail> {
        let agent_id = agent_id.trim();
        if agent_id.is_empty() {
            return Err(AppError::invalid_argument("agent_id is required"));
        }

        let agent = self
            .repo
            .get_agent(agent_id)
            .await
            .map_err(|error| AppError::internal(format!("get agent: {error}")))?
            .ok_or_else(|| AppError::not_found("agent not found"))?;

        let effective_policy = binding_from_agent(&agent);

        Ok(AgentDetail {
            agent_id: agent.agent_id,
            hostname: agent.hostname,
            version: agent.version.unwrap_or_default(),
            status: agent.status,
            metadata: json_to_map(&agent.metadata_json),
            first_seen_at: format_ts(agent.first_seen_at),
            last_seen_at: format_ts(agent.last_seen_at),
            effective_policy,
        })
    }

    pub async fn latest_diagnostics(&self, agent_id: &str) -> AppResult<DiagnosticsSnapshot> {
        let agent_id = agent_id.trim();
        if agent_id.is_empty() {
            return Err(AppError::invalid_argument("agent_id is required"));
        }

        let diagnostics = self
            .repo
            .latest_diagnostics(agent_id)
            .await
            .map_err(|error| AppError::internal(format!("get latest diagnostics: {error}")))?
            .ok_or_else(|| AppError::not_found("diagnostics not found"))?;

        Ok(DiagnosticsSnapshot {
            agent_id: diagnostics.agent_id,
            payload_json: diagnostics.payload_json.to_string(),
            created_at: format_ts(diagnostics.created_at),
        })
    }

    pub async fn get_agent_policy(&self, agent_id: &str) -> AppResult<GetAgentPolicyResponse> {
        let agent_id = agent_id.trim();
        if agent_id.is_empty() {
            return Err(AppError::invalid_argument("agent_id is required"));
        }

        let agent = self
            .repo
            .get_agent(agent_id)
            .await
            .map_err(|error| AppError::internal(format!("get agent policy: {error}")))?
            .ok_or_else(|| AppError::not_found("agent not found"))?;
        let policy = binding_from_agent(&agent)
            .ok_or_else(|| AppError::not_found("effective policy binding not found"))?;

        Ok(GetAgentPolicyResponse {
            policy: Some(policy),
        })
    }

    pub async fn list_policies(&self) -> AppResult<Vec<PolicyRecord>> {
        self.repo
            .list_policies()
            .await
            .map_err(|error| AppError::internal(format!("list policies: {error}")))
    }

    pub async fn get_policy(&self, policy_id: &str) -> AppResult<PolicyRecord> {
        let policy_id = parse_uuid(policy_id, "policy_id")?;
        self.repo
            .get_policy(policy_id)
            .await
            .map_err(|error| AppError::internal(format!("get policy: {error}")))?
            .ok_or_else(|| AppError::not_found("policy not found"))
    }

    pub async fn list_policy_revisions(
        &self,
        policy_id: &str,
    ) -> AppResult<Vec<PolicyRevisionRecord>> {
        let policy_id = parse_uuid(policy_id, "policy_id")?;
        self.repo
            .list_policy_revisions(policy_id)
            .await
            .map_err(|error| AppError::internal(format!("list policy revisions: {error}")))
    }

    pub async fn issue_bootstrap_token(
        &self,
        request: IssueBootstrapTokenRequest,
    ) -> AppResult<IssueBootstrapTokenResponse> {
        if request.policy_id.trim().is_empty() {
            return Err(AppError::invalid_argument("policy_id is required"));
        }
        if request.policy_revision_id.trim().is_empty() {
            return Err(AppError::invalid_argument("policy_revision_id is required"));
        }

        let policy_id = Uuid::parse_str(request.policy_id.trim())
            .map_err(|error| AppError::invalid_argument(format!("invalid policy_id: {error}")))?;
        let policy_revision_id =
            Uuid::parse_str(request.policy_revision_id.trim()).map_err(|error| {
                AppError::invalid_argument(format!("invalid policy_revision_id: {error}"))
            })?;

        let matches = self
            .repo
            .policy_revision_matches(policy_id, policy_revision_id)
            .await
            .map_err(|error| AppError::internal(format!("validate policy revision: {error}")))?;
        if !matches {
            return Err(AppError::not_found(
                "policy revision does not belong to the policy",
            ));
        }

        let expires_at = timestamp_or_now(request.expires_at_unix_ms);
        if expires_at <= Utc::now() {
            return Err(AppError::invalid_argument(
                "expires_at_unix_ms must be in the future",
            ));
        }

        let raw_token = format!("bootstrap-{}", Uuid::new_v4().simple());
        let issued = self
            .repo
            .issue_bootstrap_token(
                &hash_token(&raw_token),
                policy_id,
                policy_revision_id,
                expires_at,
            )
            .await
            .map_err(|error| AppError::internal(format!("issue bootstrap token: {error}")))?;

        Ok(IssueBootstrapTokenResponse {
            token_id: issued.id.to_string(),
            bootstrap_token: raw_token,
            policy_id: issued.policy_id.to_string(),
            policy_revision_id: issued.policy_revision_id.to_string(),
            expires_at_unix_ms: issued.expires_at.timestamp_millis(),
            created_at_unix_ms: issued.created_at.timestamp_millis(),
        })
    }
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn agent_matches_query(agent: &AgentRecord, query: &str) -> bool {
    agent.agent_id.to_ascii_lowercase().contains(query)
        || agent.hostname.to_ascii_lowercase().contains(query)
        || agent.status.to_ascii_lowercase().contains(query)
        || agent
            .version
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(query)
        || agent
            .policy_revision
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(query)
        || agent
            .policy_name
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(query)
}

fn binding_from_agent(agent: &AgentRecord) -> Option<AgentPolicyBinding> {
    Some(AgentPolicyBinding {
        policy_id: agent.policy_id?.to_string(),
        policy_revision_id: agent.policy_revision_id?.to_string(),
        policy_revision: agent.policy_revision.clone().unwrap_or_default(),
        assigned_at: format_ts(agent.policy_assigned_at?),
        policy_name: agent.policy_name.clone().unwrap_or_default(),
        policy_description: agent.policy_description.clone().unwrap_or_default(),
    })
}

fn json_to_map(value: &Value) -> BTreeMap<String, String> {
    match value {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| (key.clone(), json_value_to_string(value)))
            .collect(),
        _ => BTreeMap::new(),
    }
}

fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

fn optional_query(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn paginate<T>(items: Vec<T>, list: &ListInput) -> AppResult<Vec<T>> {
    let total = items.len() as u64;
    let start = list.offset.min(total) as usize;
    let end = (start as u64 + list.limit as u64).min(total) as usize;
    let mut items = items;
    Ok(items.drain(start..end).collect())
}

fn normalize_hostname(hostname: &str) -> String {
    let trimmed = hostname.trim();
    if trimmed.is_empty() {
        "unknown-host".to_string()
    } else {
        trimmed.to_string()
    }
}

fn optional_non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn timestamp_or_now(timestamp_unix_ms: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp_millis(timestamp_unix_ms).unwrap_or_else(Utc::now)
}

fn map_to_json(map: &BTreeMap<String, String>) -> AppResult<Value> {
    serde_json::to_value(map)
        .map_err(|error| AppError::internal(format!("serialize metadata: {error}")))
}

fn default_policy_body() -> Value {
    json!({
        "revision": "rev-1",
        "sources": ["/var/log/*.log", "journald"],
        "labels": {
            "env": "dev",
            "plane": "data"
        },
        "batch_size": 100,
        "batch_wait": "5s",
        "source_type": "file"
    })
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn parse_uuid(value: &str, field: &str) -> AppResult<Uuid> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::invalid_argument(format!("{field} is required")));
    }
    Uuid::parse_str(value).map_err(|error| {
        AppError::invalid_argument(format!("{field} must be a valid UUID: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::{hash_token, parse_uuid};

    #[test]
    fn hashes_bootstrap_token() {
        assert_eq!(
            hash_token("dev-bootstrap-token"),
            "7c3c6cefa0df4881d3702d011bbbcfbee7a297b87b58bb0a5c4f8f17366b62a6"
        );
    }

    #[test]
    fn rejects_invalid_policy_uuid() {
        let error = parse_uuid("nope", "policy_id").unwrap_err();
        assert_eq!(error.code().as_str(), "invalid_argument");
    }
}
