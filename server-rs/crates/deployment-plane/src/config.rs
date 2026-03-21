use std::{collections::HashMap, path::PathBuf};

use common::config::{collect_vars, optional_trimmed, required_string, SharedRuntimeConfig};
use thiserror::Error;

use crate::executor::MockFailMode;
use crate::models::ExecutorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentConfig {
    pub shared: SharedRuntimeConfig,
    pub deployment_http_addr: String,
    pub deployment_executor_kind: ExecutorKind,
    pub edge_public_url: String,
    pub edge_grpc_addr: String,
    pub agent_state_dir_default: String,
    pub ansible_runner_bin: Option<String>,
    pub ansible_playbook_path: Option<String>,
    pub deployment_temp_dir: Option<PathBuf>,
    pub mock_executor_step_delay_ms: u64,
    pub mock_executor_fail_mode: MockFailMode,
    pub mock_executor_fail_hosts: Vec<String>,
}

impl DeploymentConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_pairs(std::env::vars())
    }

    pub fn from_pairs<I, K, V>(vars: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let vars: HashMap<String, String> = collect_vars(vars);

        let shared = SharedRuntimeConfig::from_pairs(vars.clone())?;
        let deployment_http_addr = required_string(&vars, "DEPLOYMENT_HTTP_ADDR")?;
        let edge_public_url = required_string(&vars, "EDGE_PUBLIC_URL")?;
        let edge_grpc_addr = required_string(&vars, "EDGE_GRPC_ADDR")?;
        let agent_state_dir_default = required_string(&vars, "AGENT_STATE_DIR_DEFAULT")?;

        let executor_kind_raw = required_string(&vars, "DEPLOYMENT_EXECUTOR_KIND")?;
        let deployment_executor_kind = ExecutorKind::from_str(&executor_kind_raw)
            .ok_or(ConfigError::InvalidEnum("DEPLOYMENT_EXECUTOR_KIND"))?;

        let ansible_runner_bin = optional_trimmed(&vars, "ANSIBLE_RUNNER_BIN");
        let ansible_playbook_path = optional_trimmed(&vars, "ANSIBLE_PLAYBOOK_PATH");
        let deployment_temp_dir =
            optional_trimmed(&vars, "DEPLOYMENT_TEMP_DIR").map(PathBuf::from);
        let mock_executor_step_delay_ms = optional_trimmed(&vars, "MOCK_EXECUTOR_STEP_DELAY_MS")
            .map(|value| {
                value
                    .parse::<u64>()
                    .map_err(|_| ConfigError::InvalidNumber("MOCK_EXECUTOR_STEP_DELAY_MS"))
            })
            .transpose()?
            .unwrap_or(5);
        let mock_executor_fail_mode = optional_trimmed(&vars, "MOCK_EXECUTOR_FAIL_MODE")
            .map(|value| MockFailMode::from_str(&value))
            .unwrap_or(Some(MockFailMode::Never))
            .ok_or(ConfigError::InvalidEnum("MOCK_EXECUTOR_FAIL_MODE"))?;
        let mock_executor_fail_hosts = optional_trimmed(&vars, "MOCK_EXECUTOR_FAIL_HOSTS")
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            shared,
            deployment_http_addr,
            deployment_executor_kind,
            edge_public_url,
            edge_grpc_addr,
            agent_state_dir_default,
            ansible_runner_bin,
            ansible_playbook_path,
            deployment_temp_dir,
            mock_executor_step_delay_ms,
            mock_executor_fail_mode,
            mock_executor_fail_hosts,
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error(transparent)]
    Common(#[from] common::config::ConfigError),
    #[error("missing required config value: {0}")]
    Missing(&'static str),
    #[error("invalid numeric value for: {0}")]
    InvalidNumber(&'static str),
    #[error("invalid enum value for: {0}")]
    InvalidEnum(&'static str),
}

#[cfg(test)]
mod tests {
    use super::DeploymentConfig;
    use crate::models::ExecutorKind;

    #[test]
    fn parses_overrides() {
        let config = DeploymentConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("DEPLOYMENT_HTTP_ADDR", "127.0.0.1:9191"),
            ("DEPLOYMENT_EXECUTOR_KIND", "ansible"),
            ("EDGE_PUBLIC_URL", "https://edge.example.local"),
            ("EDGE_GRPC_ADDR", "edge.example.local:9090"),
            ("AGENT_STATE_DIR_DEFAULT", "/srv/doro-agent"),
            ("ANSIBLE_RUNNER_BIN", "/usr/bin/ansible-runner"),
            ("ANSIBLE_PLAYBOOK_PATH", "/srv/playbooks/install.yaml"),
            ("DEPLOYMENT_TEMP_DIR", "/tmp/doro"),
            ("MOCK_EXECUTOR_STEP_DELAY_MS", "25"),
            ("MOCK_EXECUTOR_FAIL_MODE", "partial"),
            ("MOCK_EXECUTOR_FAIL_HOSTS", "host-a,host-b"),
        ])
        .unwrap();

        assert_eq!(config.deployment_executor_kind, ExecutorKind::Ansible);
        assert_eq!(
            config.ansible_runner_bin.as_deref(),
            Some("/usr/bin/ansible-runner")
        );
        assert_eq!(
            config.deployment_temp_dir.unwrap().to_string_lossy(),
            "/tmp/doro"
        );
        assert_eq!(config.mock_executor_step_delay_ms, 25);
        assert_eq!(config.mock_executor_fail_mode, crate::executor::MockFailMode::Partial);
        assert_eq!(
            config.mock_executor_fail_hosts,
            vec!["host-a".to_string(), "host-b".to_string()]
        );
        assert_eq!(config.shared.postgres_dsn, "postgres://example");
    }
}
