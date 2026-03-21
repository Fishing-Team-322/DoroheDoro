use std::collections::BTreeMap;

use async_nats::Client;
use common::{
    nats_subjects::{CONTROL_HOSTS_GET, CONTROL_HOST_GROUPS_GET},
    proto::{
        control::{self, ControlReplyEnvelope},
        decode_message, encode_message,
    },
    AppError, AppResult,
};
use prost::Message;
use uuid::Uuid;

use crate::models::ResolvedHost;

#[derive(Clone)]
pub struct InventoryResolver {
    nats: Client,
}

impl InventoryResolver {
    pub fn new(nats: Client) -> Self {
        Self { nats }
    }

    pub async fn resolve(
        &self,
        host_ids: &[Uuid],
        host_group_ids: &[Uuid],
    ) -> AppResult<Vec<ResolvedHost>> {
        let mut resolved = BTreeMap::new();

        for host_id in host_ids {
            let host = self.get_host(*host_id).await?;
            resolved.insert(host.host_id, host);
        }

        for host_group_id in host_group_ids {
            let group = request_control_payload::<_, control::HostGroup>(
                &self.nats,
                CONTROL_HOST_GROUPS_GET,
                control::GetHostGroupRequest {
                    correlation_id: format!("inventory-group-{host_group_id}"),
                    host_group_id: host_group_id.to_string(),
                },
            )
            .await?;

            for member in group.members {
                let host_id = Uuid::parse_str(&member.host_id).map_err(|error| {
                    AppError::internal(format!("invalid host id in host group response: {error}"))
                })?;
                if resolved.contains_key(&host_id) {
                    continue;
                }
                let host = self.get_host(host_id).await?;
                resolved.insert(host.host_id, host);
            }
        }

        if resolved.is_empty() {
            return Err(AppError::invalid_argument(
                "deployment requires at least one target host",
            ));
        }

        Ok(resolved.into_values().collect())
    }

    async fn get_host(&self, host_id: Uuid) -> AppResult<ResolvedHost> {
        let host: control::Host = request_control_payload(
            &self.nats,
            CONTROL_HOSTS_GET,
            control::GetHostRequest {
                correlation_id: format!("inventory-host-{host_id}"),
                host_id: host_id.to_string(),
            },
        )
        .await?;

        Ok(ResolvedHost {
            host_id: Uuid::parse_str(&host.host_id)
                .map_err(|error| AppError::internal(format!("invalid host id: {error}")))?,
            hostname: host.hostname,
            ip: host.ip,
            ssh_port: host.ssh_port as u16,
            remote_user: host.remote_user,
            labels: host.labels.into_iter().collect(),
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
        return Err(map_control_error(&envelope));
    }
    decode_message(&envelope.payload)
}

fn map_control_error(envelope: &ControlReplyEnvelope) -> AppError {
    match envelope.code.as_str() {
        "invalid_argument" => AppError::invalid_argument(envelope.message.clone()),
        "not_found" => AppError::not_found(envelope.message.clone()),
        _ => AppError::internal(format!(
            "control-plane request failed: {} {}",
            envelope.code, envelope.message
        )),
    }
}
