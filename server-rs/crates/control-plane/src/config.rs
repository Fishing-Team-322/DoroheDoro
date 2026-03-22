use std::{collections::HashMap, time::Duration};

use common::config::{collect_vars, optional_trimmed, required_string};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultRuntimeConfig {
    pub addr: String,
    pub role_id: String,
    pub secret_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramRuntimeConfig {
    pub worker_enabled: bool,
    pub api_base_url: String,
    pub request_timeout: Duration,
    pub min_send_interval: Duration,
    pub poll_interval: Duration,
    pub batch_size: u32,
    pub max_attempts: u32,
    pub edge_public_url: Option<String>,
    pub vault: Option<VaultRuntimeConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneRuntimeConfig {
    pub shared: common::config::SharedRuntimeConfig,
    pub http_addr: String,
    pub telegram: TelegramRuntimeConfig,
}

impl ControlPlaneRuntimeConfig {
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
        let shared = common::config::SharedRuntimeConfig::from_pairs(vars.clone())?;
        let http_addr = required_string(&vars, "CONTROL_HTTP_ADDR")?;
        let worker_enabled = parse_bool(&vars, "TELEGRAM_WORKER_ENABLED")?.unwrap_or(false);
        let api_base_url = optional_trimmed(&vars, "TELEGRAM_API_BASE_URL")
            .unwrap_or_else(|| "https://api.telegram.org".to_string());
        let request_timeout_ms = parse_u64(&vars, "TELEGRAM_REQUEST_TIMEOUT_MS")?.unwrap_or(5_000);
        let min_send_interval_ms =
            parse_u64(&vars, "TELEGRAM_MIN_SEND_INTERVAL_MS")?.unwrap_or(250);
        let poll_interval_ms =
            parse_u64(&vars, "TELEGRAM_WORKER_POLL_INTERVAL_MS")?.unwrap_or(2_000);
        let batch_size = parse_u32(&vars, "TELEGRAM_WORKER_BATCH_SIZE")?.unwrap_or(20);
        let max_attempts = parse_u32(&vars, "TELEGRAM_DELIVERY_MAX_ATTEMPTS")?.unwrap_or(4);
        let edge_public_url = optional_trimmed(&vars, "EDGE_PUBLIC_URL");
        let vault = parse_vault_runtime_config(&vars)?;

        Ok(Self {
            shared,
            http_addr,
            telegram: TelegramRuntimeConfig {
                worker_enabled,
                api_base_url,
                request_timeout: Duration::from_millis(request_timeout_ms),
                min_send_interval: Duration::from_millis(min_send_interval_ms),
                poll_interval: Duration::from_millis(poll_interval_ms),
                batch_size,
                max_attempts,
                edge_public_url,
                vault,
            },
        })
    }
}

fn parse_vault_runtime_config(
    vars: &HashMap<String, String>,
) -> Result<Option<VaultRuntimeConfig>, ConfigError> {
    match (
        optional_trimmed(vars, "VAULT_ADDR"),
        optional_trimmed(vars, "VAULT_ROLE_ID"),
        optional_trimmed(vars, "VAULT_SECRET_ID"),
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

fn parse_u32(
    vars: &HashMap<String, String>,
    key: &'static str,
) -> Result<Option<u32>, ConfigError> {
    optional_trimmed(vars, key)
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| ConfigError::InvalidNumber(key))
        })
        .transpose()
}

fn parse_u64(
    vars: &HashMap<String, String>,
    key: &'static str,
) -> Result<Option<u64>, ConfigError> {
    optional_trimmed(vars, key)
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| ConfigError::InvalidNumber(key))
        })
        .transpose()
}

fn parse_bool(
    vars: &HashMap<String, String>,
    key: &'static str,
) -> Result<Option<bool>, ConfigError> {
    optional_trimmed(vars, key)
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(ConfigError::InvalidBool(key)),
        })
        .transpose()
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error(transparent)]
    Common(#[from] common::config::ConfigError),
    #[error("invalid numeric value for: {0}")]
    InvalidNumber(&'static str),
    #[error("invalid boolean value for: {0}")]
    InvalidBool(&'static str),
    #[error("{0}")]
    InvalidCombination(&'static str),
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::ControlPlaneRuntimeConfig;

    #[test]
    fn loads_defaults_without_vault() {
        let config = ControlPlaneRuntimeConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("CONTROL_HTTP_ADDR", "127.0.0.1:9092"),
        ])
        .unwrap();

        assert_eq!(config.http_addr, "127.0.0.1:9092");
        assert!(!config.telegram.worker_enabled);
        assert_eq!(config.telegram.api_base_url, "https://api.telegram.org");
        assert_eq!(
            config.telegram.min_send_interval,
            Duration::from_millis(250)
        );
        assert_eq!(config.telegram.batch_size, 20);
        assert_eq!(config.telegram.max_attempts, 4);
        assert!(config.telegram.edge_public_url.is_none());
        assert!(config.telegram.vault.is_none());
    }

    #[test]
    fn loads_overrides_with_vault() {
        let config = ControlPlaneRuntimeConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("CONTROL_HTTP_ADDR", "127.0.0.1:9092"),
            ("TELEGRAM_WORKER_ENABLED", "true"),
            ("TELEGRAM_API_BASE_URL", "http://telegram.example.local"),
            ("TELEGRAM_REQUEST_TIMEOUT_MS", "7000"),
            ("TELEGRAM_MIN_SEND_INTERVAL_MS", "500"),
            ("TELEGRAM_WORKER_POLL_INTERVAL_MS", "2500"),
            ("TELEGRAM_WORKER_BATCH_SIZE", "10"),
            ("TELEGRAM_DELIVERY_MAX_ATTEMPTS", "6"),
            ("EDGE_PUBLIC_URL", "https://edge.example.local"),
            ("VAULT_ADDR", "http://vault.example.local"),
            ("VAULT_ROLE_ID", "role-id"),
            ("VAULT_SECRET_ID", "secret-id"),
        ])
        .unwrap();

        assert!(config.telegram.worker_enabled);
        assert_eq!(
            config.telegram.api_base_url,
            "http://telegram.example.local"
        );
        assert_eq!(
            config.telegram.min_send_interval,
            Duration::from_millis(500)
        );
        assert_eq!(config.telegram.batch_size, 10);
        assert_eq!(config.telegram.max_attempts, 6);
        assert_eq!(
            config.telegram.edge_public_url.as_deref(),
            Some("https://edge.example.local")
        );
        assert_eq!(
            config
                .telegram
                .vault
                .as_ref()
                .map(|vault| vault.addr.as_str()),
            Some("http://vault.example.local")
        );
    }

    #[test]
    fn requires_complete_vault_configuration() {
        let error = ControlPlaneRuntimeConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("CONTROL_HTTP_ADDR", "127.0.0.1:9092"),
            ("VAULT_ADDR", "http://vault.example.local"),
        ])
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "VAULT_ADDR, VAULT_ROLE_ID and VAULT_SECRET_ID must be configured together"
        );
    }

    #[test]
    fn rejects_invalid_worker_flag() {
        let error = ControlPlaneRuntimeConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://example"),
            ("NATS_URL", "nats://example:4222"),
            ("CONTROL_HTTP_ADDR", "127.0.0.1:9092"),
            ("TELEGRAM_WORKER_ENABLED", "sometimes"),
        ])
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "invalid boolean value for: TELEGRAM_WORKER_ENABLED"
        );
    }
}
