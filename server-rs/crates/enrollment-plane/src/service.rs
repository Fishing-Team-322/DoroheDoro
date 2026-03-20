use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use common::{
    proto::agent::{
        DiagnosticsPayload, EnrollRequest, EnrollResponse, FetchPolicyRequest, FetchPolicyResponse,
        HeartbeatPayload,
    },
    AppError, AppResult,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::repository::EnrollmentRepository;

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

#[cfg(test)]
mod tests {
    use super::hash_token;

    #[test]
    fn hashes_bootstrap_token() {
        assert_eq!(
            hash_token("dev-bootstrap-token"),
            "7c3c6cefa0df4881d3702d011bbbcfbee7a297b87b58bb0a5c4f8f17366b62a6"
        );
    }
}
