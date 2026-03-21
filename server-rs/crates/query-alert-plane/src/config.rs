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
    DetectionMode::from_str(value).ok_or(ConfigError::InvalidEnum("DETECTION_MODE"))
}

fn parse_u32(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u32,
) -> Result<u32, ConfigError> {
    if let Some(value) = optional_trimmed(vars, key) {
        value
            .parse::<u32>()
            .map_err(|_| ConfigError::InvalidNumber(key))
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
            .map_err(|_| ConfigError::InvalidNumber(key))
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
            detection: DetectionConfig::from_vars(&vars)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DetectionConfig, QueryAlertPlaneConfig};
    use crate::pipeline::DetectionMode;

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
        ])
        .unwrap();

        assert_eq!(config.http_addr, "0.0.0.0:9095");
        assert_eq!(config.shared.nats_url, "nats://localhost:4222");
        assert_eq!(config.detection.mode, DetectionMode::Medium);
    }

    #[test]
    fn detection_config_uses_defaults() {
        let vars = minimal_vars();
        let detection = DetectionConfig::from_vars(&vars).unwrap();
        assert_eq!(detection.mode, DetectionMode::Medium);
        assert_eq!(detection.light_window_min, 5);
        assert_eq!(detection.medium_window_min, 15);
        assert_eq!(detection.heavy_window_min, 60);
        assert_eq!(detection.min_samples, 20);
        assert_eq!(detection.cooldown_sec, 900);
        assert_eq!(detection.auto_resolve_sec, 1800);
        assert_eq!(detection.max_groups_per_cycle, 1000);
        assert!(detection.security_findings_enabled);
        assert!(detection.agent_health_enabled);
        assert!(detection.log_pattern_enabled);
        assert!(!detection.safe_mode);
    }

    #[test]
    fn detection_config_respects_overrides() {
        let mut vars = minimal_vars();
        vars.insert("DETECTION_MODE".into(), "heavy".into());
        vars.insert("DETECTION_LIGHT_WINDOW_MIN".into(), "10".into());
        vars.insert("DETECTION_MEDIUM_WINDOW_MIN".into(), "20".into());
        vars.insert("DETECTION_HEAVY_WINDOW_MIN".into(), "120".into());
        vars.insert("DETECTION_MIN_SAMPLES".into(), "50".into());
        vars.insert("DETECTION_COOLDOWN_SEC".into(), "60".into());
        vars.insert("DETECTION_AUTO_RESOLVE_SEC".into(), "600".into());
        vars.insert("DETECTION_MAX_GROUPS_PER_CYCLE".into(), "42".into());
        vars.insert("DETECTION_SECURITY_FINDINGS_ENABLED".into(), "false".into());
        vars.insert("DETECTION_AGENT_HEALTH_ENABLED".into(), "0".into());
        vars.insert("DETECTION_LOG_PATTERN_ENABLED".into(), "no".into());
        vars.insert("DETECTION_SAFE_MODE".into(), "1".into());

        let detection = DetectionConfig::from_vars(&vars).unwrap();
        assert_eq!(detection.mode, DetectionMode::Heavy);
        assert_eq!(detection.light_window_min, 10);
        assert_eq!(detection.medium_window_min, 20);
        assert_eq!(detection.heavy_window_min, 120);
        assert_eq!(detection.min_samples, 50);
        assert_eq!(detection.cooldown_sec, 60);
        assert_eq!(detection.auto_resolve_sec, 600);
        assert_eq!(detection.max_groups_per_cycle, 42);
        assert!(!detection.security_findings_enabled);
        assert!(!detection.agent_health_enabled);
        assert!(!detection.log_pattern_enabled);
        assert!(detection.safe_mode);
    }

    fn minimal_vars() -> std::collections::HashMap<String, String> {
        super::collect_vars([
            ("POSTGRES_DSN", "postgres://localhost/test"),
            ("NATS_URL", "nats://localhost:4222"),
            ("QUERY_ALERT_HTTP_ADDR", "0.0.0.0:9095"),
            ("OPENSEARCH_URL", "http://localhost:9200"),
            ("OPENSEARCH_INDEX_PREFIX", "doro"),
            ("CLICKHOUSE_DSN", "http://localhost:8123"),
            ("CLICKHOUSE_DATABASE", "doro"),
            ("CLICKHOUSE_TABLE", "logs"),
        ])
    }
}
