use std::{
    fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use common::{AppError, AppResult};

use crate::{
    executor::traits::DeploymentExecutor,
    models::{DeploymentSnapshot, DeploymentTargetSnapshot, ExecutorKind, TargetExecutionResult},
    render::{bootstrap::bootstrap_file_name, inventory::render_inventory_ini},
};

#[derive(Debug, Clone)]
pub struct AnsibleRunnerExecutor {
    runner_bin: PathBuf,
    playbook_path: PathBuf,
    temp_dir: PathBuf,
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

    fn prepare_workspace(
        &self,
        _snapshot: &DeploymentSnapshot,
        target: &DeploymentTargetSnapshot,
    ) -> AppResult<PathBuf> {
        let workspace_dir = self
            .temp_dir
            .join(format!("deployment-{}", Uuid::new_v4().simple()));
        fs::create_dir_all(&workspace_dir).map_err(|error| {
            AppError::internal(format!(
                "create ansible workspace {}: {error}",
                workspace_dir.display()
            ))
        })?;

        let inventory_path = workspace_dir.join("inventory.ini");
        fs::write(&inventory_path, render_inventory_ini(target)).map_err(|error| {
            AppError::internal(format!(
                "write inventory {}: {error}",
                inventory_path.display()
            ))
        })?;

        let vars_path = workspace_dir.join("vars.json");
        fs::write(&vars_path, target.rendered_vars.to_string()).map_err(|error| {
            AppError::internal(format!("write vars {}: {error}", vars_path.display()))
        })?;

        let bootstrap_path = workspace_dir.join(bootstrap_file_name());
        fs::write(&bootstrap_path, &target.bootstrap.bootstrap_yaml).map_err(|error| {
            AppError::internal(format!(
                "write bootstrap {}: {error}",
                bootstrap_path.display()
            ))
        })?;

        Ok(workspace_dir)
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

    async fn execute_target(
        &self,
        _snapshot: &DeploymentSnapshot,
        target: &DeploymentTargetSnapshot,
        _cancellation: &CancellationToken,
    ) -> AppResult<TargetExecutionResult> {
        let workspace = self.prepare_workspace(_snapshot, target)?;
        Err(AppError::internal(format!(
            "ansible runner executor is not implemented yet; workspace prepared at {}",
            workspace.display()
        )))
    }
}
