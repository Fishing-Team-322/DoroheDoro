pub mod client;
pub mod mock;

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;

use crate::{
    error::AppResult,
    proto::{agent, ingest},
};

pub use client::EdgeGrpcTransport;
pub use mock::MockTransport;

#[derive(Debug, Clone)]
pub struct EnrollRequest {
    pub bootstrap_token: String,
    pub hostname: String,
    pub version: String,
    pub metadata: BTreeMap<String, String>,
    pub existing_agent_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnrollResponse {
    pub agent_id: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct FetchPolicyRequest {
    pub agent_id: String,
    pub current_revision: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PolicySnapshot {
    pub policy_id: String,
    pub policy_revision: String,
    pub policy_body_json: String,
    pub status: String,
}

#[async_trait]
pub trait AgentTransport: Send + Sync {
    async fn enroll(&self, request: EnrollRequest) -> AppResult<EnrollResponse>;
    async fn fetch_policy(&self, request: FetchPolicyRequest) -> AppResult<PolicySnapshot>;
    async fn send_heartbeat(&self, payload: agent::HeartbeatPayload) -> AppResult<()>;
    async fn send_batch(&self, batch: ingest::LogBatch) -> AppResult<()>;
    async fn send_diagnostics(&self, payload: agent::DiagnosticsPayload) -> AppResult<()>;
}

pub type DynTransport = Arc<dyn AgentTransport>;
