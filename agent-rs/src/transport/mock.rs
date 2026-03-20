use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tracing::info;

use crate::{
    error::AppResult,
    proto::{agent, ingest},
    transport::{
        AgentTransport, EnrollRequest, EnrollResponse, FetchPolicyRequest, PolicySnapshot,
    },
};

#[derive(Debug, Default, Clone)]
pub struct MockTransport {
    inner: Arc<Mutex<MockState>>,
}

#[derive(Debug, Default)]
struct MockState {
    next_id: u64,
    heartbeats: usize,
    diagnostics: usize,
    batches: usize,
}

#[cfg(test)]
impl MockTransport {
    pub fn snapshot(&self) -> (usize, usize, usize) {
        let inner = self.inner.lock().expect("mock transport lock poisoned");
        (inner.heartbeats, inner.diagnostics, inner.batches)
    }
}

#[async_trait]
impl AgentTransport for MockTransport {
    async fn enroll(&self, request: EnrollRequest) -> AppResult<EnrollResponse> {
        let mut inner = self.inner.lock().expect("mock transport lock poisoned");
        inner.next_id += 1;
        let agent_id = request
            .existing_agent_id
            .unwrap_or_else(|| format!("mock-agent-{}", inner.next_id));
        info!(agent_id, version = request.version, "mock enroll");
        Ok(EnrollResponse {
            agent_id,
            status: "enrolled".to_string(),
        })
    }

    async fn fetch_policy(&self, request: FetchPolicyRequest) -> AppResult<PolicySnapshot> {
        Ok(PolicySnapshot {
            agent_id: request.agent_id,
            policy_id: "mock-policy".to_string(),
            policy_revision: request
                .current_revision
                .unwrap_or_else(|| "mock-revision-1".to_string()),
            policy_body_json: r#"{"sources":["file"]}"#.to_string(),
            status: "ok".to_string(),
        })
    }

    async fn send_heartbeat(&self, _payload: agent::HeartbeatPayload) -> AppResult<()> {
        let mut inner = self.inner.lock().expect("mock transport lock poisoned");
        inner.heartbeats += 1;
        Ok(())
    }

    async fn send_batch(&self, batch: ingest::LogBatch) -> AppResult<()> {
        let mut inner = self.inner.lock().expect("mock transport lock poisoned");
        inner.batches += 1;
        info!(events = batch.events.len(), "mock batch send");
        Ok(())
    }

    async fn send_diagnostics(&self, _payload: agent::DiagnosticsPayload) -> AppResult<()> {
        let mut inner = self.inner.lock().expect("mock transport lock poisoned");
        inner.diagnostics += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::transport::{AgentTransport, EnrollRequest, FetchPolicyRequest};

    use super::MockTransport;

    #[tokio::test]
    async fn mock_transport_returns_policy_and_agent() {
        let transport = MockTransport::default();
        let enroll = transport
            .enroll(EnrollRequest {
                bootstrap_token: "token".to_string(),
                hostname: "demo-host".to_string(),
                version: "0.1.0".to_string(),
                metadata: BTreeMap::new(),
                existing_agent_id: None,
            })
            .await
            .unwrap();

        let policy = transport
            .fetch_policy(FetchPolicyRequest {
                agent_id: enroll.agent_id.clone(),
                current_revision: None,
            })
            .await
            .unwrap();

        assert!(enroll.agent_id.starts_with("mock-agent-"));
        assert_eq!(policy.agent_id, enroll.agent_id);
        assert_eq!(transport.snapshot(), (0, 0, 0));
    }
}
