use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub edge_url: String,
    pub edge_grpc_addr: String,
    pub bootstrap_token: String,
    pub state_dir: PathBuf,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub heartbeat: HeartbeatConfig,
    #[serde(default)]
    pub batch: BatchConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatConfig {
    #[serde(default = "default_heartbeat_interval")]
    pub interval_sec: u64,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_sec: default_heartbeat_interval(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchConfig {
    #[serde(default = "default_batch_max_events")]
    pub max_events: usize,
    #[serde(default = "default_batch_flush_interval")]
    pub flush_interval_sec: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_events: default_batch_max_events(),
            flush_interval_sec: default_batch_flush_interval(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct TransportConfig {
    #[serde(default)]
    pub mode: TransportMode,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    #[default]
    Edge,
    Mock,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub path: PathBuf,
    pub source: String,
    pub service: String,
    pub severity_hint: String,
}

impl AgentConfig {
    pub fn load(path: &Path) -> AppResult<Self> {
        if !path.exists() {
            return Err(AppError::MissingPath(path.to_path_buf()));
        }

        let raw = fs::read_to_string(path)?;
        let mut config: AgentConfig = serde_yaml::from_str(&raw)?;
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    fn apply_env_overrides(&mut self) {
        if let Some(value) = env::var_os("EDGE_URL") {
            self.edge_url = value.to_string_lossy().into_owned();
        }
        if let Some(value) = env::var_os("EDGE_GRPC_ADDR") {
            self.edge_grpc_addr = value.to_string_lossy().into_owned();
        }
        if let Some(value) = env::var_os("BOOTSTRAP_TOKEN") {
            self.bootstrap_token = value.to_string_lossy().into_owned();
        }
        if let Some(value) = env::var_os("STATE_DIR") {
            self.state_dir = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("LOG_LEVEL") {
            self.log_level = value.to_string_lossy().into_owned();
        }
        if let Some(value) = env::var_os("HEARTBEAT_INTERVAL_SEC") {
            self.heartbeat.interval_sec = parse_u64(&value, self.heartbeat.interval_sec);
        }
        if let Some(value) = env::var_os("BATCH_MAX_EVENTS") {
            self.batch.max_events = parse_usize(&value, self.batch.max_events);
        }
        if let Some(value) = env::var_os("BATCH_FLUSH_INTERVAL_SEC") {
            self.batch.flush_interval_sec = parse_u64(&value, self.batch.flush_interval_sec);
        }
        if let Some(value) = env::var_os("TRANSPORT_MODE") {
            let value = value.to_string_lossy().to_ascii_lowercase();
            if value == "mock" {
                self.transport.mode = TransportMode::Mock;
            } else if value == "edge" {
                self.transport.mode = TransportMode::Edge;
            }
        }
    }

    fn validate(&self) -> AppResult<()> {
        if self.edge_url.trim().is_empty() {
            return Err(AppError::invalid_config("edge_url is required"));
        }
        if self.edge_grpc_addr.trim().is_empty() {
            return Err(AppError::invalid_config("edge_grpc_addr is required"));
        }
        if self.bootstrap_token.trim().is_empty() {
            return Err(AppError::invalid_config("bootstrap_token is required"));
        }
        if self.state_dir.as_os_str().is_empty() {
            return Err(AppError::invalid_config("state_dir is required"));
        }
        if self.heartbeat.interval_sec == 0 {
            return Err(AppError::invalid_config(
                "heartbeat.interval_sec must be greater than zero",
            ));
        }
        if self.batch.max_events == 0 {
            return Err(AppError::invalid_config(
                "batch.max_events must be greater than zero",
            ));
        }
        if self.batch.flush_interval_sec == 0 {
            return Err(AppError::invalid_config(
                "batch.flush_interval_sec must be greater than zero",
            ));
        }
        for source in &self.sources {
            if source.kind != "file" {
                return Err(AppError::invalid_config(format!(
                    "unsupported source type `{}`",
                    source.kind
                )));
            }
            if source.path.as_os_str().is_empty() {
                return Err(AppError::invalid_config("file source path is required"));
            }
            if source.source.trim().is_empty() {
                return Err(AppError::invalid_config("file source `source` is required"));
            }
            if source.service.trim().is_empty() {
                return Err(AppError::invalid_config(
                    "file source `service` is required",
                ));
            }
            if source.severity_hint.trim().is_empty() {
                return Err(AppError::invalid_config(
                    "file source `severity_hint` is required",
                ));
            }
        }
        Ok(())
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_heartbeat_interval() -> u64 {
    30
}

fn default_batch_max_events() -> usize {
    500
}

fn default_batch_flush_interval() -> u64 {
    2
}

fn parse_u64(value: &std::ffi::OsStr, fallback: u64) -> u64 {
    value.to_string_lossy().parse::<u64>().unwrap_or(fallback)
}

fn parse_usize(value: &std::ffi::OsStr, fallback: usize) -> usize {
    value.to_string_lossy().parse::<usize>().unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        sync::{LazyLock, Mutex},
    };

    use tempfile::TempDir;

    use super::{AgentConfig, TransportMode};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn loads_yaml_config() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_test_env();
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("agent.yaml");
        fs::write(
            &config_path,
            r#"
edge_url: "http://localhost:8080"
edge_grpc_addr: "localhost:9090"
bootstrap_token: "token"
state_dir: "/tmp/doro-agent"
heartbeat:
  interval_sec: 15
batch:
  max_events: 42
  flush_interval_sec: 3
transport:
  mode: "mock"
sources:
  - type: "file"
    path: "/tmp/test.log"
    source: "syslog"
    service: "demo"
    severity_hint: "info"
"#,
        )
        .unwrap();

        let config = AgentConfig::load(&config_path).unwrap();
        assert_eq!(config.edge_grpc_addr, "localhost:9090");
        assert_eq!(config.batch.max_events, 42);
        assert_eq!(config.transport.mode, TransportMode::Mock);
    }

    #[test]
    fn env_overrides_scalar_fields() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_test_env();
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("agent.yaml");
        fs::write(
            &config_path,
            r#"
edge_url: "http://localhost:8080"
edge_grpc_addr: "localhost:9090"
bootstrap_token: "token"
state_dir: "/tmp/doro-agent"
sources:
  - type: "file"
    path: "/tmp/test.log"
    source: "syslog"
    service: "demo"
    severity_hint: "info"
"#,
        )
        .unwrap();

        env::set_var("EDGE_URL", "https://edge.example.local");
        env::set_var("EDGE_GRPC_ADDR", "edge.example.local:7443");
        env::set_var("BATCH_MAX_EVENTS", "100");
        env::set_var("TRANSPORT_MODE", "mock");

        let config = AgentConfig::load(&config_path).unwrap();

        env::remove_var("EDGE_URL");
        env::remove_var("EDGE_GRPC_ADDR");
        env::remove_var("BATCH_MAX_EVENTS");
        env::remove_var("TRANSPORT_MODE");

        assert_eq!(config.edge_url, "https://edge.example.local");
        assert_eq!(config.edge_grpc_addr, "edge.example.local:7443");
        assert_eq!(config.batch.max_events, 100);
        assert_eq!(config.transport.mode, TransportMode::Mock);
    }

    fn clear_test_env() {
        for key in [
            "EDGE_URL",
            "EDGE_GRPC_ADDR",
            "BOOTSTRAP_TOKEN",
            "STATE_DIR",
            "LOG_LEVEL",
            "HEARTBEAT_INTERVAL_SEC",
            "BATCH_MAX_EVENTS",
            "BATCH_FLUSH_INTERVAL_SEC",
            "TRANSPORT_MODE",
        ] {
            env::remove_var(key);
        }
    }
}
