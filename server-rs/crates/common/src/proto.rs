use prost::Message;
use serde::Serialize;

use crate::error::{AppError, AppResult};

pub mod agent {
    include!(concat!(env!("OUT_DIR"), "/dorohedoro.agent.v1.rs"));
}

pub mod ingest {
    include!(concat!(env!("OUT_DIR"), "/dorohedoro.v1.rs"));
}

pub mod control {
    include!(concat!(env!("OUT_DIR"), "/dorohedoro.control.v1.rs"));
}

pub mod deployment {
    include!(concat!(env!("OUT_DIR"), "/dorohedoro.deployment.v1.rs"));
}

use self::{
    agent::AgentReplyEnvelope, control::ControlReplyEnvelope, deployment::DeploymentReplyEnvelope,
};

pub fn encode_message<T>(message: &T) -> Vec<u8>
where
    T: Message,
{
    message.encode_to_vec()
}

pub fn decode_message<T>(payload: &[u8]) -> AppResult<T>
where
    T: Message + Default,
{
    T::decode(payload)
        .map_err(|error| AppError::invalid_argument(format!("decode protobuf payload: {error}")))
}

pub fn ok_envelope<T>(payload: &T, correlation_id: impl Into<String>) -> AgentReplyEnvelope
where
    T: Message,
{
    AgentReplyEnvelope {
        status: "ok".to_string(),
        code: "ok".to_string(),
        message: String::new(),
        payload: encode_message(payload),
        correlation_id: correlation_id.into(),
    }
}

pub fn ok_json_envelope<T>(
    payload: &T,
    correlation_id: impl Into<String>,
) -> AppResult<AgentReplyEnvelope>
where
    T: Serialize,
{
    let payload = serde_json::to_vec(payload)
        .map_err(|error| AppError::internal(format!("serialize json payload: {error}")))?;
    Ok(AgentReplyEnvelope {
        status: "ok".to_string(),
        code: "ok".to_string(),
        message: String::new(),
        payload,
        correlation_id: correlation_id.into(),
    })
}

pub fn control_ok_envelope<T>(
    payload: &T,
    correlation_id: impl Into<String>,
) -> ControlReplyEnvelope
where
    T: Message,
{
    ControlReplyEnvelope {
        status: "ok".to_string(),
        code: "ok".to_string(),
        message: String::new(),
        payload: encode_message(payload),
        correlation_id: correlation_id.into(),
    }
}

pub fn deployment_ok_envelope<T>(
    payload: &T,
    correlation_id: impl Into<String>,
) -> DeploymentReplyEnvelope
where
    T: Message,
{
    DeploymentReplyEnvelope {
        status: "ok".to_string(),
        code: "ok".to_string(),
        message: String::new(),
        payload: encode_message(payload),
        correlation_id: correlation_id.into(),
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::{
        agent::FetchPolicyRequest, control::Host, control::ListHostsResponse, control_ok_envelope,
        decode_message, deployment::ListDeploymentJobsResponse, deployment_ok_envelope,
        encode_message, ok_envelope, ok_json_envelope,
    };

    #[test]
    fn encodes_and_decodes_messages() {
        let original = FetchPolicyRequest {
            correlation_id: "corr-1".to_string(),
            agent_id: "agent-1".to_string(),
        };

        let encoded = encode_message(&original);
        let decoded: FetchPolicyRequest = decode_message(&encoded).unwrap();

        assert_eq!(decoded.correlation_id, "corr-1");
        assert_eq!(decoded.agent_id, "agent-1");
    }

    #[test]
    fn wraps_ok_envelope() {
        let payload = FetchPolicyRequest {
            correlation_id: "corr-2".to_string(),
            agent_id: "agent-2".to_string(),
        };

        let envelope = ok_envelope(&payload, "corr-2");
        assert_eq!(envelope.status, "ok");
        assert_eq!(envelope.code, "ok");
        assert_eq!(envelope.correlation_id, "corr-2");
        assert!(!envelope.payload.is_empty());
    }

    #[derive(Serialize)]
    struct JsonPayload {
        status: &'static str,
    }

    #[test]
    fn wraps_ok_json_envelope() {
        let envelope = ok_json_envelope(&JsonPayload { status: "ok" }, "corr-json").unwrap();
        assert_eq!(envelope.status, "ok");
        assert_eq!(envelope.code, "ok");
        assert_eq!(envelope.correlation_id, "corr-json");
        assert_eq!(
            String::from_utf8(envelope.payload).unwrap(),
            "{\"status\":\"ok\"}"
        );
    }

    #[test]
    fn wraps_control_ok_envelope() {
        let payload = ListHostsResponse {
            hosts: vec![Host {
                host_id: "host-1".to_string(),
                hostname: "srv-1".to_string(),
                ip: "10.0.0.1".to_string(),
                ssh_port: 22,
                remote_user: "root".to_string(),
                labels: Default::default(),
                created_at: "2026-03-21T00:00:00Z".to_string(),
                updated_at: "2026-03-21T00:00:00Z".to_string(),
            }],
        };
        let envelope = control_ok_envelope(&payload, "corr-3");
        assert_eq!(envelope.status, "ok");
        assert_eq!(envelope.code, "ok");
        assert_eq!(envelope.correlation_id, "corr-3");
        assert!(!envelope.payload.is_empty());
    }

    #[test]
    fn wraps_deployment_ok_envelope() {
        let payload = ListDeploymentJobsResponse {
            jobs: Vec::new(),
            limit: 10,
            offset: 0,
            total: 0,
        };
        let envelope = deployment_ok_envelope(&payload, "corr-4");
        assert_eq!(envelope.status, "ok");
        assert_eq!(envelope.code, "ok");
        assert_eq!(envelope.correlation_id, "corr-4");
        assert!(!envelope.payload.is_empty());
    }
}
