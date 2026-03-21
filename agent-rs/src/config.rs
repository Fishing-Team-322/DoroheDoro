use std::{
    collections::BTreeMap,
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
    pub diagnostics: DiagnosticsConfig,
    #[serde(default)]
    pub batch: BatchConfig,
    #[serde(default)]
    pub queues: QueueConfig,
    #[serde(default)]
    pub degraded: DegradedConfig,
    #[serde(default)]
    pub spool: SpoolConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub install: InstallConfig,
    #[serde(default)]
    pub platform: PlatformConfig,
    #[serde(default)]
    pub scope: ScopeConfig,
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
pub struct DiagnosticsConfig {
    #[serde(default)]
    pub interval_sec: u64,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self { interval_sec: 0 }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchConfig {
    #[serde(default = "default_batch_max_events")]
    pub max_events: usize,
    #[serde(default = "default_batch_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_batch_flush_interval_ms")]
    pub flush_interval_ms: u64,
    #[serde(default = "default_batch_compress_threshold_bytes")]
    pub compress_threshold_bytes: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_events: default_batch_max_events(),
            max_bytes: default_batch_max_bytes(),
            flush_interval_ms: default_batch_flush_interval_ms(),
            compress_threshold_bytes: default_batch_compress_threshold_bytes(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    #[serde(default = "default_event_queue_capacity")]
    pub event_capacity: usize,
    #[serde(default = "default_send_queue_capacity")]
    pub send_capacity: usize,
    #[serde(default = "default_event_bytes_soft_limit")]
    pub event_bytes_soft_limit: usize,
    #[serde(default = "default_send_bytes_soft_limit")]
    pub send_bytes_soft_limit: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            event_capacity: default_event_queue_capacity(),
            send_capacity: default_send_queue_capacity(),
            event_bytes_soft_limit: default_event_bytes_soft_limit(),
            send_bytes_soft_limit: default_send_bytes_soft_limit(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DegradedConfig {
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    #[serde(default = "default_server_unavailable_sec")]
    pub server_unavailable_sec: u64,
    #[serde(default = "default_queue_pressure_pct")]
    pub queue_pressure_pct: u8,
    #[serde(default = "default_queue_recover_pct")]
    pub queue_recover_pct: u8,
    #[serde(default = "default_unacked_lag_bytes")]
    pub unacked_lag_bytes: u64,
    #[serde(default = "default_shutdown_spool_grace_sec")]
    pub shutdown_spool_grace_sec: u64,
}

impl Default for DegradedConfig {
    fn default() -> Self {
        Self {
            failure_threshold: default_failure_threshold(),
            server_unavailable_sec: default_server_unavailable_sec(),
            queue_pressure_pct: default_queue_pressure_pct(),
            queue_recover_pct: default_queue_recover_pct(),
            unacked_lag_bytes: default_unacked_lag_bytes(),
            shutdown_spool_grace_sec: default_shutdown_spool_grace_sec(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpoolConfig {
    #[serde(default = "default_spool_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub dir: PathBuf,
    #[serde(default = "default_spool_max_disk_bytes")]
    pub max_disk_bytes: u64,
}

impl Default for SpoolConfig {
    fn default() -> Self {
        Self {
            enabled: default_spool_enabled(),
            dir: PathBuf::new(),
            max_disk_bytes: default_spool_max_disk_bytes(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct TransportConfig {
    #[serde(default)]
    pub mode: TransportMode,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct InstallConfig {
    #[serde(default)]
    pub mode: InstallMode,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            mode: InstallMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct PlatformConfig {
    #[serde(default)]
    pub allow_machine_id: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct ScopeConfig {
    #[serde(default)]
    pub configured_cluster_id: Option<String>,
    #[serde(default)]
    pub configured_cluster_tags: BTreeMap<String, String>,
    #[serde(default)]
    pub cluster_name: Option<String>,
    #[serde(default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub host_labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    #[default]
    Edge,
    Mock,
}

impl TransportMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Edge => "edge",
            Self::Mock => "mock",
        }
    }

    pub fn is_edge(&self) -> bool {
        matches!(self, Self::Edge)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum InstallMode {
    #[default]
    Auto,
    Package,
    Tarball,
    Ansible,
    Dev,
}

impl InstallMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Package => "package",
            Self::Tarball => "tarball",
            Self::Ansible => "ansible",
            Self::Dev => "dev",
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StartAt {
    Beginning,
    #[default]
    End,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub source_id: Option<String>,
    pub path: PathBuf,
    #[serde(default)]
    pub start_at: StartAt,
    pub source: String,
    pub service: String,
    pub severity_hint: String,
}

impl SourceConfig {
    pub fn source_id(&self) -> &str {
        self.source_id
            .as_deref()
            .expect("source_id should be normalized before use")
    }
}

impl AgentConfig {
    pub fn load(path: &Path) -> AppResult<Self> {
        if !path.exists() {
            return Err(AppError::MissingPath(path.to_path_buf()));
        }

        let raw = fs::read_to_string(path)?;
        let mut config: AgentConfig = serde_yaml::from_str(&raw)?;
        config.apply_env_overrides();
        config.normalize();
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
        if let Some(value) = env::var_os("DIAGNOSTICS_INTERVAL_SEC") {
            self.diagnostics.interval_sec = parse_u64(&value, self.diagnostics.interval_sec);
        }
        if let Some(value) = env::var_os("BATCH_MAX_EVENTS") {
            self.batch.max_events = parse_usize(&value, self.batch.max_events);
        }
        if let Some(value) = env::var_os("BATCH_MAX_BYTES") {
            self.batch.max_bytes = parse_usize(&value, self.batch.max_bytes);
        }
        if let Some(value) = env::var_os("BATCH_FLUSH_INTERVAL_MS") {
            self.batch.flush_interval_ms = parse_u64(&value, self.batch.flush_interval_ms);
        } else if let Some(value) = env::var_os("BATCH_FLUSH_INTERVAL_SEC") {
            self.batch.flush_interval_ms =
                parse_u64(&value, self.batch.flush_interval_ms / 1000).saturating_mul(1000);
        }
        if let Some(value) = env::var_os("BATCH_COMPRESS_THRESHOLD_BYTES") {
            self.batch.compress_threshold_bytes =
                parse_usize(&value, self.batch.compress_threshold_bytes);
        }
        if let Some(value) = env::var_os("QUEUE_EVENT_CAPACITY") {
            self.queues.event_capacity = parse_usize(&value, self.queues.event_capacity);
        }
        if let Some(value) = env::var_os("QUEUE_SEND_CAPACITY") {
            self.queues.send_capacity = parse_usize(&value, self.queues.send_capacity);
        }
        if let Some(value) = env::var_os("QUEUE_EVENT_BYTES_SOFT_LIMIT") {
            self.queues.event_bytes_soft_limit =
                parse_usize(&value, self.queues.event_bytes_soft_limit);
        }
        if let Some(value) = env::var_os("QUEUE_SEND_BYTES_SOFT_LIMIT") {
            self.queues.send_bytes_soft_limit =
                parse_usize(&value, self.queues.send_bytes_soft_limit);
        }
        if let Some(value) = env::var_os("DEGRADED_FAILURE_THRESHOLD") {
            self.degraded.failure_threshold = parse_u32(&value, self.degraded.failure_threshold);
        }
        if let Some(value) = env::var_os("DEGRADED_SERVER_UNAVAILABLE_SEC") {
            self.degraded.server_unavailable_sec =
                parse_u64(&value, self.degraded.server_unavailable_sec);
        }
        if let Some(value) = env::var_os("DEGRADED_QUEUE_PRESSURE_PCT") {
            self.degraded.queue_pressure_pct = parse_u8(&value, self.degraded.queue_pressure_pct);
        }
        if let Some(value) = env::var_os("DEGRADED_QUEUE_RECOVER_PCT") {
            self.degraded.queue_recover_pct = parse_u8(&value, self.degraded.queue_recover_pct);
        }
        if let Some(value) = env::var_os("DEGRADED_UNACKED_LAG_BYTES") {
            self.degraded.unacked_lag_bytes = parse_u64(&value, self.degraded.unacked_lag_bytes);
        }
        if let Some(value) = env::var_os("DEGRADED_SHUTDOWN_SPOOL_GRACE_SEC") {
            self.degraded.shutdown_spool_grace_sec =
                parse_u64(&value, self.degraded.shutdown_spool_grace_sec);
        }
        if let Some(value) = env::var_os("SPOOL_ENABLED") {
            self.spool.enabled = parse_bool(&value, self.spool.enabled);
        }
        if let Some(value) = env::var_os("SPOOL_DIR") {
            self.spool.dir = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("SPOOL_MAX_DISK_BYTES") {
            self.spool.max_disk_bytes = parse_u64(&value, self.spool.max_disk_bytes);
        }
        if let Some(value) = env::var_os("TRANSPORT_MODE") {
            let value = value.to_string_lossy().to_ascii_lowercase();
            if value == "mock" {
                self.transport.mode = TransportMode::Mock;
            } else if value == "edge" {
                self.transport.mode = TransportMode::Edge;
            }
        }
        if let Some(value) = env::var_os("INSTALL_MODE") {
            match value.to_string_lossy().to_ascii_lowercase().as_str() {
                "auto" => self.install.mode = InstallMode::Auto,
                "package" => self.install.mode = InstallMode::Package,
                "tarball" => self.install.mode = InstallMode::Tarball,
                "ansible" => self.install.mode = InstallMode::Ansible,
                "dev" => self.install.mode = InstallMode::Dev,
                _ => {}
            }
        }
        if let Some(value) = env::var_os("ALLOW_MACHINE_ID") {
            self.platform.allow_machine_id = parse_bool(&value, self.platform.allow_machine_id);
        }
        if let Some(value) = env::var_os("CLUSTER_ID") {
            self.scope.configured_cluster_id = Some(value.to_string_lossy().into_owned());
        }
        if let Some(value) = env::var_os("CLUSTER_NAME") {
            self.scope.cluster_name = Some(value.to_string_lossy().into_owned());
        }
        if let Some(value) = env::var_os("SERVICE_NAME") {
            self.scope.service_name = Some(value.to_string_lossy().into_owned());
        }
        if let Some(value) = env::var_os("ENVIRONMENT") {
            self.scope.environment = Some(value.to_string_lossy().into_owned());
        }
    }

    fn normalize(&mut self) {
        if self.diagnostics.interval_sec == 0 {
            self.diagnostics.interval_sec = self.heartbeat.interval_sec;
        }
        if self.spool.dir.as_os_str().is_empty() {
            self.spool.dir = self.state_dir.join("spool");
        }
        normalize_optional_string(&mut self.scope.configured_cluster_id);
        normalize_optional_string(&mut self.scope.cluster_name);
        normalize_optional_string(&mut self.scope.service_name);
        normalize_optional_string(&mut self.scope.environment);
        for source in &mut self.sources {
            if source.source_id.is_none() {
                source.source_id = Some(format!("file:{}", source.path.to_string_lossy()));
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
        if self.diagnostics.interval_sec == 0 {
            return Err(AppError::invalid_config(
                "diagnostics.interval_sec must be greater than zero",
            ));
        }
        if self.batch.max_events == 0 {
            return Err(AppError::invalid_config(
                "batch.max_events must be greater than zero",
            ));
        }
        if self.batch.max_bytes == 0 {
            return Err(AppError::invalid_config(
                "batch.max_bytes must be greater than zero",
            ));
        }
        if self.batch.flush_interval_ms == 0 {
            return Err(AppError::invalid_config(
                "batch.flush_interval_ms must be greater than zero",
            ));
        }
        if self.queues.event_capacity == 0 || self.queues.send_capacity == 0 {
            return Err(AppError::invalid_config(
                "queue capacities must be greater than zero",
            ));
        }
        if self.degraded.failure_threshold == 0 {
            return Err(AppError::invalid_config(
                "degraded.failure_threshold must be greater than zero",
            ));
        }
        if self.degraded.queue_pressure_pct == 0 || self.degraded.queue_pressure_pct > 100 {
            return Err(AppError::invalid_config(
                "degraded.queue_pressure_pct must be within 1..=100",
            ));
        }
        if self.degraded.queue_recover_pct >= self.degraded.queue_pressure_pct {
            return Err(AppError::invalid_config(
                "degraded.queue_recover_pct must be less than degraded.queue_pressure_pct",
            ));
        }
        if self.spool.enabled && self.spool.max_disk_bytes == 0 {
            return Err(AppError::invalid_config(
                "spool.max_disk_bytes must be greater than zero when spool is enabled",
            ));
        }
        for source in &self.sources {
            if source.kind != "file" {
                return Err(AppError::invalid_config(format!(
                    "unsupported source type `{}`",
                    source.kind
                )));
            }
            if source.source_id().trim().is_empty() {
                return Err(AppError::invalid_config(
                    "file source `source_id` is required",
                ));
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
        for (key, value) in &self.scope.configured_cluster_tags {
            if key.trim().is_empty() || value.trim().is_empty() {
                return Err(AppError::invalid_config(
                    "scope.configured_cluster_tags keys and values must be non-empty",
                ));
            }
        }
        for (key, value) in &self.scope.host_labels {
            if key.trim().is_empty() || value.trim().is_empty() {
                return Err(AppError::invalid_config(
                    "scope.host_labels keys and values must be non-empty",
                ));
            }
        }
        Ok(())
    }
}

fn normalize_optional_string(value: &mut Option<String>) {
    if value
        .as_ref()
        .map(|current| current.trim().is_empty())
        .unwrap_or(false)
    {
        *value = None;
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

fn default_batch_max_bytes() -> usize {
    512 * 1024
}

fn default_batch_flush_interval_ms() -> u64 {
    2_000
}

fn default_batch_compress_threshold_bytes() -> usize {
    16 * 1024
}

fn default_event_queue_capacity() -> usize {
    4_096
}

fn default_send_queue_capacity() -> usize {
    32
}

fn default_event_bytes_soft_limit() -> usize {
    8 * 1024 * 1024
}

fn default_send_bytes_soft_limit() -> usize {
    16 * 1024 * 1024
}

fn default_failure_threshold() -> u32 {
    3
}

fn default_server_unavailable_sec() -> u64 {
    30
}

fn default_queue_pressure_pct() -> u8 {
    80
}

fn default_queue_recover_pct() -> u8 {
    40
}

fn default_unacked_lag_bytes() -> u64 {
    16 * 1024 * 1024
}

fn default_shutdown_spool_grace_sec() -> u64 {
    5
}

fn default_spool_enabled() -> bool {
    true
}

fn default_spool_max_disk_bytes() -> u64 {
    256 * 1024 * 1024
}

fn parse_bool(value: &std::ffi::OsStr, fallback: bool) -> bool {
    match value.to_string_lossy().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_u8(value: &std::ffi::OsStr, fallback: u8) -> u8 {
    value.to_string_lossy().parse::<u8>().unwrap_or(fallback)
}

fn parse_u32(value: &std::ffi::OsStr, fallback: u32) -> u32 {
    value.to_string_lossy().parse::<u32>().unwrap_or(fallback)
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
        path::PathBuf,
        sync::{LazyLock, Mutex},
    };

    use tempfile::TempDir;

    use super::{AgentConfig, InstallMode, StartAt, TransportMode};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn loads_phase_two_yaml_config() {
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
diagnostics:
  interval_sec: 10
batch:
  max_events: 42
  max_bytes: 10000
  flush_interval_ms: 2500
  compress_threshold_bytes: 2048
queues:
  event_capacity: 256
  send_capacity: 8
  event_bytes_soft_limit: 4096
  send_bytes_soft_limit: 8192
transport:
  mode: "mock"
sources:
  - type: "file"
    source_id: "syslog-main"
    path: "/tmp/test.log"
    start_at: "beginning"
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
        assert_eq!(config.sources[0].start_at, StartAt::Beginning);
        assert_eq!(config.sources[0].source_id(), "syslog-main");
    }

    #[test]
    fn applies_defaults_for_source_id_diagnostics_and_spool_dir() {
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

        let config = AgentConfig::load(&config_path).unwrap();
        assert_eq!(
            config.diagnostics.interval_sec,
            config.heartbeat.interval_sec
        );
        assert_eq!(
            config.spool.dir,
            PathBuf::from("/tmp/doro-agent").join("spool")
        );
        assert_eq!(config.sources[0].source_id(), "file:/tmp/test.log");
        assert_eq!(config.sources[0].start_at, StartAt::End);
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
        env::set_var("BATCH_FLUSH_INTERVAL_MS", "750");
        env::set_var("QUEUE_SEND_CAPACITY", "16");
        env::set_var("SPOOL_ENABLED", "false");
        env::set_var("TRANSPORT_MODE", "mock");
        env::set_var("INSTALL_MODE", "ansible");
        env::set_var("ALLOW_MACHINE_ID", "true");
        env::set_var("CLUSTER_ID", "cluster-a");
        env::set_var("CLUSTER_NAME", "prod");
        env::set_var("SERVICE_NAME", "api");
        env::set_var("ENVIRONMENT", "production");

        let config = AgentConfig::load(&config_path).unwrap();

        env::remove_var("EDGE_URL");
        env::remove_var("EDGE_GRPC_ADDR");
        env::remove_var("BATCH_MAX_EVENTS");
        env::remove_var("BATCH_FLUSH_INTERVAL_MS");
        env::remove_var("QUEUE_SEND_CAPACITY");
        env::remove_var("SPOOL_ENABLED");
        env::remove_var("TRANSPORT_MODE");
        env::remove_var("INSTALL_MODE");
        env::remove_var("ALLOW_MACHINE_ID");
        env::remove_var("CLUSTER_ID");
        env::remove_var("CLUSTER_NAME");
        env::remove_var("SERVICE_NAME");
        env::remove_var("ENVIRONMENT");

        assert_eq!(config.edge_url, "https://edge.example.local");
        assert_eq!(config.edge_grpc_addr, "edge.example.local:7443");
        assert_eq!(config.batch.max_events, 100);
        assert_eq!(config.batch.flush_interval_ms, 750);
        assert_eq!(config.queues.send_capacity, 16);
        assert!(!config.spool.enabled);
        assert_eq!(config.transport.mode, TransportMode::Mock);
        assert_eq!(config.install.mode, InstallMode::Ansible);
        assert!(config.platform.allow_machine_id);
        assert_eq!(
            config.scope.configured_cluster_id.as_deref(),
            Some("cluster-a")
        );
        assert_eq!(config.scope.cluster_name.as_deref(), Some("prod"));
        assert_eq!(config.scope.service_name.as_deref(), Some("api"));
        assert_eq!(config.scope.environment.as_deref(), Some("production"));
    }

    fn clear_test_env() {
        for key in [
            "EDGE_URL",
            "EDGE_GRPC_ADDR",
            "BOOTSTRAP_TOKEN",
            "STATE_DIR",
            "LOG_LEVEL",
            "HEARTBEAT_INTERVAL_SEC",
            "DIAGNOSTICS_INTERVAL_SEC",
            "BATCH_MAX_EVENTS",
            "BATCH_MAX_BYTES",
            "BATCH_FLUSH_INTERVAL_MS",
            "BATCH_FLUSH_INTERVAL_SEC",
            "BATCH_COMPRESS_THRESHOLD_BYTES",
            "QUEUE_EVENT_CAPACITY",
            "QUEUE_SEND_CAPACITY",
            "QUEUE_EVENT_BYTES_SOFT_LIMIT",
            "QUEUE_SEND_BYTES_SOFT_LIMIT",
            "DEGRADED_FAILURE_THRESHOLD",
            "DEGRADED_SERVER_UNAVAILABLE_SEC",
            "DEGRADED_QUEUE_PRESSURE_PCT",
            "DEGRADED_QUEUE_RECOVER_PCT",
            "DEGRADED_UNACKED_LAG_BYTES",
            "DEGRADED_SHUTDOWN_SPOOL_GRACE_SEC",
            "SPOOL_ENABLED",
            "SPOOL_DIR",
            "SPOOL_MAX_DISK_BYTES",
            "TRANSPORT_MODE",
            "INSTALL_MODE",
            "ALLOW_MACHINE_ID",
            "CLUSTER_ID",
            "CLUSTER_NAME",
            "SERVICE_NAME",
            "ENVIRONMENT",
        ] {
            env::remove_var(key);
        }
    }
}
