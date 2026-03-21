use std::{collections::HashMap, path::PathBuf};

use common::config::{collect_vars, optional_trimmed, required_string, SharedRuntimeConfig};
use thiserror::Error;

use crate::executor::MockFailMode;
use crate::models::ExecutorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactResolverConfig {
    pub manifest_url: String,
    pub release_base_url: Option<String>,
    pub artifact_version: Option<String>,
    pub preferred_package_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultRuntimeConfig {
    pub addr: String,
    pub role_id: String,
    pub secret_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTlsMaterialConfig {
    pub ca_vault_ref: Option<String>,
    pub cert_vault_ref: Option<String>,
    pub key_vault_ref: Option<String>,
}

impl AgentTlsMaterialConfig {
    pub fn has_any_material(&self) -> bool {
        self.ca_vault_ref.is_some() || self.cert_vault_ref.is_some() || self.key_vault_ref.is_some()
    }
}

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
    pub artifact_manifest_url: Option<String>,
    pub release_base_url: Option<String>,
    pub artifact_version: Option<String>,
    pub preferred_package_type: Option<String>,
    pub vault_addr: Option<String>,
    pub vault_role_id: Option<String>,
    pub vault_secret_id: Option<String>,
    pub agent_tls_material: AgentTlsMaterialConfig,
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
        let deployment_temp_dir = optional_trimmed(&vars, "DEPLOYMENT_TEMP_DIR").map(PathBuf::from);
        let artifact_manifest_url = optional_trimmed(&vars, "AGENT_ARTIFACT_MANIFEST_URL");
        let release_base_url = optional_trimmed(&vars, "AGENT_RELEASE_BASE_URL");
        let artifact_version = optional_trimmed(&vars, "AGENT_ARTIFACT_VERSION");
        let preferred_package_type = optional_trimmed(&vars, "AGENT_PREFERRED_PACKAGE_TYPE");
        if let Some(value) = preferred_package_type.as_deref() {
            if !matches!(value, "deb" | "tar.gz") {
                return Err(ConfigError::InvalidEnum("AGENT_PREFERRED_PACKAGE_TYPE"));
            }
        }
        let vault_addr = optional_trimmed(&vars, "VAULT_ADDR");
        let vault_role_id = optional_trimmed(&vars, "VAULT_ROLE_ID");
        let vault_secret_id = optional_trimmed(&vars, "VAULT_SECRET_ID");
        let agent_tls_material = AgentTlsMaterialConfig {
            ca_vault_ref: optional_trimmed(&vars, "AGENT_MTLS_CA_VAULT_REF"),
            cert_vault_ref: optional_trimmed(&vars, "AGENT_MTLS_CERT_VAULT_REF"),
            key_vault_ref: optional_trimmed(&vars, "AGENT_MTLS_KEY_VAULT_REF"),
        };
        if agent_tls_material.cert_vault_ref.is_some() ^ agent_tls_material.key_vault_ref.is_some()
        {
            return Err(ConfigError::InvalidCombination(
                "AGENT_MTLS_CERT_VAULT_REF and AGENT_MTLS_KEY_VAULT_REF must be configured together",
            ));
        }
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
            artifact_manifest_url,
            release_base_url,
            artifact_version,
            preferred_package_type,
            vault_addr,
            vault_role_id,
            vault_secret_id,
            agent_tls_material,
            mock_executor_step_delay_ms,
            mock_executor_fail_mode,
            mock_executor_fail_hosts,
        })
    }

    pub fn artifact_resolver_config(&self) -> Result<Option<ArtifactResolverConfig>, ConfigError> {
        let Some(manifest_url) = self.artifact_manifest_url.clone() else {
            return Ok(None);
        };

        Ok(Some(ArtifactResolverConfig {
            manifest_url,
            release_base_url: self.release_base_url.clone(),
            artifact_version: self.artifact_version.clone(),
            preferred_package_type: self.preferred_package_type.clone(),
        }))
    }

    pub fn vault_runtime_config(&self) -> Result<Option<VaultRuntimeConfig>, ConfigError> {
        match (
            self.vault_addr.clone(),
            self.vault_role_id.clone(),
            self.vault_secret_id.clone(),
        ) {
            (None, None, None) => Ok(None),
            (Some(addr), Some(role_id), Some(secret_id)) => Ok(Some(VaultRuntimeConfig {
                addr,
                role_id,
                secret_id,
            })),
            _ => Err(ConfigError::InvalidCombination(
                "VAULT_ADDR, VAULT_ROLE_ID and VAULT_SECRET_ID must be configured together",
            )),
        }
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
    #[error("{0}")]
    InvalidCombination(&'static str),
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
            (
                "AGENT_ARTIFACT_MANIFEST_URL",
                "https://downloads.example.local/manifest.json",
            ),
            (
                "AGENT_RELEASE_BASE_URL",
                "https://downloads.example.local/agent",
            ),
            ("AGENT_ARTIFACT_VERSION", "0.2.0"),
            ("AGENT_PREFERRED_PACKAGE_TYPE", "deb"),
            ("VAULT_ADDR", "https://vault.example.local"),
            ("VAULT_ROLE_ID", "role-id"),
            ("VAULT_SECRET_ID", "secret-id"),
            ("AGENT_MTLS_CA_VAULT_REF", "secret/data/agent/ca"),
            ("AGENT_MTLS_CERT_VAULT_REF", "secret/data/agent/cert"),
            ("AGENT_MTLS_KEY_VAULT_REF", "secret/data/agent/key"),
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
            config
                .deployment_temp_dir
                .as_ref()
                .unwrap()
                .to_string_lossy(),
            "/tmp/doro"
        );
        assert_eq!(
            config.artifact_manifest_url.as_deref(),
            Some("https://downloads.example.local/manifest.json")
        );
        assert_eq!(config.preferred_package_type.as_deref(), Some("deb"));
        assert_eq!(
            config.vault_addr.as_deref(),
            Some("https://vault.example.local")
        );
        assert_eq!(
            config.agent_tls_material.cert_vault_ref.as_deref(),
            Some("secret/data/agent/cert")
        );
        assert_eq!(config.mock_executor_step_delay_ms, 25);
        assert_eq!(
            config.mock_executor_fail_mode,
            crate::executor::MockFailMode::Partial
        );
        assert_eq!(
            config.mock_executor_fail_hosts,
            vec!["host-a".to_string(), "host-b".to_string()]
        );
        assert_eq!(config.shared.postgres_dsn, "postgres://example");
        assert!(config.artifact_resolver_config().unwrap().is_some());
        assert!(config.vault_runtime_config().unwrap().is_some());
    }

    #[test]
    fn rejects_partial_agent_mtls_keypair_config() {
        let error = DeploymentConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("DEPLOYMENT_HTTP_ADDR", "127.0.0.1:9191"),
            ("DEPLOYMENT_EXECUTOR_KIND", "ansible"),
            ("EDGE_PUBLIC_URL", "https://edge.example.local"),
            ("EDGE_GRPC_ADDR", "edge.example.local:9090"),
            ("AGENT_STATE_DIR_DEFAULT", "/srv/doro-agent"),
            ("AGENT_MTLS_CERT_VAULT_REF", "secret/data/agent/cert"),
        ])
        .unwrap_err();

        assert!(error.to_string().contains(
            "AGENT_MTLS_CERT_VAULT_REF and AGENT_MTLS_KEY_VAULT_REF must be configured together"
        ));
    }
}
