use std::{collections::BTreeSet, time::Duration};

use async_trait::async_trait;
use serde_json::json;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use common::AppResult;

use crate::{
    executor::traits::DeploymentExecutor,
    models::{
        DeploymentSnapshot, DeploymentStepStatus, DeploymentTargetSnapshot, DeploymentTargetStatus,
        ExecutorKind, StepExecutionResult, TargetExecutionResult,
    },
};

#[derive(Debug, Clone)]
pub struct MockExecutorOptions {
    pub step_delay_ms: u64,
    pub fail_hostnames: BTreeSet<String>,
}

impl Default for MockExecutorOptions {
    fn default() -> Self {
        Self {
            step_delay_ms: 5,
            fail_hostnames: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MockExecutor {
    options: MockExecutorOptions,
}

impl MockExecutor {
    pub fn new(options: MockExecutorOptions) -> Self {
        Self { options }
    }
}

#[async_trait]
impl DeploymentExecutor for MockExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Mock
    }

    async fn readiness_check(&self) -> AppResult<()> {
        Ok(())
    }

    async fn execute_target(
        &self,
        snapshot: &DeploymentSnapshot,
        target: &DeploymentTargetSnapshot,
        cancellation: &CancellationToken,
    ) -> AppResult<TargetExecutionResult> {
        let mut steps = Vec::new();

        if cancellation.is_cancelled() {
            steps.push(StepExecutionResult {
                step_name: "mock.cancelled".to_string(),
                status: DeploymentStepStatus::Skipped,
                message: "execution cancelled before target started".to_string(),
                payload_json: json!({ "hostname": target.host.hostname }),
            });
            return Ok(TargetExecutionResult {
                status: DeploymentTargetStatus::Cancelled,
                error_message: Some("cancelled before execution".to_string()),
                steps,
            });
        }

        for (idx, step_name) in ["mock.connect", "mock.install", "mock.verify"]
            .into_iter()
            .enumerate()
        {
            sleep(Duration::from_millis(self.options.step_delay_ms)).await;
            if cancellation.is_cancelled() {
                steps.push(StepExecutionResult {
                    step_name: step_name.to_string(),
                    status: DeploymentStepStatus::Skipped,
                    message: "execution cancelled".to_string(),
                    payload_json: json!({
                        "job_type": snapshot.job_type.as_str(),
                        "hostname": target.host.hostname,
                        "step_index": idx,
                    }),
                });
                return Ok(TargetExecutionResult {
                    status: DeploymentTargetStatus::Cancelled,
                    error_message: Some("cancelled during execution".to_string()),
                    steps,
                });
            }

            let should_fail = step_name == "mock.install"
                && self.options.fail_hostnames.contains(&target.host.hostname);
            let status = if should_fail {
                DeploymentStepStatus::Failed
            } else {
                DeploymentStepStatus::Succeeded
            };
            let message = if should_fail {
                "simulated install failure"
            } else {
                "step completed"
            };

            steps.push(StepExecutionResult {
                step_name: step_name.to_string(),
                status,
                message: message.to_string(),
                payload_json: json!({
                    "hostname": target.host.hostname,
                    "job_type": snapshot.job_type.as_str(),
                    "step_index": idx,
                }),
            });

            if should_fail {
                return Ok(TargetExecutionResult {
                    status: DeploymentTargetStatus::Failed,
                    error_message: Some(format!(
                        "mock executor forced failure for host {}",
                        target.host.hostname
                    )),
                    steps,
                });
            }
        }

        Ok(TargetExecutionResult {
            status: DeploymentTargetStatus::Succeeded,
            error_message: None,
            steps,
        })
    }
}
