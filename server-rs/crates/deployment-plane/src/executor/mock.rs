use std::{collections::BTreeSet, time::Duration};

use async_trait::async_trait;
use serde_json::json;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use common::AppResult;

use crate::{
    executor::traits::DeploymentExecutor,
    models::{
        DeploymentJobStatus, DeploymentStepStatus, DeploymentTargetStatus, ExecutionOutput,
        ExecutionResult, ExecutionSnapshot, ExecutionTargetResult, ExecutorKind,
        StepExecutionResult,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockFailMode {
    Never,
    Partial,
    All,
}

impl MockFailMode {
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "never" => Some(Self::Never),
            "partial" => Some(Self::Partial),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MockExecutorOptions {
    pub step_delay_ms: u64,
    pub fail_mode: MockFailMode,
    pub fail_hostnames: BTreeSet<String>,
}

impl Default for MockExecutorOptions {
    fn default() -> Self {
        Self {
            step_delay_ms: 5,
            fail_mode: MockFailMode::Never,
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

    fn should_fail_target(&self, index: usize, hostname: &str) -> bool {
        if self.options.fail_hostnames.contains(hostname) {
            return true;
        }

        match self.options.fail_mode {
            MockFailMode::Never => false,
            MockFailMode::Partial => index == 0,
            MockFailMode::All => true,
        }
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

    async fn execute(
        &self,
        snapshot: &ExecutionSnapshot,
        cancellation: &CancellationToken,
    ) -> AppResult<ExecutionResult> {
        let mut targets = Vec::with_capacity(snapshot.targets.len());

        for (index, target) in snapshot.targets.iter().enumerate() {
            let mut steps = Vec::new();
            let mut final_result = None;

            if cancellation.is_cancelled() {
                steps.push(StepExecutionResult {
                    step_name: "mock.cancelled".to_string(),
                    status: DeploymentStepStatus::Skipped,
                    message: "execution cancelled before target started".to_string(),
                    payload_json: json!({
                        "hostname": target.host.hostname,
                        "deployment_target_id": target.deployment_target_id,
                    }),
                });
                targets.push(ExecutionTargetResult {
                    deployment_target_id: target.deployment_target_id,
                    status: DeploymentTargetStatus::Cancelled,
                    error_message: Some("cancelled before execution".to_string()),
                    steps,
                });
                continue;
            }

            let should_fail = self.should_fail_target(index, &target.host.hostname);

            for (step_index, step_name) in ["mock.connect", "mock.install", "mock.verify"]
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
                            "hostname": target.host.hostname,
                            "job_type": snapshot.job_type.as_str(),
                            "step_index": step_index,
                        }),
                    });
                    final_result = Some(ExecutionTargetResult {
                        deployment_target_id: target.deployment_target_id,
                        status: DeploymentTargetStatus::Cancelled,
                        error_message: Some("cancelled during execution".to_string()),
                        steps: steps.clone(),
                    });
                    break;
                }

                let failed_step = should_fail && step_name == "mock.install";
                steps.push(StepExecutionResult {
                    step_name: step_name.to_string(),
                    status: if failed_step {
                        DeploymentStepStatus::Failed
                    } else {
                        DeploymentStepStatus::Succeeded
                    },
                    message: if failed_step {
                        "simulated install failure".to_string()
                    } else {
                        "step completed".to_string()
                    },
                    payload_json: json!({
                        "hostname": target.host.hostname,
                        "job_type": snapshot.job_type.as_str(),
                        "step_index": step_index,
                        "fail_mode": format!("{:?}", self.options.fail_mode).to_ascii_lowercase(),
                    }),
                });

                if failed_step {
                    final_result = Some(ExecutionTargetResult {
                        deployment_target_id: target.deployment_target_id,
                        status: DeploymentTargetStatus::Failed,
                        error_message: Some(format!(
                            "mock executor forced failure for host {}",
                            target.host.hostname
                        )),
                        steps: steps.clone(),
                    });
                    break;
                }
            }

            if let Some(result) = final_result {
                targets.push(result);
                continue;
            }

            targets.push(ExecutionTargetResult {
                deployment_target_id: target.deployment_target_id,
                status: DeploymentTargetStatus::Succeeded,
                error_message: None,
                steps,
            });
        }

        let job_status = derive_job_status(&targets);
        Ok(ExecutionResult {
            current_phase: match job_status {
                DeploymentJobStatus::Succeeded => "mock.completed".to_string(),
                DeploymentJobStatus::PartialSuccess => "mock.partial_failure".to_string(),
                DeploymentJobStatus::Failed => "mock.failed".to_string(),
                DeploymentJobStatus::Cancelled => "mock.cancelled".to_string(),
                DeploymentJobStatus::Queued | DeploymentJobStatus::Running => {
                    "mock.running".to_string()
                }
            },
            targets,
            steps: vec![StepExecutionResult {
                step_name: "mock.executor".to_string(),
                status: match job_status {
                    DeploymentJobStatus::Succeeded | DeploymentJobStatus::PartialSuccess => {
                        DeploymentStepStatus::Succeeded
                    }
                    DeploymentJobStatus::Failed => DeploymentStepStatus::Failed,
                    DeploymentJobStatus::Cancelled => DeploymentStepStatus::Skipped,
                    DeploymentJobStatus::Queued | DeploymentJobStatus::Running => {
                        DeploymentStepStatus::Running
                    }
                },
                message: format!(
                    "mock executor completed with {} targets",
                    snapshot.targets.len()
                ),
                payload_json: json!({
                    "fail_mode": format!("{:?}", self.options.fail_mode).to_ascii_lowercase(),
                    "fail_hostnames": self.options.fail_hostnames,
                    "step_delay_ms": self.options.step_delay_ms,
                }),
            }],
            output: ExecutionOutput {
                exit_code: Some(match job_status {
                    DeploymentJobStatus::Succeeded => 0,
                    DeploymentJobStatus::PartialSuccess => 2,
                    DeploymentJobStatus::Failed => 1,
                    DeploymentJobStatus::Cancelled => 130,
                    DeploymentJobStatus::Queued | DeploymentJobStatus::Running => 0,
                }),
                stdout_ref: None,
                stdout_excerpt: Some(format!(
                    "mock executor processed {} targets",
                    snapshot.targets.len()
                )),
                stderr_ref: None,
                stderr_excerpt: match job_status {
                    DeploymentJobStatus::Succeeded => None,
                    DeploymentJobStatus::PartialSuccess => {
                        Some("mock executor completed with partial failures".to_string())
                    }
                    DeploymentJobStatus::Failed => {
                        Some("mock executor completed with failures".to_string())
                    }
                    DeploymentJobStatus::Cancelled => {
                        Some("mock executor cancelled execution".to_string())
                    }
                    DeploymentJobStatus::Queued | DeploymentJobStatus::Running => None,
                },
            },
        })
    }
}

fn derive_job_status(targets: &[ExecutionTargetResult]) -> DeploymentJobStatus {
    let total = targets.len();
    let succeeded = targets
        .iter()
        .filter(|target| target.status == DeploymentTargetStatus::Succeeded)
        .count();
    let failed = targets
        .iter()
        .filter(|target| target.status == DeploymentTargetStatus::Failed)
        .count();
    let cancelled = targets
        .iter()
        .filter(|target| target.status == DeploymentTargetStatus::Cancelled)
        .count();

    if total == 0 {
        return DeploymentJobStatus::Failed;
    }
    if cancelled == total {
        return DeploymentJobStatus::Cancelled;
    }
    if succeeded == total {
        return DeploymentJobStatus::Succeeded;
    }
    if failed == total {
        return DeploymentJobStatus::Failed;
    }
    if failed > 0 || cancelled > 0 {
        return DeploymentJobStatus::PartialSuccess;
    }
    DeploymentJobStatus::Running
}
