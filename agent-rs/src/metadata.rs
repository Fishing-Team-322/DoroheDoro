use std::{
    collections::BTreeMap,
    env, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    config::{AgentConfig, InstallMode},
    error::AppResult,
};

pub const CANONICAL_CONFIG_PATH: &str = "/etc/doro-agent/config.yaml";
pub const CANONICAL_ENV_PATH: &str = "/etc/doro-agent/agent.env";
pub const CANONICAL_STATE_DIR: &str = "/var/lib/doro-agent";
pub const CANONICAL_SPOOL_DIR: &str = "/var/lib/doro-agent/spool";
pub const CANONICAL_LOG_DIR: &str = "/var/log/doro-agent";
pub const CANONICAL_PACKAGE_BIN: &str = "/usr/bin/doro-agent";
pub const CANONICAL_SERVICE_UNIT: &str = "doro-agent.service";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMetadata {
    pub agent_version: String,
    pub git_commit: String,
    pub build_id: String,
    pub target_triple: String,
    pub build_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetadata {
    pub os_family: String,
    pub distro_name: Option<String>,
    pub distro_version: Option<String>,
    pub kernel_version: Option<String>,
    pub architecture: String,
    pub hostname: String,
    pub machine_id_hash: Option<String>,
    pub service_manager: String,
    pub systemd_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallMetadata {
    pub configured_mode: String,
    pub resolved_mode: String,
    pub resolution_source: String,
    pub canonical_layout: bool,
    pub systemd_expected: bool,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMetadata {
    pub current_exe: String,
    pub config_path: String,
    pub state_dir: String,
    pub spool_dir: String,
    pub state_db_path: String,
    pub canonical_package_bin: String,
    pub canonical_config_path: String,
    pub canonical_env_path: String,
    pub canonical_state_dir: String,
    pub canonical_spool_dir: String,
    pub canonical_log_dir: String,
    pub service_unit_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetadata {
    pub configured_cluster_id: Option<String>,
    pub cluster_name: Option<String>,
    pub service_name: Option<String>,
    pub environment: Option<String>,
    pub configured_cluster_tags: BTreeMap<String, String>,
    pub effective_cluster_tags: BTreeMap<String, String>,
    pub host_labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompatibilitySnapshot {
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub permission_issues: Vec<String>,
    pub source_path_issues: Vec<String>,
    pub insecure_transport: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityStatusSnapshot {
    pub status: String,
    pub reason: Option<String>,
}

impl Default for IdentityStatusSnapshot {
    fn default() -> Self {
        Self {
            status: "unknown".to_string(),
            reason: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeMetadataContext {
    pub build: BuildMetadata,
    pub platform: PlatformMetadata,
    pub install: InstallMetadata,
    pub paths: PathMetadata,
    pub cluster: ClusterMetadata,
    pub compatibility: CompatibilitySnapshot,
    pub event_enrichment: EventEnrichmentContext,
}

#[derive(Debug, Clone, Default)]
pub struct EventEnrichmentContext {
    base_labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolvedInstallMode {
    Package,
    Tarball,
    Ansible,
    Dev,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct InstallResolution {
    pub resolved_mode: ResolvedInstallMode,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourcePathStatus {
    Readable,
    Missing,
    Unreadable,
}

#[derive(Debug, Clone)]
pub struct SourcePathCheck {
    pub path: PathBuf,
    pub status: SourcePathStatus,
    pub message: Option<String>,
}

impl RuntimeMetadataContext {
    pub fn detect(config: &AgentConfig, config_path: &Path, hostname: &str) -> AppResult<Self> {
        let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("unknown"));
        let paths = detect_paths(config, config_path, &current_exe);
        let install_resolution =
            resolve_install_mode(config.install.mode, config_path, &current_exe, config);
        let install = InstallMetadata {
            configured_mode: config.install.mode.as_str().to_string(),
            resolved_mode: install_resolution.resolved_mode.as_str().to_string(),
            resolution_source: if config.install.mode == InstallMode::Auto {
                "auto".to_string()
            } else {
                "explicit".to_string()
            },
            canonical_layout: config_path == Path::new(CANONICAL_CONFIG_PATH)
                && config.state_dir == Path::new(CANONICAL_STATE_DIR)
                && config.spool.dir == Path::new(CANONICAL_SPOOL_DIR)
                && current_exe == PathBuf::from(CANONICAL_PACKAGE_BIN),
            systemd_expected: matches!(
                install_resolution.resolved_mode,
                ResolvedInstallMode::Package | ResolvedInstallMode::Ansible
            ),
            notes: install_resolution.notes.clone(),
            warnings: install_resolution.warnings.clone(),
        };
        let platform = detect_platform(hostname, config.platform.allow_machine_id)?;
        let cluster = resolve_cluster_metadata(config);
        let compatibility = build_compatibility_snapshot(config, &platform, &install, config_path);
        let event_enrichment = EventEnrichmentContext::from_cluster(&cluster);

        Ok(Self {
            build: BuildMetadata::current(),
            platform,
            install,
            paths,
            cluster,
            compatibility,
            event_enrichment,
        })
    }
}

impl BuildMetadata {
    pub fn current() -> Self {
        Self {
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: env!("DORO_AGENT_GIT_COMMIT").to_string(),
            build_id: env!("DORO_AGENT_BUILD_ID").to_string(),
            target_triple: env!("DORO_AGENT_TARGET_TRIPLE").to_string(),
            build_profile: env!("DORO_AGENT_BUILD_PROFILE").to_string(),
        }
    }
}

impl EventEnrichmentContext {
    pub fn from_cluster(cluster: &ClusterMetadata) -> Self {
        let mut base_labels = BTreeMap::new();
        if let Some(value) = cluster.configured_cluster_id.clone() {
            base_labels.insert("cluster_id".to_string(), value);
        }
        if let Some(value) = cluster.cluster_name.clone() {
            base_labels.insert("cluster_name".to_string(), value);
        }
        if let Some(value) = cluster.service_name.clone() {
            base_labels.insert("service_name".to_string(), value);
        }
        if let Some(value) = cluster.environment.clone() {
            base_labels.insert("environment".to_string(), value);
        }
        for (key, value) in &cluster.host_labels {
            base_labels.insert(format!("host_labels.{key}"), value.clone());
        }

        Self { base_labels }
    }

    pub fn labels_for_source(
        &self,
        path: &str,
        source: &str,
        source_id: &str,
    ) -> BTreeMap<String, String> {
        let mut labels = self.base_labels.clone();
        labels.insert("path".to_string(), path.to_string());
        labels.insert("source".to_string(), source.to_string());
        labels.insert("source_id".to_string(), source_id.to_string());
        labels
    }
}

impl ResolvedInstallMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Tarball => "tarball",
            Self::Ansible => "ansible",
            Self::Dev => "dev",
            Self::Unknown => "unknown",
        }
    }
}

pub fn detect_paths(config: &AgentConfig, config_path: &Path, current_exe: &Path) -> PathMetadata {
    let state_db_path = config.state_dir.join("state.db");

    PathMetadata {
        current_exe: current_exe.display().to_string(),
        config_path: config_path.display().to_string(),
        state_dir: config.state_dir.display().to_string(),
        spool_dir: config.spool.dir.display().to_string(),
        state_db_path: state_db_path.display().to_string(),
        canonical_package_bin: CANONICAL_PACKAGE_BIN.to_string(),
        canonical_config_path: CANONICAL_CONFIG_PATH.to_string(),
        canonical_env_path: CANONICAL_ENV_PATH.to_string(),
        canonical_state_dir: CANONICAL_STATE_DIR.to_string(),
        canonical_spool_dir: CANONICAL_SPOOL_DIR.to_string(),
        canonical_log_dir: CANONICAL_LOG_DIR.to_string(),
        service_unit_name: CANONICAL_SERVICE_UNIT.to_string(),
    }
}

pub fn resolve_install_mode(
    configured_mode: InstallMode,
    config_path: &Path,
    current_exe: &Path,
    config: &AgentConfig,
) -> InstallResolution {
    if configured_mode != InstallMode::Auto {
        return InstallResolution {
            resolved_mode: match configured_mode {
                InstallMode::Package => ResolvedInstallMode::Package,
                InstallMode::Tarball => ResolvedInstallMode::Tarball,
                InstallMode::Ansible => ResolvedInstallMode::Ansible,
                InstallMode::Dev => ResolvedInstallMode::Dev,
                InstallMode::Auto => ResolvedInstallMode::Unknown,
            },
            notes: vec![format!(
                "install mode pinned by config/env override to `{}`",
                configured_mode.as_str()
            )],
            warnings: Vec::new(),
        };
    }

    let config_is_canonical = config_path == Path::new(CANONICAL_CONFIG_PATH);
    let state_is_canonical = config.state_dir == Path::new(CANONICAL_STATE_DIR);
    let spool_is_canonical = config.spool.dir == Path::new(CANONICAL_SPOOL_DIR);
    let exe_is_canonical = current_exe == Path::new(CANONICAL_PACKAGE_BIN);

    if config_is_canonical && state_is_canonical && spool_is_canonical && exe_is_canonical {
        return InstallResolution {
            resolved_mode: ResolvedInstallMode::Package,
            notes: vec!["detected canonical package-managed layout".to_string()],
            warnings: Vec::new(),
        };
    }

    if path_looks_like_dev(current_exe)
        || path_looks_like_dev(config_path)
        || path_looks_like_dev(&config.state_dir)
    {
        return InstallResolution {
            resolved_mode: ResolvedInstallMode::Dev,
            notes: vec!["detected local development layout".to_string()],
            warnings: Vec::new(),
        };
    }

    if looks_like_tarball_layout(current_exe, config_path) {
        return InstallResolution {
            resolved_mode: ResolvedInstallMode::Tarball,
            notes: vec!["detected colocated tarball-style layout".to_string()],
            warnings: Vec::new(),
        };
    }

    let mut warnings = Vec::new();
    if !exe_is_canonical {
        warnings.push(format!(
            "binary path `{}` is not the canonical package path `{CANONICAL_PACKAGE_BIN}`",
            current_exe.display()
        ));
    }
    if !config_is_canonical {
        warnings.push(format!(
            "config path `{}` is non-canonical for package installs",
            config_path.display()
        ));
    }

    InstallResolution {
        resolved_mode: ResolvedInstallMode::Unknown,
        notes: vec!["install mode could not be resolved from local layout".to_string()],
        warnings,
    }
}

pub fn detect_platform(hostname: &str, allow_machine_id: bool) -> AppResult<PlatformMetadata> {
    let os_release = detect_os_release();
    let systemd_detected = detect_systemd();

    Ok(PlatformMetadata {
        os_family: std::env::consts::OS.to_string(),
        distro_name: os_release.name,
        distro_version: os_release.version_id,
        kernel_version: detect_kernel_version(),
        architecture: std::env::consts::ARCH.to_string(),
        hostname: hostname.to_string(),
        machine_id_hash: detect_machine_id(allow_machine_id),
        service_manager: if systemd_detected {
            "systemd".to_string()
        } else {
            "unknown".to_string()
        },
        systemd_detected,
    })
}

pub fn detect_source_paths(config: &AgentConfig) -> Vec<SourcePathCheck> {
    config
        .sources
        .iter()
        .map(|source| {
            let path = source.path.clone();
            if !path.exists() {
                return SourcePathCheck {
                    path,
                    status: SourcePathStatus::Missing,
                    message: Some(
                        "source path is missing and will start in waiting mode".to_string(),
                    ),
                };
            }

            match fs::File::open(&path) {
                Ok(_) => SourcePathCheck {
                    path,
                    status: SourcePathStatus::Readable,
                    message: None,
                },
                Err(error) => SourcePathCheck {
                    path,
                    status: SourcePathStatus::Unreadable,
                    message: Some(error.to_string()),
                },
            }
        })
        .collect()
}

pub fn can_read_file(path: &Path) -> bool {
    fs::File::open(path).is_ok()
}

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

pub fn directory_write_access(path: &Path) -> bool {
    let target = if path.exists() {
        path
    } else {
        path.parent().unwrap_or(path)
    };

    if !target.exists() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let raw = target.as_os_str().as_bytes();
        let Ok(c_path) = CString::new(raw) else {
            return false;
        };

        // SAFETY: `c_path` is a valid, nul-terminated path buffer for `access(2)`.
        unsafe { libc::access(c_path.as_ptr(), libc::W_OK) == 0 }
    }

    #[cfg(not(unix))]
    {
        fs::metadata(target)
            .map(|metadata| !metadata.permissions().readonly())
            .unwrap_or(false)
    }
}

pub fn resolve_cluster_metadata(config: &AgentConfig) -> ClusterMetadata {
    ClusterMetadata {
        configured_cluster_id: config.scope.configured_cluster_id.clone(),
        cluster_name: config.scope.cluster_name.clone(),
        service_name: config.scope.service_name.clone(),
        environment: config.scope.environment.clone(),
        configured_cluster_tags: config.scope.configured_cluster_tags.clone(),
        effective_cluster_tags: config.scope.configured_cluster_tags.clone(),
        host_labels: config.scope.host_labels.clone(),
    }
}

fn build_compatibility_snapshot(
    config: &AgentConfig,
    platform: &PlatformMetadata,
    install: &InstallMetadata,
    config_path: &Path,
) -> CompatibilitySnapshot {
    let mut snapshot = CompatibilitySnapshot {
        notes: install.notes.clone(),
        warnings: install.warnings.clone(),
        errors: Vec::new(),
        permission_issues: Vec::new(),
        source_path_issues: Vec::new(),
        insecure_transport: config.transport.mode.is_edge()
            && config.edge_url.starts_with("http://"),
    };

    if snapshot.insecure_transport {
        snapshot.warnings.push(format!(
            "edge_url `{}` uses plain HTTP; TLS is recommended for deployed agents",
            config.edge_url
        ));
    }

    if !can_read_file(config_path) {
        snapshot.errors.push(format!(
            "config path `{}` is not readable",
            config_path.display()
        ));
    }

    if !directory_write_access(&config.state_dir) {
        snapshot.permission_issues.push(format!(
            "state_dir `{}` is not writable by the current user",
            config.state_dir.display()
        ));
    }

    if config.spool.enabled && !directory_write_access(&config.spool.dir) {
        snapshot.permission_issues.push(format!(
            "spool dir `{}` is not writable by the current user",
            config.spool.dir.display()
        ));
    }

    for source in detect_source_paths(config) {
        match source.status {
            SourcePathStatus::Readable => {}
            SourcePathStatus::Missing => snapshot.source_path_issues.push(format!(
                "source path `{}` is missing: {}",
                source.path.display(),
                source
                    .message
                    .unwrap_or_else(|| "path not found".to_string())
            )),
            SourcePathStatus::Unreadable => snapshot.source_path_issues.push(format!(
                "source path `{}` is unreadable: {}",
                source.path.display(),
                source
                    .message
                    .unwrap_or_else(|| "permission denied".to_string())
            )),
        }
    }

    if install.systemd_expected && !platform.systemd_detected {
        snapshot.warnings.push(
            "systemd-managed install mode is expected, but no systemd environment was detected"
                .to_string(),
        );
    }

    snapshot
}

fn detect_os_release() -> OsRelease {
    for path in ["/etc/os-release", "/usr/lib/os-release"] {
        if let Ok(content) = fs::read_to_string(path) {
            return parse_os_release(&content);
        }
    }

    OsRelease::default()
}

fn detect_kernel_version() -> Option<String> {
    if let Ok(value) = fs::read_to_string("/proc/sys/kernel/osrelease") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn detect_machine_id(allow_machine_id: bool) -> Option<String> {
    if !allow_machine_id {
        return None;
    }

    for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(content) = fs::read_to_string(path) {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut hasher = Sha256::new();
            hasher.update(trimmed.as_bytes());
            return Some(hex_encode(&hasher.finalize()));
        }
    }

    None
}

fn detect_systemd() -> bool {
    Path::new("/run/systemd/system").exists()
        || env::var_os("INVOCATION_ID").is_some()
        || env::var_os("JOURNAL_STREAM").is_some()
        || env::var_os("NOTIFY_SOCKET").is_some()
}

fn looks_like_tarball_layout(current_exe: &Path, config_path: &Path) -> bool {
    let Some(exe_dir) = current_exe.parent() else {
        return false;
    };
    let Some(config_dir) = config_path.parent() else {
        return false;
    };

    config_dir == exe_dir
        || config_dir.parent() == Some(exe_dir)
        || exe_dir.parent() == Some(config_dir)
}

fn path_looks_like_dev(path: &Path) -> bool {
    let temp_dir = env::temp_dir();
    if path.starts_with(&temp_dir) {
        return true;
    }

    path.components().any(|component| match component {
        Component::Normal(value) => {
            let segment = value.to_string_lossy().to_ascii_lowercase();
            matches!(
                segment.as_str(),
                "target" | "debug" | "release" | "agent-rs" | "dorohedoro"
            )
        }
        _ => false,
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(nibble_to_hex(byte >> 4));
        output.push(nibble_to_hex(byte & 0x0f));
    }
    output
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}

#[derive(Debug, Clone, Default)]
struct OsRelease {
    name: Option<String>,
    version_id: Option<String>,
}

fn parse_os_release(input: &str) -> OsRelease {
    let mut values = BTreeMap::new();
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        values.insert(
            key.trim().to_string(),
            unquote_os_release_value(value.trim()),
        );
    }

    OsRelease {
        name: values
            .get("NAME")
            .cloned()
            .or_else(|| values.get("ID").cloned()),
        version_id: values
            .get("VERSION_ID")
            .cloned()
            .or_else(|| values.get("BUILD_ID").cloned()),
    }
}

fn unquote_os_release_value(value: &str) -> String {
    value
        .trim_matches('"')
        .trim_matches('\'')
        .replace("\\\"", "\"")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        hex_encode, parse_os_release, resolve_install_mode, BuildMetadata, ResolvedInstallMode,
        CANONICAL_PACKAGE_BIN,
    };
    use crate::config::{
        AgentConfig, InstallConfig, InstallMode, PlatformConfig, ScopeConfig, SourceConfig,
        SpoolConfig, StartAt, TransportConfig, TransportMode,
    };

    fn test_config() -> AgentConfig {
        AgentConfig {
            edge_url: "https://edge.example.local".to_string(),
            edge_grpc_addr: "edge.example.local:7443".to_string(),
            bootstrap_token: "token".to_string(),
            state_dir: PathBuf::from("/var/lib/doro-agent"),
            log_level: "info".to_string(),
            heartbeat: Default::default(),
            diagnostics: Default::default(),
            batch: Default::default(),
            queues: Default::default(),
            degraded: Default::default(),
            spool: SpoolConfig {
                enabled: true,
                dir: PathBuf::from("/var/lib/doro-agent/spool"),
                max_disk_bytes: 1,
            },
            transport: TransportConfig {
                mode: TransportMode::Edge,
            },
            install: InstallConfig {
                mode: InstallMode::Auto,
            },
            platform: PlatformConfig {
                allow_machine_id: false,
            },
            scope: ScopeConfig::default(),
            sources: vec![SourceConfig {
                kind: "file".to_string(),
                source_id: Some("file:/var/log/syslog".to_string()),
                path: PathBuf::from("/var/log/syslog"),
                start_at: StartAt::End,
                source: "syslog".to_string(),
                service: "host".to_string(),
                severity_hint: "info".to_string(),
            }],
        }
    }

    #[test]
    fn parses_os_release() {
        let release = parse_os_release(
            r#"
NAME="Astra Linux"
ID=astra
VERSION_ID="1.8"
"#,
        );

        assert_eq!(release.name.as_deref(), Some("Astra Linux"));
        assert_eq!(release.version_id.as_deref(), Some("1.8"));
    }

    #[test]
    fn resolves_package_mode_from_canonical_layout() {
        let config = test_config();
        let resolution = resolve_install_mode(
            InstallMode::Auto,
            Path::new("/etc/doro-agent/config.yaml"),
            Path::new(CANONICAL_PACKAGE_BIN),
            &config,
        );

        assert_eq!(resolution.resolved_mode, ResolvedInstallMode::Package);
    }

    #[test]
    fn resolves_dev_mode_from_target_binary() {
        let mut config = test_config();
        config.state_dir = PathBuf::from("/tmp/doro-agent");
        config.spool.dir = PathBuf::from("/tmp/doro-agent/spool");
        let resolution = resolve_install_mode(
            InstallMode::Auto,
            Path::new("C:/develop/DoroheDoro/agent-rs/config/agent.yaml"),
            Path::new("C:/develop/DoroheDoro/agent-rs/target/debug/doro-agent"),
            &config,
        );

        assert_eq!(resolution.resolved_mode, ResolvedInstallMode::Dev);
    }

    #[test]
    fn resolves_tarball_mode_from_colocated_layout() {
        let mut config = test_config();
        config.state_dir = PathBuf::from("/opt/doro-agent/state");
        config.spool.dir = PathBuf::from("/opt/doro-agent/state/spool");
        let resolution = resolve_install_mode(
            InstallMode::Auto,
            Path::new("/opt/doro-agent/config.yaml"),
            Path::new("/opt/doro-agent/doro-agent"),
            &config,
        );

        assert_eq!(resolution.resolved_mode, ResolvedInstallMode::Tarball);
    }

    #[test]
    fn resolves_unknown_mode_when_layout_is_ambiguous() {
        let mut config = test_config();
        config.state_dir = PathBuf::from("/srv/doro-agent/state");
        config.spool.dir = PathBuf::from("/srv/doro-agent/spool");
        let resolution = resolve_install_mode(
            InstallMode::Auto,
            Path::new("/srv/etc/doro-agent/config.yaml"),
            Path::new("/srv/bin/doro-agent"),
            &config,
        );

        assert_eq!(resolution.resolved_mode, ResolvedInstallMode::Unknown);
        assert!(!resolution.warnings.is_empty());
    }

    #[test]
    fn build_metadata_is_populated() {
        let metadata = BuildMetadata::current();
        assert!(!metadata.agent_version.is_empty());
        assert!(!metadata.target_triple.is_empty());
        assert!(!metadata.build_profile.is_empty());
    }

    #[test]
    fn hex_encodes_bytes() {
        assert_eq!(hex_encode(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    #[test]
    fn enrichment_context_uses_scope_labels_only() {
        let mut config = test_config();
        config.scope.configured_cluster_id = Some("cluster-a".to_string());
        config.scope.cluster_name = Some("prod".to_string());
        config.scope.service_name = Some("api".to_string());
        config.scope.environment = Some("production".to_string());
        config
            .scope
            .host_labels
            .insert("role".to_string(), "edge".to_string());
        config
            .scope
            .configured_cluster_tags
            .insert("tenant".to_string(), "team-a".to_string());

        let cluster = super::resolve_cluster_metadata(&config);
        let context = super::EventEnrichmentContext::from_cluster(&cluster);
        let labels = context.labels_for_source("/var/log/syslog", "syslog", "file:/var/log/syslog");

        assert_eq!(
            labels.get("cluster_id").map(String::as_str),
            Some("cluster-a")
        );
        assert_eq!(labels.get("cluster_name").map(String::as_str), Some("prod"));
        assert_eq!(labels.get("service_name").map(String::as_str), Some("api"));
        assert_eq!(
            labels.get("environment").map(String::as_str),
            Some("production")
        );
        assert_eq!(
            labels.get("host_labels.role").map(String::as_str),
            Some("edge")
        );
        assert!(labels.get("cluster_tags.tenant").is_none());
    }
}
