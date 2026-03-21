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
        })
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
        ])
        .unwrap();

        assert_eq!(config.http_addr, "0.0.0.0:9095");
        assert_eq!(config.shared.nats_url, "nats://localhost:4222");
    }
}
