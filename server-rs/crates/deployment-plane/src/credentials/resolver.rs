use async_nats::Client;
use common::{
    nats_subjects::CONTROL_CREDENTIALS_GET,
    proto::{
        control::{self},
        decode_message, encode_message, runtime,
    },
    AppError, AppResult,
};
use prost::Message;
use uuid::Uuid;

use crate::models::ResolvedCredentialProfile;

#[derive(Clone)]
pub struct CredentialsResolver {
    nats: Client,
}

impl CredentialsResolver {
    pub fn new(nats: Client) -> Self {
        Self { nats }
    }

    pub async fn resolve(
        &self,
        credential_profile_id: Uuid,
    ) -> AppResult<ResolvedCredentialProfile> {
        let credentials: control::CredentialProfileMetadata = request_control_payload(
            &self.nats,
            CONTROL_CREDENTIALS_GET,
            control::GetCredentialsRequest {
                correlation_id: format!("credentials-{credential_profile_id}"),
                credentials_profile_id: credential_profile_id.to_string(),
            },
        )
        .await?;

        Ok(ResolvedCredentialProfile {
            credential_profile_id: Uuid::parse_str(&credentials.credentials_profile_id).map_err(
                |error| AppError::internal(format!("invalid credentials profile id: {error}")),
            )?,
            name: credentials.name,
            kind: credentials.kind,
            description: credentials.description,
            vault_ref: credentials.vault_ref,
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
    let envelope: runtime::RuntimeReplyEnvelope = decode_message(message.payload.as_ref())?;
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
