use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub postgres_dsn: String,
    pub nats_url: String,
    pub enrollment_http_addr: String,
    pub rust_log: String,
    pub enrollment_dev_bootstrap_token: String,
}

impl RuntimeConfig {
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

        let postgres_dsn = vars
            .get("POSTGRES_DSN")
            .cloned()
            .unwrap_or_else(|| "postgres://postgres:postgres@localhost:5432/doro".to_string());
        let nats_url = vars
            .get("NATS_URL")
            .cloned()
            .unwrap_or_else(|| "nats://localhost:4222".to_string());
        let enrollment_http_addr = vars
            .get("ENROLLMENT_HTTP_ADDR")
            .cloned()
            .unwrap_or_else(|| "0.0.0.0:8081".to_string());
        let rust_log = vars
            .get("RUST_LOG")
            .cloned()
            .unwrap_or_else(|| "info".to_string());
        let enrollment_dev_bootstrap_token = vars
            .get("ENROLLMENT_DEV_BOOTSTRAP_TOKEN")
            .cloned()
            .unwrap_or_else(|| "dev-bootstrap-token".to_string());

        if postgres_dsn.trim().is_empty() {
            return Err(ConfigError::Missing("POSTGRES_DSN"));
        }
        if nats_url.trim().is_empty() {
            return Err(ConfigError::Missing("NATS_URL"));
        }
        if enrollment_http_addr.trim().is_empty() {
            return Err(ConfigError::Missing("ENROLLMENT_HTTP_ADDR"));
        }
        if enrollment_dev_bootstrap_token.trim().is_empty() {
            return Err(ConfigError::Missing("ENROLLMENT_DEV_BOOTSTRAP_TOKEN"));
        }

        Ok(Self {
            postgres_dsn,
            nats_url,
            enrollment_http_addr,
            rust_log,
            enrollment_dev_bootstrap_token,
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required config value: {0}")]
    Missing(&'static str),
}

#[cfg(test)]
mod tests {
    use super::RuntimeConfig;

    #[test]
    fn loads_defaults_when_env_is_missing() {
        let cfg = RuntimeConfig::from_pairs(std::iter::empty::<(String, String)>()).unwrap();
        assert_eq!(
            cfg.postgres_dsn,
            "postgres://postgres:postgres@localhost:5432/doro"
        );
        assert_eq!(cfg.nats_url, "nats://localhost:4222");
        assert_eq!(cfg.enrollment_http_addr, "0.0.0.0:8081");
        assert_eq!(cfg.rust_log, "info");
        assert_eq!(cfg.enrollment_dev_bootstrap_token, "dev-bootstrap-token");
    }

    #[test]
    fn prefers_explicit_env_values() {
        let cfg = RuntimeConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("ENROLLMENT_HTTP_ADDR", "127.0.0.1:9091"),
            ("RUST_LOG", "debug"),
            ("ENROLLMENT_DEV_BOOTSTRAP_TOKEN", "token-123"),
        ])
        .unwrap();

        assert_eq!(cfg.postgres_dsn, "postgres://example");
        assert_eq!(cfg.nats_url, "nats://example:4222");
        assert_eq!(cfg.enrollment_http_addr, "127.0.0.1:9091");
        assert_eq!(cfg.rust_log, "debug");
        assert_eq!(cfg.enrollment_dev_bootstrap_token, "token-123");
    }
}
