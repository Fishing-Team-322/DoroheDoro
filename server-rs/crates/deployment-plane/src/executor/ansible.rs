use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    time::SystemTime,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{io::AsyncReadExt, process::Command};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use common::{AppError, AppResult};

use crate::{
    config::{AgentTlsMaterialConfig, VaultRuntimeConfig},
    executor::traits::DeploymentExecutor,
    models::{
        DeploymentStepStatus, DeploymentTargetStatus, ExecutionOutput, ExecutionResult,
        ExecutionSnapshot, ExecutionTarget, ExecutionTargetResult, ExecutorKind,
        StepExecutionResult,
    },
    vault,
};

#[derive(Debug, Clone)]
pub struct AnsibleRunnerExecutor {
    runner_bin: PathBuf,
    playbook_path: PathBuf,
    temp_dir: PathBuf,
    successful_workspace_retention: usize,
    vault_config: VaultRuntimeConfig,
    agent_tls_material: AgentTlsMaterialConfig,
}

#[derive(Debug, Clone)]
struct PreparedTargetWorkspace {
    deployment_target_id: Uuid,
    private_data_dir: PathBuf,
    inventory_path: PathBuf,
    extravars_path: PathBuf,
    bootstrap_path: PathBuf,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

#[derive(Debug, Clone)]
struct PreparedWorkspace {
    workspace_dir: PathBuf,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    targets: Vec<PreparedTargetWorkspace>,
}

#[derive(Debug, Clone)]
struct RuntimeSecrets {
    ssh_user: Option<String>,
    ssh_private_key: Option<String>,
    ssh_password: Option<String>,
    ssh_private_key_passphrase: Option<String>,
    tls_ca_pem: Option<String>,
    tls_cert_pem: Option<String>,
    tls_key_pem: Option<String>,
}

const WORKSPACE_METADATA_FILE: &str = ".doro-workspace.json";
const STEP_MARKER_PREFIX: &str = "DORO_STEP|";
const RUNTIME_STEP_ORDER: &[&str] = &[
    "precheck_runtime",
    "connect_host",
    "pull_image",
    "render_runner",
    "render_unit",
    "restart_service",
    "run_health_check",
    "persist_last_known_good",
    "rollback_previous_image",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum WorkspaceOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceMetadata {
    outcome: WorkspaceOutcome,
}

impl AnsibleRunnerExecutor {
    pub fn new(
        runner_bin: impl Into<PathBuf>,
        playbook_path: impl Into<PathBuf>,
        temp_dir: impl Into<PathBuf>,
        successful_workspace_retention: usize,
        vault_config: VaultRuntimeConfig,
        agent_tls_material: AgentTlsMaterialConfig,
    ) -> Self {
        Self {
            runner_bin: runner_bin.into(),
            playbook_path: playbook_path.into(),
            temp_dir: temp_dir.into(),
            successful_workspace_retention,
            vault_config,
            agent_tls_material,
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

    fn roles_dir(&self) -> AppResult<PathBuf> {
        let playbook_dir = self.playbook_path.parent().ok_or_else(|| {
            AppError::internal(format!(
                "ansible playbook `{}` has no parent directory",
                self.playbook_path.display()
            ))
        })?;
        let roles_dir = playbook_dir
            .parent()
            .map(|dir| dir.join("roles"))
            .ok_or_else(|| AppError::internal("ansible playbook directory has no roles root"))?;
        Self::ensure_exists(&roles_dir, "ansible roles directory")?;
        Ok(roles_dir)
    }

    fn workspace_metadata_path(workspace_dir: &Path) -> PathBuf {
        workspace_dir.join(WORKSPACE_METADATA_FILE)
    }

    fn record_workspace_outcome(
        &self,
        workspace_dir: &Path,
        outcome: WorkspaceOutcome,
    ) -> AppResult<()> {
        let payload = serde_json::to_vec_pretty(&WorkspaceMetadata { outcome })
            .map_err(|error| AppError::internal(format!("encode workspace metadata: {error}")))?;
        fs::write(Self::workspace_metadata_path(workspace_dir), payload).map_err(|error| {
            AppError::internal(format!(
                "write workspace metadata {}: {error}",
                workspace_dir.display()
            ))
        })
    }

    fn prune_successful_workspaces(&self) -> AppResult<()> {
        fs::create_dir_all(&self.temp_dir).map_err(|error| {
            AppError::internal(format!(
                "create deployment temp dir {}: {error}",
                self.temp_dir.display()
            ))
        })?;

        let mut successful = Vec::new();
        for entry in fs::read_dir(&self.temp_dir).map_err(|error| {
            AppError::internal(format!(
                "read deployment temp dir {}: {error}",
                self.temp_dir.display()
            ))
        })? {
            let entry = entry.map_err(|error| {
                AppError::internal(format!(
                    "read deployment temp dir entry {}: {error}",
                    self.temp_dir.display()
                ))
            })?;
            if !entry
                .file_type()
                .map_err(|error| {
                    AppError::internal(format!(
                        "read deployment temp dir entry type {}: {error}",
                        entry.path().display()
                    ))
                })?
                .is_dir()
            {
                continue;
            }

            let metadata_path = Self::workspace_metadata_path(&entry.path());
            if !metadata_path.exists() {
                continue;
            }
            let Ok(payload) = fs::read(&metadata_path) else {
                continue;
            };
            let Ok(metadata) = serde_json::from_slice::<WorkspaceMetadata>(&payload) else {
                continue;
            };
            if metadata.outcome != WorkspaceOutcome::Succeeded {
                continue;
            }

            let modified = fs::metadata(&metadata_path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            successful.push((modified, entry.path()));
        }

        successful.sort_by(|left, right| right.0.cmp(&left.0));
        for (_, path) in successful
            .into_iter()
            .skip(self.successful_workspace_retention)
        {
            fs::remove_dir_all(&path).map_err(|error| {
                AppError::internal(format!(
                    "remove pruned ansible workspace {}: {error}",
                    path.display()
                ))
            })?;
        }

        Ok(())
    }

    async fn resolve_runtime_secrets(
        &self,
        snapshot: &ExecutionSnapshot,
    ) -> AppResult<RuntimeSecrets> {
        let ssh_secret =
            vault::read_secret(&self.vault_config, &snapshot.credentials.vault_ref).await?;
        let ssh_user =
            ssh_secret.get_first_string(&["ssh_user", "remote_user", "username", "user"]);
        let ssh_private_key =
            ssh_secret.get_first_string(&["ssh_private_key", "private_key", "private_key_pem"]);
        let ssh_password = ssh_secret.get_first_string(&["ssh_password", "password"]);
        let ssh_private_key_passphrase = ssh_secret.get_first_string(&[
            "ssh_private_key_passphrase",
            "private_key_passphrase",
            "passphrase",
        ]);
        if ssh_private_key.is_none() && ssh_password.is_none() {
            return Err(AppError::internal(format!(
                "vault secret `{}` must contain ssh key or password material",
                snapshot.credentials.vault_ref
            )));
        }

        let tls_ca_pem = if let Some(vault_ref) = self.agent_tls_material.ca_vault_ref.as_deref() {
            let secret = vault::read_secret(&self.vault_config, vault_ref).await?;
            Some(required_secret_string(
                &secret,
                vault_ref,
                &["ca_pem", "pem", "value", "certificate"],
            )?)
        } else {
            None
        };
        let tls_cert_pem =
            if let Some(vault_ref) = self.agent_tls_material.cert_vault_ref.as_deref() {
                let secret = vault::read_secret(&self.vault_config, vault_ref).await?;
                Some(required_secret_string(
                    &secret,
                    vault_ref,
                    &["cert_pem", "pem", "value", "certificate"],
                )?)
            } else {
                None
            };
        let tls_key_pem = if let Some(vault_ref) = self.agent_tls_material.key_vault_ref.as_deref()
        {
            let secret = vault::read_secret(&self.vault_config, vault_ref).await?;
            Some(required_secret_string(
                &secret,
                vault_ref,
                &["key_pem", "pem", "value", "private_key"],
            )?)
        } else {
            None
        };

        Ok(RuntimeSecrets {
            ssh_user,
            ssh_private_key,
            ssh_password,
            ssh_private_key_passphrase,
            tls_ca_pem,
            tls_cert_pem,
            tls_key_pem,
        })
    }

    fn prepare_workspace(
        &self,
        snapshot: &ExecutionSnapshot,
        secrets: &RuntimeSecrets,
    ) -> AppResult<PreparedWorkspace> {
        let workspace_dir = self.temp_dir.join(format!(
            "deployment-{}",
            snapshot.deployment_attempt_id.simple()
        ));
        if workspace_dir.exists() {
            fs::remove_dir_all(&workspace_dir).map_err(|error| {
                AppError::internal(format!(
                    "remove stale ansible workspace {}: {error}",
                    workspace_dir.display()
                ))
            })?;
        }
        fs::create_dir_all(&workspace_dir).map_err(|error| {
            AppError::internal(format!(
                "create ansible workspace {}: {error}",
                workspace_dir.display()
            ))
        })?;
        let stdout_path = workspace_dir.join("ansible.stdout.log");
        let stderr_path = workspace_dir.join("ansible.stderr.log");
        fs::write(&stdout_path, "").map_err(|error| {
            AppError::internal(format!(
                "write stdout log {}: {error}",
                stdout_path.display()
            ))
        })?;
        fs::write(&stderr_path, "").map_err(|error| {
            AppError::internal(format!(
                "write stderr log {}: {error}",
                stderr_path.display()
            ))
        })?;

        let roles_dir = self.roles_dir()?;
        let playbook_name = self
            .playbook_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::internal("ansible playbook file name is missing"))?
            .to_string();

        let mut targets = Vec::with_capacity(snapshot.targets.len());
        for target in &snapshot.targets {
            let target_dir = workspace_dir.join(sanitize_hostname(&target.host.hostname));
            let private_data_dir = target_dir.join("private_data");
            let inventory_path = private_data_dir.join("inventory").join("hosts.ini");
            let extravars_path = private_data_dir.join("env").join("extravars");
            let bootstrap_path = private_data_dir
                .join("bootstrap")
                .join(format!("{}.yaml", target.host.hostname));
            let project_dir = private_data_dir.join("project");
            let roles_target_dir = project_dir.join("roles");
            let secrets_dir = private_data_dir.join("secrets");
            fs::create_dir_all(inventory_path.parent().unwrap())
                .map_err(internal_path_error("create inventory dir"))?;
            fs::create_dir_all(extravars_path.parent().unwrap())
                .map_err(internal_path_error("create env dir"))?;
            fs::create_dir_all(bootstrap_path.parent().unwrap())
                .map_err(internal_path_error("create bootstrap dir"))?;
            fs::create_dir_all(&project_dir).map_err(internal_path_error("create project dir"))?;
            fs::create_dir_all(&secrets_dir).map_err(internal_path_error("create secrets dir"))?;
            fs::copy(&self.playbook_path, project_dir.join(&playbook_name))
                .map_err(internal_path_error("copy ansible playbook"))?;
            copy_dir_all(&roles_dir, &roles_target_dir)?;

            fs::write(&inventory_path, render_inventory(target, secrets))
                .map_err(internal_path_error("write inventory"))?;
            fs::write(&bootstrap_path, &target.bootstrap.bootstrap_yaml)
                .map_err(internal_path_error("write bootstrap"))?;
            let merged_vars = merge_target_vars(target, secrets, &secrets_dir)?;
            fs::write(
                &extravars_path,
                serde_json::to_vec_pretty(&merged_vars).map_err(|error| {
                    AppError::internal(format!("encode ansible extravars: {error}"))
                })?,
            )
            .map_err(internal_path_error("write extravars"))?;

            let target_stdout_path = target_dir.join("ansible.stdout.log");
            let target_stderr_path = target_dir.join("ansible.stderr.log");
            fs::write(&target_stdout_path, "")
                .map_err(internal_path_error("write target stdout"))?;
            fs::write(&target_stderr_path, "")
                .map_err(internal_path_error("write target stderr"))?;
            targets.push(PreparedTargetWorkspace {
                deployment_target_id: target.deployment_target_id,
                private_data_dir,
                inventory_path,
                extravars_path,
                bootstrap_path,
                stdout_path: target_stdout_path,
                stderr_path: target_stderr_path,
            });
        }

        Ok(PreparedWorkspace {
            workspace_dir,
            stdout_path,
            stderr_path,
            targets,
        })
    }

    async fn run_target(
        &self,
        snapshot: &ExecutionSnapshot,
        workspace: &PreparedTargetWorkspace,
        cancellation: &CancellationToken,
    ) -> AppResult<(i32, bool, Vec<u8>, Vec<u8>)> {
        let playbook_name = self
            .playbook_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::internal("ansible playbook file name is missing"))?;
        let mut command = Command::new(&self.runner_bin);
        command
            .arg("run")
            .arg(&workspace.private_data_dir)
            .arg("-p")
            .arg(playbook_name)
            .arg("-i")
            .arg(&workspace.inventory_path)
            .arg("--rotate-artifacts")
            .arg("1")
            .env("ANSIBLE_HOST_KEY_CHECKING", "False")
            .env("PYTHONUNBUFFERED", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if snapshot.flags.dry_run || snapshot.flags.force {
            let mut cmdline = Vec::new();
            if snapshot.flags.dry_run {
                cmdline.push("--check");
            }
            if snapshot.flags.force {
                cmdline.push("--force-handlers");
            }
            command.arg("--cmdline").arg(cmdline.join(" "));
        }

        let mut child = command
            .spawn()
            .map_err(|error| AppError::internal(format!("spawn ansible-runner: {error}")))?;
        let stdout_task = child.stdout.take().map(read_pipe);
        let stderr_task = child.stderr.take().map(read_pipe);

        let mut cancelled = false;
        let status = tokio::select! {
            status = child.wait() => status.map_err(|error| AppError::internal(format!("wait for ansible-runner: {error}")))?,
            _ = cancellation.cancelled() => {
                cancelled = true;
                let _ = child.kill().await;
                child.wait().await.map_err(|error| AppError::internal(format!("wait for cancelled ansible-runner: {error}")))?
            }
        };
        let stdout = join_pipe(stdout_task).await?;
        let stderr = join_pipe(stderr_task).await?;
        Ok((
            if cancelled {
                130
            } else {
                status.code().unwrap_or(1)
            },
            cancelled,
            stdout,
            stderr,
        ))
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
        self.roles_dir()?;
        fs::create_dir_all(&self.temp_dir).map_err(|error| {
            AppError::internal(format!(
                "create deployment temp dir {}: {error}",
                self.temp_dir.display()
            ))
        })?;
        self.prune_successful_workspaces()?;
        Ok(())
    }

    async fn execute(
        &self,
        snapshot: &ExecutionSnapshot,
        cancellation: &CancellationToken,
    ) -> AppResult<ExecutionResult> {
        if cancellation.is_cancelled() {
            return Ok(cancelled_result(
                snapshot,
                "cancelled before ansible execution started",
            ));
        }
        let secrets = self.resolve_runtime_secrets(snapshot).await?;
        let workspace = self.prepare_workspace(snapshot, &secrets)?;

        let mut current_phase = "ansible.completed".to_string();
        let mut overall_exit_code = 0;
        let mut targets = Vec::with_capacity(snapshot.targets.len());

        for (index, target) in snapshot.targets.iter().enumerate() {
            let prepared = workspace
                .targets
                .iter()
                .find(|item| item.deployment_target_id == target.deployment_target_id)
                .ok_or_else(|| AppError::internal("target workspace is missing"))?;

            if cancellation.is_cancelled() {
                current_phase = "ansible.cancelled".to_string();
                push_cancelled_targets(
                    &mut targets,
                    &snapshot.targets[index..],
                    &workspace.targets,
                    "cancelled before target execution started",
                )?;
                overall_exit_code = 130;
                break;
            }
            if target.artifact.source_uri.trim().is_empty() {
                overall_exit_code = 1;
                current_phase = "ansible.artifact_unresolved".to_string();
                targets.push(unresolved_artifact_result(target, prepared));
                continue;
            }

            let (exit_code, cancelled, stdout, stderr) =
                self.run_target(snapshot, prepared, cancellation).await?;
            fs::write(&prepared.stdout_path, &stdout)
                .map_err(internal_path_error("write target stdout"))?;
            fs::write(&prepared.stderr_path, &stderr)
                .map_err(internal_path_error("write target stderr"))?;
            append_bytes(&workspace.stdout_path, &stdout)?;
            append_bytes(&workspace.stderr_path, &stderr)?;
            overall_exit_code = overall_exit_code.max(exit_code);
            let status = if cancelled {
                current_phase = "ansible.cancelled".to_string();
                DeploymentTargetStatus::Cancelled
            } else if exit_code == 0 {
                DeploymentTargetStatus::Succeeded
            } else {
                current_phase = "ansible.failed".to_string();
                DeploymentTargetStatus::Failed
            };
            let runtime_steps =
                normalize_runtime_steps(parse_step_markers(&stdout, &target.host.hostname));
            let mut step_log = Vec::with_capacity(runtime_steps.len() + 3);
            step_log.push(StepExecutionResult {
                step_name: "ansible.workspace.prepared".to_string(),
                status: DeploymentStepStatus::Succeeded,
                message: "ansible private_data_dir prepared".to_string(),
                payload_json: json!({
                    "hostname": target.host.hostname,
                    "inventory_path": prepared.inventory_path,
                    "extravars_path": prepared.extravars_path,
                    "bootstrap_path": prepared.bootstrap_path,
                }),
            });
            step_log.push(StepExecutionResult {
                step_name: "vault.secrets.resolved".to_string(),
                status: DeploymentStepStatus::Succeeded,
                message: "vault-backed ssh credentials and agent tls material resolved".to_string(),
                payload_json: json!({
                    "vault_ref": snapshot.credentials.vault_ref,
                    "has_ssh_private_key": secrets.ssh_private_key.is_some(),
                    "has_ssh_password": secrets.ssh_password.is_some(),
                    "has_tls_ca": secrets.tls_ca_pem.is_some(),
                    "has_tls_cert": secrets.tls_cert_pem.is_some(),
                    "has_tls_key": secrets.tls_key_pem.is_some(),
                }),
            });
            step_log.extend(runtime_steps);
            step_log.push(StepExecutionResult {
                step_name: if cancelled {
                    "ansible.runner.cancelled".to_string()
                } else {
                    "ansible.runner.completed".to_string()
                },
                status: if status == DeploymentTargetStatus::Succeeded {
                    DeploymentStepStatus::Succeeded
                } else if status == DeploymentTargetStatus::Cancelled {
                    DeploymentStepStatus::Skipped
                } else {
                    DeploymentStepStatus::Failed
                },
                message: format!(
                    "ansible-runner finished for host `{}` with exit code {exit_code}",
                    target.host.hostname
                ),
                payload_json: json!({
                    "hostname": target.host.hostname,
                    "artifact": target.artifact,
                    "exit_code": exit_code,
                    "stdout_ref": prepared.stdout_path,
                    "stderr_ref": prepared.stderr_path,
                    "stdout_excerpt": excerpt(&stdout),
                    "stderr_excerpt": excerpt(&stderr),
                }),
            });

            targets.push(ExecutionTargetResult {
                deployment_target_id: target.deployment_target_id,
                status,
                error_message: if status == DeploymentTargetStatus::Succeeded {
                    None
                } else {
                    Some(format!(
                        "ansible-runner exited with code {exit_code} for host `{}`",
                        target.host.hostname
                    ))
                },
                steps: step_log,
            });
            if cancelled {
                push_cancelled_targets(
                    &mut targets,
                    &snapshot.targets[index + 1..],
                    &workspace.targets,
                    "cancelled after a previous target execution was interrupted",
                )?;
                break;
            }
        }

        let workspace_outcome = derive_workspace_outcome(overall_exit_code, &targets);
        if let Err(error) = self
            .record_workspace_outcome(&workspace.workspace_dir, workspace_outcome)
            .and_then(|_| self.prune_successful_workspaces())
        {
            append_text(
                &workspace.stderr_path,
                &format!("\n[workspace-cleanup] {error}\n"),
            )?;
        }

        Ok(ExecutionResult {
            current_phase,
            targets,
            steps: vec![StepExecutionResult {
                step_name: "ansible.execution.summary".to_string(),
                status: if overall_exit_code == 0 {
                    DeploymentStepStatus::Succeeded
                } else if overall_exit_code == 130 {
                    DeploymentStepStatus::Skipped
                } else {
                    DeploymentStepStatus::Failed
                },
                message: format!(
                    "ansible-runner processed {} targets",
                    snapshot.targets.len()
                ),
                payload_json: json!({
                    "workspace_dir": workspace.workspace_dir,
                    "stdout_path": workspace.stdout_path,
                    "stderr_path": workspace.stderr_path,
                    "runner_bin": self.runner_bin,
                    "playbook_path": self.playbook_path,
                }),
            }],
            output: ExecutionOutput {
                exit_code: Some(overall_exit_code),
                stdout_ref: Some(workspace.stdout_path.display().to_string()),
                stdout_excerpt: Some(excerpt(
                    &fs::read(&workspace.stdout_path).unwrap_or_default(),
                )),
                stderr_ref: Some(workspace.stderr_path.display().to_string()),
                stderr_excerpt: Some(excerpt(
                    &fs::read(&workspace.stderr_path).unwrap_or_default(),
                )),
            },
        })
    }
}

fn required_secret_string(
    secret: &vault::VaultSecretMap,
    vault_ref: &str,
    keys: &[&str],
) -> AppResult<String> {
    secret.get_first_string(keys).ok_or_else(|| {
        AppError::internal(format!(
            "vault secret `{vault_ref}` is missing any of {:?}",
            keys
        ))
    })
}

fn render_inventory(target: &ExecutionTarget, secrets: &RuntimeSecrets) -> String {
    let ansible_user = secrets
        .ssh_user
        .as_deref()
        .unwrap_or(&target.host.remote_user);
    format!(
        "[agents]\n{} ansible_host={} ansible_port={} ansible_user={}\n",
        target.host.hostname, target.host.ip, target.host.ssh_port, ansible_user
    )
}

fn merge_target_vars(
    target: &ExecutionTarget,
    secrets: &RuntimeSecrets,
    secrets_dir: &Path,
) -> AppResult<Value> {
    let mut vars = target
        .rendered_vars
        .as_object()
        .cloned()
        .ok_or_else(|| AppError::internal("rendered ansible vars must be a JSON object"))?;
    if let Some(user) = secrets.ssh_user.as_ref() {
        vars.insert("ansible_user".to_string(), Value::String(user.clone()));
    }
    if let Some(password) = secrets.ssh_password.as_ref() {
        vars.insert(
            "ansible_password".to_string(),
            Value::String(password.clone()),
        );
    }
    if let Some(passphrase) = secrets.ssh_private_key_passphrase.as_ref() {
        vars.insert(
            "ansible_ssh_private_key_passphrase".to_string(),
            Value::String(passphrase.clone()),
        );
    }
    if let Some(private_key) = secrets.ssh_private_key.as_ref() {
        let path = secrets_dir.join("ssh-private-key.pem");
        write_secret_file(&path, private_key)?;
        vars.insert(
            "ansible_ssh_private_key_file".to_string(),
            Value::String(path.display().to_string()),
        );
    }
    write_tls_src_path(
        &mut vars,
        secrets_dir,
        "doro_agent_tls_ca_src_path",
        "ca.pem",
        secrets.tls_ca_pem.as_deref(),
    )?;
    write_tls_src_path(
        &mut vars,
        secrets_dir,
        "doro_agent_tls_cert_src_path",
        "agent.pem",
        secrets.tls_cert_pem.as_deref(),
    )?;
    write_tls_src_path(
        &mut vars,
        secrets_dir,
        "doro_agent_tls_key_src_path",
        "agent.key",
        secrets.tls_key_pem.as_deref(),
    )?;
    Ok(Value::Object(vars))
}

fn write_tls_src_path(
    vars: &mut serde_json::Map<String, Value>,
    secrets_dir: &Path,
    key: &str,
    filename: &str,
    contents: Option<&str>,
) -> AppResult<()> {
    if let Some(contents) = contents {
        let path = secrets_dir.join(filename);
        write_secret_file(&path, contents)?;
        vars.insert(key.to_string(), Value::String(path.display().to_string()));
    }
    Ok(())
}

fn push_cancelled_targets(
    out: &mut Vec<ExecutionTargetResult>,
    targets: &[ExecutionTarget],
    workspaces: &[PreparedTargetWorkspace],
    message: &str,
) -> AppResult<()> {
    for target in targets {
        let workspace = workspaces
            .iter()
            .find(|item| item.deployment_target_id == target.deployment_target_id)
            .ok_or_else(|| AppError::internal("target workspace is missing"))?;
        out.push(ExecutionTargetResult {
            deployment_target_id: target.deployment_target_id,
            status: DeploymentTargetStatus::Cancelled,
            error_message: Some(message.to_string()),
            steps: vec![StepExecutionResult {
                step_name: "ansible.runner.cancelled".to_string(),
                status: DeploymentStepStatus::Skipped,
                message: message.to_string(),
                payload_json: json!({
                    "hostname": target.host.hostname,
                    "inventory_path": workspace.inventory_path,
                    "extravars_path": workspace.extravars_path,
                }),
            }],
        });
    }
    Ok(())
}

fn parse_step_markers(output: &[u8], hostname: &str) -> Vec<StepExecutionResult> {
    let mut steps = Vec::new();
    let text = String::from_utf8_lossy(output);
    for line in text.lines() {
        if let Some(index) = line.find(STEP_MARKER_PREFIX) {
            let mut marker = line[index + STEP_MARKER_PREFIX.len()..].trim();
            marker = marker.trim_start_matches(|c| c == '"' || c == ' ');
            marker = marker.trim_end_matches(|c| c == '"' || c == ',' || c == ' ');
            let mut parts = marker.splitn(4, '|');
            let event_host = parts.next().unwrap_or("").trim();
            if event_host != hostname {
                continue;
            }
            let step_name = parts.next().unwrap_or("").trim();
            if step_name.is_empty() {
                continue;
            }
            let status_raw = parts.next().unwrap_or("").trim();
            let payload_raw = parts.next().unwrap_or("{}").trim().trim_matches('"');
            let payload_json =
                serde_json::from_str(payload_raw).unwrap_or_else(|_| json!({ "raw": payload_raw }));
            let message = payload_json
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or("ansible runtime step reported");
            steps.push(StepExecutionResult {
                step_name: step_name.to_string(),
                status: match status_raw {
                    "succeeded" => DeploymentStepStatus::Succeeded,
                    "failed" => DeploymentStepStatus::Failed,
                    "skipped" => DeploymentStepStatus::Skipped,
                    _ => DeploymentStepStatus::Failed,
                },
                message: message.to_string(),
                payload_json,
            });
        }
    }
    steps
}

fn normalize_runtime_steps(steps: Vec<StepExecutionResult>) -> Vec<StepExecutionResult> {
    let mut by_name: HashMap<String, StepExecutionResult> = HashMap::new();
    for step in steps {
        by_name.entry(step.step_name.clone()).or_insert(step);
    }
    let mut ordered = Vec::with_capacity(RUNTIME_STEP_ORDER.len());
    for name in RUNTIME_STEP_ORDER {
        if let Some(step) = by_name.remove(*name) {
            ordered.push(step);
        } else {
            ordered.push(StepExecutionResult {
                step_name: name.to_string(),
                status: DeploymentStepStatus::Skipped,
                message: "step not reported by ansible-runner".to_string(),
                payload_json: json!({ "reason": "not_reported" }),
            });
        }
    }
    ordered.extend(by_name.into_values());
    ordered
}

fn unresolved_artifact_result(
    target: &ExecutionTarget,
    workspace: &PreparedTargetWorkspace,
) -> ExecutionTargetResult {
    ExecutionTargetResult {
        deployment_target_id: target.deployment_target_id,
        status: DeploymentTargetStatus::Failed,
        error_message: Some("resolved artifact is missing source_uri".to_string()),
        steps: vec![StepExecutionResult {
            step_name: "artifact.unresolved".to_string(),
            status: DeploymentStepStatus::Failed,
            message: "resolved artifact is missing source_uri".to_string(),
            payload_json: json!({
                "hostname": target.host.hostname,
                "artifact": target.artifact,
                "inventory_path": workspace.inventory_path,
                "extravars_path": workspace.extravars_path,
            }),
        }],
    }
}

fn cancelled_result(snapshot: &ExecutionSnapshot, message: &str) -> ExecutionResult {
    ExecutionResult {
        current_phase: "ansible.cancelled".to_string(),
        targets: snapshot
            .targets
            .iter()
            .map(|target| ExecutionTargetResult {
                deployment_target_id: target.deployment_target_id,
                status: DeploymentTargetStatus::Cancelled,
                error_message: Some(message.to_string()),
                steps: vec![StepExecutionResult {
                    step_name: "ansible.runner.cancelled".to_string(),
                    status: DeploymentStepStatus::Skipped,
                    message: message.to_string(),
                    payload_json: json!({ "hostname": target.host.hostname }),
                }],
            })
            .collect(),
        steps: vec![],
        output: ExecutionOutput {
            exit_code: Some(130),
            stdout_ref: None,
            stdout_excerpt: None,
            stderr_ref: None,
            stderr_excerpt: Some(message.to_string()),
        },
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> AppResult<()> {
    fs::create_dir_all(dst).map_err(internal_path_error("create directory"))?;
    for entry in fs::read_dir(src).map_err(internal_path_error("read directory"))? {
        let entry =
            entry.map_err(|error| AppError::internal(format!("read directory entry: {error}")))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| AppError::internal(format!("read file type: {error}")))?
            .is_dir()
        {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(internal_path_error("copy file"))?;
        }
    }
    Ok(())
}

fn write_secret_file(path: &Path, contents: &str) -> AppResult<()> {
    fs::write(path, contents).map_err(internal_path_error("write secret file"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(internal_path_error("set secret file permissions"))?;
    }
    Ok(())
}

fn append_bytes(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(internal_path_error("open log for append"))?;
    file.write_all(bytes)
        .map_err(internal_path_error("append log"))?;
    Ok(())
}

fn append_text(path: &Path, text: &str) -> AppResult<()> {
    append_bytes(path, text.as_bytes())
}

fn derive_workspace_outcome(
    overall_exit_code: i32,
    targets: &[ExecutionTargetResult],
) -> WorkspaceOutcome {
    if !targets.is_empty()
        && targets
            .iter()
            .all(|target| target.status == DeploymentTargetStatus::Succeeded)
        && overall_exit_code == 0
    {
        return WorkspaceOutcome::Succeeded;
    }
    if targets
        .iter()
        .any(|target| target.status == DeploymentTargetStatus::Cancelled)
        || overall_exit_code == 130
    {
        return WorkspaceOutcome::Cancelled;
    }
    WorkspaceOutcome::Failed
}

fn excerpt(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.len() <= 4096 {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..4096])
    }
}

fn sanitize_hostname(hostname: &str) -> String {
    hostname
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn read_pipe<R>(mut reader: R) -> tokio::task::JoinHandle<std::io::Result<Vec<u8>>>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.map(|_| buf)
    })
}

async fn join_pipe(
    task: Option<tokio::task::JoinHandle<std::io::Result<Vec<u8>>>>,
) -> AppResult<Vec<u8>> {
    let Some(task) = task else {
        return Ok(Vec::new());
    };
    task.await
        .map_err(|error| AppError::internal(format!("join process output task: {error}")))?
        .map_err(|error| AppError::internal(format!("read process output: {error}")))
}

fn internal_path_error(context: &'static str) -> impl FnOnce(std::io::Error) -> AppError {
    move |error| AppError::internal(format!("{context}: {error}"))
}
