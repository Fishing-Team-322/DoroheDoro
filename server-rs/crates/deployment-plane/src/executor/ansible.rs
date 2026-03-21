use std::{
    fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use common::{AppError, AppResult};

use crate::{
    executor::traits::DeploymentExecutor,
    models::{
        DeploymentStepStatus, DeploymentTargetStatus, ExecutionOutput, ExecutionResult,
        ExecutionSnapshot, ExecutionTargetResult, ExecutorKind, StepExecutionResult,
    },
};

#[derive(Debug, Clone)]
pub struct AnsibleRunnerExecutor {
    runner_bin: PathBuf,
    playbook_path: PathBuf,
    temp_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct RenderedTargetWorkspace {
    deployment_target_id: Uuid,
    hostname: String,
    vars_path: PathBuf,
    bootstrap_path: PathBuf,
}

#[derive(Debug, Clone)]
struct PreparedWorkspace {
    workspace_dir: PathBuf,
    inventory_path: PathBuf,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    rendered_targets: Vec<RenderedTargetWorkspace>,
}

impl AnsibleRunnerExecutor {
    pub fn new(
        runner_bin: impl Into<PathBuf>,
        playbook_path: impl Into<PathBuf>,
        temp_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            runner_bin: runner_bin.into(),
            playbook_path: playbook_path.into(),
            temp_dir: temp_dir.into(),
        }
    }

    fn ensure_exists(path: &Path, label: &str) -> AppResult<()> {
        if path.exists() {
            Ok(())
        } else {
            Err(AppError::internal(format!(
                "{label} does not exist: {}",
                path.display()
            )))
        }
    }

    fn prepare_workspace(&self, snapshot: &ExecutionSnapshot) -> AppResult<PreparedWorkspace> {
        let workspace_dir = self.temp_dir.join(format!(
            "deployment-{}",
            snapshot.deployment_attempt_id.simple()
        ));
        fs::create_dir_all(&workspace_dir).map_err(|error| {
            AppError::internal(format!(
                "create ansible workspace {}: {error}",
                workspace_dir.display()
            ))
        })?;

        let inventory_path = workspace_dir.join("inventory.ini");
        let inventory = render_inventory(snapshot);
        fs::write(&inventory_path, inventory).map_err(|error| {
            AppError::internal(format!(
                "write inventory {}: {error}",
                inventory_path.display()
            ))
        })?;

        let vars_dir = workspace_dir.join("vars");
        let bootstrap_dir = workspace_dir.join("bootstrap");
        fs::create_dir_all(&vars_dir).map_err(|error| {
            AppError::internal(format!("create vars dir {}: {error}", vars_dir.display()))
        })?;
        fs::create_dir_all(&bootstrap_dir).map_err(|error| {
            AppError::internal(format!(
                "create bootstrap dir {}: {error}",
                bootstrap_dir.display()
            ))
        })?;

        let mut rendered_targets = Vec::with_capacity(snapshot.targets.len());
        for target in &snapshot.targets {
            let vars_path = vars_dir.join(format!("{}.json", target.host.hostname));
            fs::write(&vars_path, target.rendered_vars.to_string()).map_err(|error| {
                AppError::internal(format!("write vars {}: {error}", vars_path.display()))
            })?;

            let bootstrap_path = bootstrap_dir.join(format!("{}.yaml", target.host.hostname));
            fs::write(&bootstrap_path, &target.bootstrap.bootstrap_yaml).map_err(|error| {
                AppError::internal(format!(
                    "write bootstrap {}: {error}",
                    bootstrap_path.display()
                ))
            })?;

            rendered_targets.push(RenderedTargetWorkspace {
                deployment_target_id: target.deployment_target_id,
                hostname: target.host.hostname.clone(),
                vars_path,
                bootstrap_path,
            });
        }

        let stdout_path = workspace_dir.join("ansible.stdout.log");
        let stderr_path = workspace_dir.join("ansible.stderr.log");
        fs::write(&stdout_path, "").map_err(|error| {
            AppError::internal(format!(
                "write stdout log {}: {error}",
                stdout_path.display()
            ))
        })?;
        fs::write(
            &stderr_path,
            "ansible runner execution is not implemented yet; render outputs are prepared\n",
        )
        .map_err(|error| {
            AppError::internal(format!(
                "write stderr log {}: {error}",
                stderr_path.display()
            ))
        })?;

        Ok(PreparedWorkspace {
            workspace_dir,
            inventory_path,
            stdout_path,
            stderr_path,
            rendered_targets,
        })
    }
}

#[async_trait]
impl DeploymentExecutor for AnsibleRunnerExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Ansible
    }

    async fn readiness_check(&self) -> AppResult<()> {
        Self::ensure_exists(&self.runner_bin, "ansible runner binary")?;
        Self::ensure_exists(&self.playbook_path, "ansible playbook")?;
        fs::create_dir_all(&self.temp_dir).map_err(|error| {
            AppError::internal(format!(
                "create deployment temp dir {}: {error}",
                self.temp_dir.display()
            ))
        })?;
        Ok(())
    }

    async fn execute(
        &self,
        snapshot: &ExecutionSnapshot,
        cancellation: &CancellationToken,
    ) -> AppResult<ExecutionResult> {
        if cancellation.is_cancelled() {
            return Ok(ExecutionResult {
                current_phase: "ansible.cancelled".to_string(),
                targets: snapshot
                    .targets
                    .iter()
                    .map(|target| ExecutionTargetResult {
                        deployment_target_id: target.deployment_target_id,
                        status: DeploymentTargetStatus::Cancelled,
                        error_message: Some("cancelled before ansible render".to_string()),
                        steps: vec![],
                    })
                    .collect(),
                steps: vec![],
                output: ExecutionOutput {
                    exit_code: Some(130),
                    stdout_ref: None,
                    stdout_excerpt: None,
                    stderr_ref: None,
                    stderr_excerpt: Some("ansible execution cancelled before render".to_string()),
                },
            });
        }

        let workspace = self.prepare_workspace(snapshot)?;
        let rendered_targets = workspace
            .rendered_targets
            .iter()
            .map(|target| ExecutionTargetResult {
                deployment_target_id: target.deployment_target_id,
                status: DeploymentTargetStatus::Failed,
                error_message: Some("ansible runner execution is not implemented yet".to_string()),
                steps: vec![
                    StepExecutionResult {
                        step_name: "ansible.rendered".to_string(),
                        status: DeploymentStepStatus::Succeeded,
                        message: "inventory, vars and bootstrap artifacts rendered".to_string(),
                        payload_json: json!({
                            "hostname": target.hostname,
                            "vars_path": target.vars_path,
                            "bootstrap_path": target.bootstrap_path,
                        }),
                    },
                    StepExecutionResult {
                        step_name: "ansible.runner.pending".to_string(),
                        status: DeploymentStepStatus::Failed,
                        message: "ansible runner execution is not implemented yet".to_string(),
                        payload_json: json!({
                            "runner_bin": self.runner_bin,
                            "playbook_path": self.playbook_path,
                        }),
                    },
                ],
            })
            .collect::<Vec<_>>();

        Ok(ExecutionResult {
            current_phase: "ansible.rendered".to_string(),
            targets: rendered_targets,
            steps: vec![StepExecutionResult {
                step_name: "ansible.workspace.prepared".to_string(),
                status: DeploymentStepStatus::Succeeded,
                message: "ansible workspace prepared for next execution stage".to_string(),
                payload_json: json!({
                    "workspace_dir": workspace.workspace_dir,
                    "inventory_path": workspace.inventory_path,
                    "runner_bin": self.runner_bin,
                    "playbook_path": self.playbook_path,
                    "stdout_path": workspace.stdout_path,
                    "stderr_path": workspace.stderr_path,
                    "targets": workspace.rendered_targets.iter().map(|target| json!({
                        "deployment_target_id": target.deployment_target_id,
                        "hostname": target.hostname,
                        "vars_path": target.vars_path,
                        "bootstrap_path": target.bootstrap_path,
                    })).collect::<Vec<_>>(),
                }),
            }],
            output: ExecutionOutput {
                exit_code: None,
                stdout_ref: Some(workspace.stdout_path.display().to_string()),
                stdout_excerpt: Some("ansible runner not invoked; workspace prepared".to_string()),
                stderr_ref: Some(workspace.stderr_path.display().to_string()),
                stderr_excerpt: Some(
                    "ansible runner execution is not implemented yet; render outputs are prepared"
                        .to_string(),
                ),
            },
        })
    }
}

fn render_inventory(snapshot: &ExecutionSnapshot) -> String {
    let mut inventory = String::from("[targets]\n");
    for target in &snapshot.targets {
        inventory.push_str(&format!(
            "{} ansible_host={} ansible_port={} ansible_user={}\n",
            target.host.hostname, target.host.ip, target.host.ssh_port, target.host.remote_user
        ));
    }
    inventory
}
