use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use common::AppResult;

use crate::models::{
    ExecutionResult, ExecutionSnapshot, ExecutorKind,
};

#[async_trait]
pub trait DeploymentExecutor: Send + Sync {
    fn kind(&self) -> ExecutorKind;

    async fn readiness_check(&self) -> AppResult<()>;

    async fn execute(
        &self,
        snapshot: &ExecutionSnapshot,
        cancellation: &CancellationToken,
    ) -> AppResult<ExecutionResult>;

    async fn cancel(&self, _job_id: Uuid) -> AppResult<()> {
        Ok(())
    }
}

pub type DynDeploymentExecutor = Arc<dyn DeploymentExecutor>;
