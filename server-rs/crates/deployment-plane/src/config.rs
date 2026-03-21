use std::{collections::HashMap, path::PathBuf};

use thiserror::Error;

use crate::models::ExecutorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentConfig {
    pub postgres_dsn: String,
    pub nats_url: String,
    pub deployment_http_addr: String,
    pub deployment_executor_kind: ExecutorKind,
    pub edge_public_url: String,
    pub edge_grpc_addr: String,
    pub agent_state_dir_default: String,
    pub ansible_runner_bin: Option<String>,
    pub ansible_playbook_path: Option<String>,
    pub deployment_temp_dir: Option<PathBuf>,
    pub rust_log: String,
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
        let vars: HashMap<String, String> = vars
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();

        let postgres_dsn = required_or_default(
            &vars,
            "POSTGRES_DSN",
            "postgres://postgres:postgres@localhost:5432/doro",
        )?;
        let nats_url = required_or_default(&vars, "NATS_URL", "nats://localhost:4222")?;
        let deployment_http_addr =
            required_or_default(&vars, "DEPLOYMENT_HTTP_ADDR", "0.0.0.0:8083")?;
        let edge_public_url =
            required_or_default(&vars, "EDGE_PUBLIC_URL", "http://localhost:8080")?;
        let edge_grpc_addr = required_or_default(&vars, "EDGE_GRPC_ADDR", "localhost:9090")?;
        let agent_state_dir_default =
            required_or_default(&vars, "AGENT_STATE_DIR_DEFAULT", "/var/lib/doro-agent")?;
        let rust_log = vars
            .get("RUST_LOG")
            .cloned()
            .unwrap_or_else(|| "info".to_string());

        let executor_kind_raw = required_or_default(&vars, "DEPLOYMENT_EXECUTOR_KIND", "mock")?;
        let deployment_executor_kind = ExecutorKind::from_str(&executor_kind_raw)
            .ok_or(ConfigError::InvalidEnum("DEPLOYMENT_EXECUTOR_KIND"))?;

        let ansible_runner_bin = optional_trimmed(&vars, "ANSIBLE_RUNNER_BIN");
        let ansible_playbook_path = optional_trimmed(&vars, "ANSIBLE_PLAYBOOK_PATH");
        let deployment_temp_dir = optional_trimmed(&vars, "DEPLOYMENT_TEMP_DIR").map(PathBuf::from);

        Ok(Self {
            postgres_dsn,
            nats_url,
            deployment_http_addr,
            deployment_executor_kind,
            edge_public_url,
            edge_grpc_addr,
            agent_state_dir_default,
            ansible_runner_bin,
            ansible_playbook_path,
            deployment_temp_dir,
            rust_log,
        })
    }
}

fn required_or_default(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: &'static str,
) -> Result<String, ConfigError> {
    let value = vars
        .get(key)
        .cloned()
        .unwrap_or_else(|| default.to_string());
    if value.trim().is_empty() {
        return Err(ConfigError::Missing(key));
    }
    Ok(value)
}

fn optional_trimmed(vars: &HashMap<String, String>, key: &'static str) -> Option<String> {
    vars.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required config value: {0}")]
    Missing(&'static str),
    #[error("invalid enum value for: {0}")]
    InvalidEnum(&'static str),
}

#[cfg(test)]
mod tests {
    use super::DeploymentConfig;
    use crate::models::ExecutorKind;

    #[test]
    fn loads_defaults() {
        let config = DeploymentConfig::from_pairs(std::iter::empty::<(String, String)>()).unwrap();
        assert_eq!(config.deployment_http_addr, "0.0.0.0:8083");
        assert_eq!(config.deployment_executor_kind, ExecutorKind::Mock);
        assert_eq!(config.edge_grpc_addr, "localhost:9090");
    }

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
            ("RUST_LOG", "debug"),
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
    }
}
