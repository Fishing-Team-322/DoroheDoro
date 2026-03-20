use prost::Message;

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

use self::{agent::AgentReplyEnvelope, control::ControlReplyEnvelope};

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

#[cfg(test)]
mod tests {
    use super::{
        agent::FetchPolicyRequest, control::ListHostsResponse, control_ok_envelope, decode_message,
        encode_message, ok_envelope,
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

    #[test]
    fn wraps_control_ok_envelope() {
        let payload = ListHostsResponse { hosts: Vec::new() };
        let envelope = control_ok_envelope(&payload, "corr-3");
        assert_eq!(envelope.status, "ok");
        assert_eq!(envelope.code, "ok");
        assert_eq!(envelope.correlation_id, "corr-3");
        assert!(!envelope.payload.is_empty());
    }
}
