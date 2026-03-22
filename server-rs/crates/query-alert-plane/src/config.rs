use std::collections::HashMap;

use common::{
    config::{collect_vars, optional_trimmed, required_string, ConfigError},
    SharedRuntimeConfig,
};

use crate::pipeline::DetectionMode;

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
pub struct DetectionConfig {
    pub mode: DetectionMode,
    pub light_window_min: u32,
    pub medium_window_min: u32,
    pub heavy_window_min: u32,
    pub min_samples: u32,
    pub cooldown_sec: u64,
    pub auto_resolve_sec: u64,
    pub max_groups_per_cycle: u32,
    pub security_findings_enabled: bool,
    pub agent_health_enabled: bool,
    pub log_pattern_enabled: bool,
    pub safe_mode: bool,
}

impl DetectionConfig {
    fn from_vars(vars: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let mode = optional_trimmed(vars, "DETECTION_MODE")
            .map(|value| parse_mode(&value))
            .transpose()?
            .unwrap_or(DetectionMode::Medium);
        let light_window_min = parse_u32(vars, "DETECTION_LIGHT_WINDOW_MIN", 5)?;
        let medium_window_min = parse_u32(vars, "DETECTION_MEDIUM_WINDOW_MIN", 15)?;
        let heavy_window_min = parse_u32(vars, "DETECTION_HEAVY_WINDOW_MIN", 60)?;
        let min_samples = parse_u32(vars, "DETECTION_MIN_SAMPLES", 20)?;
        let cooldown_sec = parse_u64(vars, "DETECTION_COOLDOWN_SEC", 900)?;
        let auto_resolve_sec = parse_u64(vars, "DETECTION_AUTO_RESOLVE_SEC", 1800)?;
        let max_groups_per_cycle = parse_u32(vars, "DETECTION_MAX_GROUPS_PER_CYCLE", 1000)?;

        Ok(Self {
            mode,
            light_window_min,
            medium_window_min,
            heavy_window_min,
            min_samples,
            cooldown_sec,
            auto_resolve_sec,
            max_groups_per_cycle,
            security_findings_enabled: parse_bool(
                vars,
                "DETECTION_SECURITY_FINDINGS_ENABLED",
                true,
            ),
            agent_health_enabled: parse_bool(vars, "DETECTION_AGENT_HEALTH_ENABLED", true),
            log_pattern_enabled: parse_bool(vars, "DETECTION_LOG_PATTERN_ENABLED", true),
            safe_mode: parse_bool(vars, "DETECTION_SAFE_MODE", false),
        })
    }
}

fn parse_mode(value: &str) -> Result<DetectionMode, ConfigError> {
    DetectionMode::from_str(value)
        .ok_or_else(|| ConfigError::InvalidNumber("DETECTION_MODE".to_string()))
}

fn parse_u32(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u32,
) -> Result<u32, ConfigError> {
    if let Some(value) = optional_trimmed(vars, key) {
        value
            .parse::<u32>()
            .map_err(|_| ConfigError::InvalidNumber(key.to_string()))
    } else {
        Ok(default)
    }
}

fn parse_u64(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u64,
) -> Result<u64, ConfigError> {
    if let Some(value) = optional_trimmed(vars, key) {
        value
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidNumber(key.to_string()))
    } else {
        Ok(default)
    }
}

fn parse_bool(vars: &HashMap<String, String>, key: &'static str, default: bool) -> bool {
    optional_trimmed(vars, key)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAlertPlaneConfig {
    pub shared: SharedRuntimeConfig,
    pub http_addr: String,
    pub opensearch: OpenSearchConfig,
    pub clickhouse: ClickHouseConfig,
    pub rare_fingerprint: RareFingerprintConfig,
    pub anomaly: AnomalyEngineConfig,
    pub detection: DetectionConfig,
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
            detection: DetectionConfig::from_vars(&vars)?,
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
    use super::{DetectionConfig, QueryAlertPlaneConfig};
    use crate::pipeline::DetectionMode;
    use common::config::collect_vars;

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
        assert_eq!(config.detection.mode, DetectionMode::Medium);
        assert!(config.detection.security_findings_enabled);
    }

    #[test]
    fn loads_detection_config_overrides() {
        let vars = collect_vars([
            ("DETECTION_MODE", "heavy"),
            ("DETECTION_LIGHT_WINDOW_MIN", "1"),
            ("DETECTION_MEDIUM_WINDOW_MIN", "2"),
            ("DETECTION_HEAVY_WINDOW_MIN", "3"),
            ("DETECTION_MIN_SAMPLES", "4"),
            ("DETECTION_COOLDOWN_SEC", "5"),
            ("DETECTION_AUTO_RESOLVE_SEC", "6"),
            ("DETECTION_MAX_GROUPS_PER_CYCLE", "7"),
            ("DETECTION_SECURITY_FINDINGS_ENABLED", "false"),
            ("DETECTION_AGENT_HEALTH_ENABLED", "false"),
            ("DETECTION_LOG_PATTERN_ENABLED", "false"),
            ("DETECTION_SAFE_MODE", "true"),
        ]);
        let config = DetectionConfig::from_vars(&vars).unwrap();

        assert_eq!(config.mode, DetectionMode::Heavy);
        assert_eq!(config.light_window_min, 1);
        assert_eq!(config.medium_window_min, 2);
        assert_eq!(config.heavy_window_min, 3);
        assert_eq!(config.min_samples, 4);
        assert_eq!(config.cooldown_sec, 5);
        assert_eq!(config.auto_resolve_sec, 6);
        assert_eq!(config.max_groups_per_cycle, 7);
        assert!(!config.security_findings_enabled);
        assert!(!config.agent_health_enabled);
        assert!(!config.log_pattern_enabled);
        assert!(config.safe_mode);
    }
}
