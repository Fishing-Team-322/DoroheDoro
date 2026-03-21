use async_nats::Client;
use common::{
    nats_subjects::CONTROL_POLICIES_GET,
    proto::{
        control::{self, ControlReplyEnvelope},
        decode_message, encode_message,
    },
    AppError, AppResult,
};
use prost::Message;
use uuid::Uuid;

use crate::models::ResolvedPolicy;

#[derive(Clone)]
pub struct PolicyResolver {
    nats: Client,
}

impl PolicyResolver {
    pub fn new(nats: Client) -> Self {
        Self { nats }
    }

    pub async fn resolve(&self, policy_id: Uuid) -> AppResult<ResolvedPolicy> {
        let policy: control::Policy = request_control_payload(
            &self.nats,
            CONTROL_POLICIES_GET,
            control::GetPolicyRequest {
                correlation_id: format!("policy-{policy_id}"),
                policy_id: policy_id.to_string(),
            },
        )
        .await?;

        let policy_body_json = serde_json::from_str(&policy.policy_body_json).map_err(|error| {
            AppError::internal(format!(
                "policy body json is invalid for deployment: {error}"
            ))
        })?;

        Ok(ResolvedPolicy {
            policy_id: Uuid::parse_str(&policy.policy_id)
                .map_err(|error| AppError::internal(format!("invalid policy id: {error}")))?,
            policy_revision_id: Uuid::parse_str(&policy.latest_revision_id).map_err(|error| {
                AppError::internal(format!("invalid policy revision id: {error}"))
            })?,
            policy_revision: policy.latest_revision,
            policy_body_json,
        })
    }
}

async fn request_control_payload<Req, Resp>(
    client: &Client,
    subject: &str,
    request: Req,
) -> AppResult<Resp>
where
    Req: Message,
    Resp: Message + Default,
{
    let message = client
        .request(subject.to_string(), encode_message(&request).into())
        .await
        .map_err(|error| AppError::internal(format!("request {subject}: {error}")))?;
    let envelope: ControlReplyEnvelope = decode_message(message.payload.as_ref())?;
    if envelope.status != "ok" {
        return Err(match envelope.code.as_str() {
            "invalid_argument" => AppError::invalid_argument(envelope.message.clone()),
            "not_found" => AppError::not_found(envelope.message.clone()),
            _ => AppError::internal(format!(
                "control-plane request failed: {} {}",
                envelope.code, envelope.message
            )),
        });
    }
    decode_message(&envelope.payload)
}
