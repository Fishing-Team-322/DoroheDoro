use std::collections::HashMap;

use common::{
    config::{collect_vars, optional_trimmed, required_string, ConfigError},
    SharedRuntimeConfig,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSearchConfig {
    pub url: String,
    pub index_prefix: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickHouseConfig {
    pub dsn: String,
    pub database: String,
    pub table: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAlertPlaneConfig {
    pub shared: SharedRuntimeConfig,
    pub http_addr: String,
    pub opensearch: OpenSearchConfig,
    pub clickhouse: ClickHouseConfig,
    pub rare_fingerprint: RareFingerprintConfig,
    pub anomaly: AnomalyEngineConfig,
}

impl QueryAlertPlaneConfig {
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
            http_addr: required_string(&vars, "QUERY_ALERT_HTTP_ADDR")?,
            opensearch: OpenSearchConfig {
                url: required_string(&vars, "OPENSEARCH_URL")?,
                index_prefix: required_string(&vars, "OPENSEARCH_INDEX_PREFIX")?,
                username: optional_trimmed(&vars, "OPENSEARCH_USERNAME"),
                password: optional_trimmed(&vars, "OPENSEARCH_PASSWORD"),
            },
            clickhouse: ClickHouseConfig {
                dsn: required_string(&vars, "CLICKHOUSE_DSN")?,
                database: required_string(&vars, "CLICKHOUSE_DATABASE")?,
                table: required_string(&vars, "CLICKHOUSE_TABLE")?,
            },
            rare_fingerprint: RareFingerprintConfig::from_vars(&vars)?,
            anomaly: AnomalyEngineConfig::from_vars(&vars)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RareFingerprintConfig {
    pub enabled: bool,
    pub window_minutes: u32,
    pub max_count: u64,
    pub severity: String,
}

impl RareFingerprintConfig {
    fn from_vars(vars: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let enabled = parse_bool(vars, "RARE_FINGERPRINT_ENABLED", true);
        let window_minutes = parse_number(vars, "RARE_FINGERPRINT_WINDOW_MINUTES", 60)?;
        let max_count = parse_number(vars, "RARE_FINGERPRINT_MAX_COUNT", 1)?;
        if window_minutes == 0 || window_minutes > 24 * 60 {
            return Err(ConfigError::InvalidNumber(
                "RARE_FINGERPRINT_WINDOW_MINUTES".to_string(),
            ));
        }
        let severity = vars
            .get("RARE_FINGERPRINT_SEVERITY")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "medium".to_string());
        Ok(Self {
            enabled,
            window_minutes,
            max_count,
            severity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnomalyEngineConfig {
    pub evaluation_interval_secs: u64,
    pub rule_cache_ttl_secs: u64,
}

impl AnomalyEngineConfig {
    fn from_vars(vars: &HashMap<String, String>) -> Result<Self, ConfigError> {
        Ok(Self {
            evaluation_interval_secs: parse_number(
                vars,
                "ANOMALY_EVALUATION_INTERVAL_SECS",
                60u64,
            )?,
            rule_cache_ttl_secs: parse_number(vars, "ANOMALY_RULE_CACHE_TTL_SECS", 30u64)?,
        })
    }
}

fn parse_bool(vars: &HashMap<String, String>, key: &str, default: bool) -> bool {
    vars.get(key)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn parse_number<T>(vars: &HashMap<String, String>, key: &str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
{
    match vars
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        Some(raw) => raw
            .parse::<T>()
            .map_err(|_| ConfigError::InvalidNumber(key.to_string())),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::QueryAlertPlaneConfig;

    #[test]
    fn loads_query_alert_config() {
        let config = QueryAlertPlaneConfig::from_pairs([
            ("POSTGRES_DSN", "postgres://localhost/test"),
            ("NATS_URL", "nats://localhost:4222"),
            ("QUERY_ALERT_HTTP_ADDR", "0.0.0.0:9095"),
            ("OPENSEARCH_URL", "http://localhost:9200"),
            ("OPENSEARCH_INDEX_PREFIX", "doro"),
            ("CLICKHOUSE_DSN", "http://localhost:8123"),
            ("CLICKHOUSE_DATABASE", "doro"),
            ("CLICKHOUSE_TABLE", "logs"),
            ("RARE_FINGERPRINT_ENABLED", "true"),
            ("RARE_FINGERPRINT_WINDOW_MINUTES", "30"),
            ("RARE_FINGERPRINT_MAX_COUNT", "2"),
            ("RARE_FINGERPRINT_SEVERITY", "low"),
            ("ANOMALY_EVALUATION_INTERVAL_SECS", "45"),
            ("ANOMALY_RULE_CACHE_TTL_SECS", "20"),
        ])
        .unwrap();

        assert_eq!(config.http_addr, "0.0.0.0:9095");
        assert_eq!(config.shared.nats_url, "nats://localhost:4222");
        assert!(config.rare_fingerprint.enabled);
        assert_eq!(config.rare_fingerprint.window_minutes, 30);
        assert_eq!(config.rare_fingerprint.max_count, 2);
        assert_eq!(config.rare_fingerprint.severity, "low");
        assert_eq!(config.anomaly.evaluation_interval_secs, 45);
        assert_eq!(config.anomaly.rule_cache_ttl_secs, 20);
    }
}
