use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedRuntimeConfig {
    pub postgres_dsn: String,
    pub nats_url: String,
    pub rust_log: String,
}

impl SharedRuntimeConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_pairs(std::env::vars())
    }

    pub fn from_pairs<I, K, V>(vars: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let vars = collect_vars(vars);

        let postgres_dsn = required_string(&vars, "POSTGRES_DSN")?;
        let nats_url = required_string(&vars, "NATS_URL")?;
        let rust_log = vars
            .get("RUST_LOG")
            .cloned()
            .unwrap_or_else(|| "info".to_string());

        Ok(Self {
            postgres_dsn,
            nats_url,
            rust_log,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollmentPlaneConfig {
    pub shared: SharedRuntimeConfig,
    pub http_addr: String,
    pub dev_bootstrap_token: String,
}

impl EnrollmentPlaneConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_pairs(std::env::vars())
    }

    pub fn from_pairs<I, K, V>(vars: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let vars = collect_vars(vars);
        Ok(Self {
            shared: SharedRuntimeConfig::from_pairs(vars.clone())?,
            http_addr: required_string(&vars, "ENROLLMENT_HTTP_ADDR")?,
            dev_bootstrap_token: required_string(&vars, "ENROLLMENT_DEV_BOOTSTRAP_TOKEN")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneConfig {
    pub shared: SharedRuntimeConfig,
    pub http_addr: String,
}

impl ControlPlaneConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_pairs(std::env::vars())
    }

    pub fn from_pairs<I, K, V>(vars: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let vars = collect_vars(vars);
        Ok(Self {
            shared: SharedRuntimeConfig::from_pairs(vars.clone())?,
            http_addr: required_string(&vars, "CONTROL_HTTP_ADDR")?,
        })
    }
}

pub fn collect_vars<I, K, V>(vars: I) -> HashMap<String, String>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    vars.into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect()
}

pub fn required_string(
    vars: &HashMap<String, String>,
    key: &'static str,
) -> Result<String, ConfigError> {
    let value = vars.get(key).cloned().ok_or(ConfigError::Missing(key))?;
    if value.trim().is_empty() {
        return Err(ConfigError::Missing(key));
    }
    Ok(value)
}

pub fn optional_trimmed(vars: &HashMap<String, String>, key: &'static str) -> Option<String> {
    vars.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required config value: {0}")]
    Missing(&'static str),
}

#[cfg(test)]
mod tests {
    use super::{collect_vars, ControlPlaneConfig, EnrollmentPlaneConfig, SharedRuntimeConfig};

    #[test]
    fn loads_explicit_shared_values() {
        let cfg = SharedRuntimeConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("RUST_LOG", "debug"),
        ])
        .unwrap();

        assert_eq!(cfg.postgres_dsn, "postgres://example");
        assert_eq!(cfg.nats_url, "nats://example:4222");
        assert_eq!(cfg.rust_log, "debug");
    }

    #[test]
    fn enrollment_plane_requires_plane_specific_values() {
        let cfg = EnrollmentPlaneConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("ENROLLMENT_HTTP_ADDR", "127.0.0.1:9091"),
            ("ENROLLMENT_DEV_BOOTSTRAP_TOKEN", "token-123"),
        ])
        .unwrap();

        assert_eq!(cfg.http_addr, "127.0.0.1:9091");
        assert_eq!(cfg.dev_bootstrap_token, "token-123");
    }

    #[test]
    fn control_plane_requires_control_addr() {
        let cfg = ControlPlaneConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("CONTROL_HTTP_ADDR", "127.0.0.1:9092"),
        ])
        .unwrap();

        assert_eq!(cfg.http_addr, "127.0.0.1:9092");
    }

    #[test]
    fn collect_vars_normalizes_env_pairs() {
        let vars = collect_vars([("POSTGRES_DSN", "postgres://example")]);
        assert_eq!(
            vars.get("POSTGRES_DSN").map(String::as_str),
            Some("postgres://example")
        );
    }
}
