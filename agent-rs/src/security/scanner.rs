use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::{SecurityScanConfig, SecurityScanProfile};

use super::{
    SecurityAssetVersion, SecurityFinding, SecurityFindingSummary, SecurityMisconfigurationCheck,
    SecurityPackageRule, SecurityPortState, SecurityPostureReport, SecurityRulesFile,
    SecurityRulesLoadedEvent, SecurityRuntimeIssue, SecurityScanFailureEvent,
    SecurityScanSkippedEvent, SecurityScanStateRecord, SecuritySeverity,
    SECURITY_POSTURE_REPORT_EVENT, SECURITY_POSTURE_RULES_LOADED_EVENT,
    SECURITY_POSTURE_SCAN_FAILED_EVENT, SECURITY_POSTURE_SCAN_SKIPPED_EVENT,
    SECURITY_SCHEMA_VERSION,
};

const SECURITY_REPORT_FILE: &str = "security/last-report.json";

#[derive(Debug, Clone)]
pub struct SecurityScanContext {
    pub agent_id: String,
    pub hostname: String,
    pub state_dir: PathBuf,
    pub config: SecurityScanConfig,
}

#[derive(Debug, Clone, Default)]
pub struct SecurityScanArtifacts {
    pub report: Option<SecurityPostureReport>,
    pub failure: Option<SecurityScanFailureEvent>,
    pub skipped: Option<SecurityScanSkippedEvent>,
    pub rules_loaded: Option<SecurityRulesLoadedEvent>,
    pub state: SecurityScanStateRecord,
    pub report_json: Option<String>,
    pub report_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
struct RulesCatalog {
    digest: String,
    package_rule_count: usize,
    watchlist: Vec<String>,
    rules_by_name: BTreeMap<String, SecurityPackageRule>,
}

#[derive(Debug, Clone)]
struct PackageObservation {
    asset: SecurityAssetVersion,
    finding: Option<SecurityFinding>,
    runtime_issue: Option<SecurityRuntimeIssue>,
}

#[derive(Debug)]
struct CommandCapture {
    success: bool,
    stdout: String,
    stderr: String,
}

pub fn run_security_scan(context: &SecurityScanContext) -> SecurityScanArtifacts {
    let created_at = Utc::now();
    if !context.config.enabled {
        let event = build_skipped_event(
            &context.agent_id,
            &context.hostname,
            context.config.profile.as_str(),
            "disabled_by_config",
            "security posture scan is disabled by configuration",
            None,
            created_at,
        );
        return SecurityScanArtifacts {
            skipped: Some(event),
            state: SecurityScanStateRecord {
                last_status: Some("disabled".to_string()),
                last_status_reason: Some("disabled_by_config".to_string()),
                updated_at_unix_ms: created_at.timestamp_millis(),
                ..SecurityScanStateRecord::default()
            },
            ..SecurityScanArtifacts::default()
        };
    }

    if !cfg!(target_os = "linux") {
        let event = build_skipped_event(
            &context.agent_id,
            &context.hostname,
            context.config.profile.as_str(),
            "unsupported_platform",
            "security posture scan currently supports Linux hosts only",
            None,
            created_at,
        );
        return SecurityScanArtifacts {
            skipped: Some(event),
            state: SecurityScanStateRecord {
                last_status: Some("skipped".to_string()),
                last_status_reason: Some("unsupported_platform".to_string()),
                updated_at_unix_ms: created_at.timestamp_millis(),
                ..SecurityScanStateRecord::default()
            },
            ..SecurityScanArtifacts::default()
        };
    }

    let started_at = Utc::now();
    let started_at_unix_ms = started_at.timestamp_millis();
    let deadline = Instant::now() + Duration::from_secs(context.config.timeout_sec);

    match execute_scan(context, started_at, deadline) {
        Ok(mut artifacts) => {
            artifacts.state.last_started_at_unix_ms = Some(started_at_unix_ms);
            if artifacts.state.updated_at_unix_ms == 0 {
                artifacts.state.updated_at_unix_ms = Utc::now().timestamp_millis();
            }
            artifacts
        }
        Err((error_code, error_message)) => {
            let finished_at = Utc::now();
            let event = SecurityScanFailureEvent {
                schema_version: SECURITY_SCHEMA_VERSION.to_string(),
                event_name: SECURITY_POSTURE_SCAN_FAILED_EVENT.to_string(),
                event_id: Uuid::new_v4().to_string(),
                created_at: iso_timestamp(finished_at),
                agent_id: context.agent_id.clone(),
                hostname: context.hostname.clone(),
                profile: context.config.profile.as_str().to_string(),
                started_at: iso_timestamp(started_at),
                finished_at: Some(iso_timestamp(finished_at)),
                error_kind: "scan_execution".to_string(),
                error_code,
                error_message: error_message.clone(),
                retry_backoff_sec: None,
            };
            SecurityScanArtifacts {
                failure: Some(event),
                state: SecurityScanStateRecord {
                    last_started_at_unix_ms: Some(started_at_unix_ms),
                    last_finished_at_unix_ms: Some(finished_at.timestamp_millis()),
                    last_status: Some("failed".to_string()),
                    last_status_reason: Some(error_message),
                    updated_at_unix_ms: finished_at.timestamp_millis(),
                    ..SecurityScanStateRecord::default()
                },
                ..SecurityScanArtifacts::default()
            }
        }
    }
}

fn execute_scan(
    context: &SecurityScanContext,
    started_at: chrono::DateTime<Utc>,
    deadline: Instant,
) -> Result<SecurityScanArtifacts, (String, String)> {
    let mut runtime_health = Vec::new();
    let rules = load_rules_catalog(
        &context.config.version_rules_path,
        &context.config.package_watchlist,
        &mut runtime_health,
    )
    .map_err(|message| ("rules_load_failed".to_string(), message))?;
    ensure_not_expired(deadline, "scan timeout before port inspection")?;

    let (port_states, mut findings, port_issues) = scan_ports(&context.config);
    runtime_health.extend(port_issues);
    ensure_not_expired(deadline, "scan timeout before package inspection")?;

    let (asset_versions, package_findings, package_issues) = scan_packages(&context.config, &rules);
    findings.extend(package_findings);
    runtime_health.extend(package_issues);
    ensure_not_expired(deadline, "scan timeout before hardening checks")?;

    let (misconfig_checks, misconfig_findings, misconfig_issues) =
        scan_misconfigurations(context.config.profile);
    findings.extend(misconfig_findings);
    runtime_health.extend(misconfig_issues);
    ensure_not_expired(deadline, "scan timeout before report assembly")?;

    let finished_at = Utc::now();
    let duration_ms = finished_at
        .timestamp_millis()
        .saturating_sub(started_at.timestamp_millis())
        .max(0) as u64;
    let summary = summarize_findings(&findings);
    let status = if summary.total == 0 {
        "clean".to_string()
    } else {
        "findings_detected".to_string()
    };
    let report = SecurityPostureReport {
        schema_version: SECURITY_SCHEMA_VERSION.to_string(),
        event_name: SECURITY_POSTURE_REPORT_EVENT.to_string(),
        report_id: Uuid::new_v4().to_string(),
        created_at: iso_timestamp(finished_at),
        agent_id: context.agent_id.clone(),
        hostname: context.hostname.clone(),
        profile: context.config.profile.as_str().to_string(),
        interval_sec: context.config.interval_sec,
        started_at: iso_timestamp(started_at),
        finished_at: iso_timestamp(finished_at),
        duration_ms,
        status: status.clone(),
        port_states,
        asset_versions,
        misconfig_checks,
        runtime_health,
        findings,
        summary: summary.clone(),
    };
    let report_json = serde_json::to_string_pretty(&report)
        .map_err(|error| ("report_serialize_failed".to_string(), error.to_string()))?;

    let state = SecurityScanStateRecord {
        last_started_at_unix_ms: Some(started_at.timestamp_millis()),
        last_finished_at_unix_ms: Some(finished_at.timestamp_millis()),
        last_status: Some("completed".to_string()),
        last_status_reason: Some(status),
        last_report_id: Some(report.report_id.clone()),
        last_rules_loaded_at_unix_ms: if rules.package_rule_count > 0 {
            Some(finished_at.timestamp_millis())
        } else {
            None
        },
        last_rules_digest: if rules.package_rule_count > 0 {
            Some(rules.digest.clone())
        } else {
            None
        },
        updated_at_unix_ms: finished_at.timestamp_millis(),
        summary,
        ..SecurityScanStateRecord::default()
    };

    Ok(SecurityScanArtifacts {
        rules_loaded: if rules.package_rule_count > 0 {
            Some(SecurityRulesLoadedEvent {
                schema_version: SECURITY_SCHEMA_VERSION.to_string(),
                event_name: SECURITY_POSTURE_RULES_LOADED_EVENT.to_string(),
                event_id: Uuid::new_v4().to_string(),
                created_at: iso_timestamp(finished_at),
                agent_id: context.agent_id.clone(),
                hostname: context.hostname.clone(),
                source_path: context.config.version_rules_path.display().to_string(),
                rules_digest: rules.digest,
                package_rule_count: rules.package_rule_count,
                watchlist: rules.watchlist,
            })
        } else {
            None
        },
        report_path: if context.config.persist_last_report {
            Some(context.state_dir.join(SECURITY_REPORT_FILE))
        } else {
            None
        },
        report_json: Some(report_json),
        report: Some(report),
        state,
        ..SecurityScanArtifacts::default()
    })
}

fn load_rules_catalog(
    path: &Path,
    watchlist: &[String],
    runtime_health: &mut Vec<SecurityRuntimeIssue>,
) -> Result<RulesCatalog, String> {
    if !path.exists() {
        runtime_health.push(SecurityRuntimeIssue {
            issue_id: Uuid::new_v4().to_string(),
            issue_kind: "rules_file_missing".to_string(),
            severity: SecuritySeverity::Info,
            detail: format!(
                "security rules file `{}` is missing; version thresholds are unavailable",
                path.display()
            ),
        });
        return Ok(RulesCatalog {
            watchlist: watchlist.to_vec(),
            ..RulesCatalog::default()
        });
    }

    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read security rules `{}`: {error}",
            path.display()
        )
    })?;
    let file: SecurityRulesFile = serde_yaml::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse security rules `{}`: {error}",
            path.display()
        )
    })?;
    if file.schema_version != SECURITY_SCHEMA_VERSION {
        return Err(format!(
            "unsupported security rules schema version `{}`",
            file.schema_version
        ));
    }

    let mut digest = Sha256::new();
    digest.update(raw.as_bytes());
    let digest = format!("{:x}", digest.finalize());

    let mut rules_by_name = BTreeMap::new();
    for rule in &file.packages {
        rules_by_name.insert(rule.name.to_ascii_lowercase(), rule.clone());
        for alias in &rule.aliases {
            rules_by_name.insert(alias.to_ascii_lowercase(), rule.clone());
        }
    }

    Ok(RulesCatalog {
        digest,
        package_rule_count: file.packages.len(),
        watchlist: watchlist.to_vec(),
        rules_by_name,
    })
}

fn build_skipped_event(
    agent_id: &str,
    hostname: &str,
    profile: &str,
    reason_code: &str,
    reason_message: &str,
    retry_after_sec: Option<u64>,
    created_at: chrono::DateTime<Utc>,
) -> SecurityScanSkippedEvent {
    SecurityScanSkippedEvent {
        schema_version: SECURITY_SCHEMA_VERSION.to_string(),
        event_name: SECURITY_POSTURE_SCAN_SKIPPED_EVENT.to_string(),
        event_id: Uuid::new_v4().to_string(),
        created_at: iso_timestamp(created_at),
        agent_id: agent_id.to_string(),
        hostname: hostname.to_string(),
        profile: profile.to_string(),
        reason_code: reason_code.to_string(),
        reason_message: reason_message.to_string(),
        retry_after_sec,
    }
}

fn ensure_not_expired(deadline: Instant, message: &str) -> Result<(), (String, String)> {
    if Instant::now() > deadline {
        Err(("scan_timeout".to_string(), message.to_string()))
    } else {
        Ok(())
    }
}

fn summarize_findings(findings: &[SecurityFinding]) -> SecurityFindingSummary {
    let mut summary = SecurityFindingSummary::default();
    for finding in findings {
        summary.observe(finding.severity);
    }
    summary
}

pub fn persist_report(path: &Path, report_json: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, report_json).map_err(|error| error.to_string())
}

fn iso_timestamp(timestamp: chrono::DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn scan_ports(
    config: &SecurityScanConfig,
) -> (
    Vec<SecurityPortState>,
    Vec<SecurityFinding>,
    Vec<SecurityRuntimeIssue>,
) {
    let mut states = BTreeMap::new();
    let mut runtime_issues = Vec::new();
    for (path, protocol, tcp_only) in [
        ("/proc/net/tcp", "tcp", true),
        ("/proc/net/tcp6", "tcp6", true),
        ("/proc/net/udp", "udp", false),
        ("/proc/net/udp6", "udp6", false),
    ] {
        if let Ok(entries) = read_socket_table(path, protocol, tcp_only) {
            for entry in entries {
                let exposure = classify_exposure(&entry.listen_address).to_string();
                states.insert(
                    format!("{}:{}:{}", entry.protocol, entry.listen_address, entry.port),
                    SecurityPortState {
                        port_id: stable_id(
                            "port",
                            &[
                                &entry.protocol,
                                &entry.listen_address,
                                &entry.port.to_string(),
                            ],
                        ),
                        protocol: entry.protocol,
                        listen_address: entry.listen_address,
                        port: entry.port,
                        exposure,
                        inode: Some(entry.inode),
                        process_id: None,
                        process_name: None,
                        executable_path: None,
                        service_unit: None,
                        owner_uid: None,
                        detail: None,
                    },
                );
            }
        }
    }

    let owners = resolve_socket_owners(
        &states
            .values()
            .filter_map(|state| state.inode)
            .collect::<Vec<_>>(),
        &mut runtime_issues,
    );
    for state in states.values_mut() {
        if let Some(inode) = state.inode {
            if let Some(owner) = owners.get(&inode) {
                state.process_id = Some(owner.pid);
                state.process_name = Some(owner.process_name.clone());
                state.executable_path = owner.executable_path.clone();
                state.service_unit = owner.service_unit.clone();
                state.owner_uid = owner.owner_uid;
                state.detail = Some(owner.detail());
            }
        }
    }

    let mut findings = Vec::new();
    for port_state in states.values() {
        if config.blocked_ports.contains(&port_state.port) {
            findings.push(SecurityFinding {
                finding_id: stable_id(
                    "finding",
                    &[
                        "ports",
                        "blocked",
                        &port_state.protocol,
                        &port_state.port.to_string(),
                        &port_state.listen_address,
                    ],
                ),
                finding_fingerprint: Some(stable_id(
                    "fingerprint",
                    &[
                        "ports",
                        "blocked",
                        &port_state.protocol,
                        &port_state.port.to_string(),
                        &port_state.listen_address,
                    ],
                )),
                category: "ports".to_string(),
                severity: if port_state.exposure == "all_interfaces" {
                    SecuritySeverity::Critical
                } else {
                    SecuritySeverity::High
                },
                title: format!("blocked port {} is listening", port_state.port),
                detail: format!(
                    "{} is listening on {}:{}",
                    port_state.protocol, port_state.listen_address, port_state.port
                ),
                asset_type: "socket".to_string(),
                asset_name: format!("{}:{}", port_state.protocol, port_state.port),
                observed_value: Some(port_state.listen_address.clone()),
                expected_value: Some("port must not listen".to_string()),
                check_id: format!("port-blocked-{}", port_state.port),
                remediation: Some("stop the listener or remove the port from the blocked list if it is intentionally exposed".to_string()),
                evidence: Some(port_evidence(port_state)),
            });
        } else if !config.allowed_ports.is_empty()
            && !config.allowed_ports.contains(&port_state.port)
        {
            findings.push(SecurityFinding {
                finding_id: stable_id(
                    "finding",
                    &[
                        "ports",
                        "allowlist",
                        &port_state.protocol,
                        &port_state.port.to_string(),
                        &port_state.listen_address,
                    ],
                ),
                finding_fingerprint: Some(stable_id(
                    "fingerprint",
                    &[
                        "ports",
                        "allowlist",
                        &port_state.protocol,
                        &port_state.port.to_string(),
                        &port_state.listen_address,
                    ],
                )),
                category: "ports".to_string(),
                severity: SecuritySeverity::Medium,
                title: format!("unexpected listening port {}", port_state.port),
                detail: format!(
                    "{} is listening on {}:{} but is not present in security_scan.allowed_ports",
                    port_state.protocol, port_state.listen_address, port_state.port
                ),
                asset_type: "socket".to_string(),
                asset_name: format!("{}:{}", port_state.protocol, port_state.port),
                observed_value: Some(port_state.listen_address.clone()),
                expected_value: Some(format!("{:?}", config.allowed_ports)),
                check_id: format!("port-allowlist-{}", port_state.port),
                remediation: Some(
                    "add the port to security_scan.allowed_ports or stop the unexpected listener"
                        .to_string(),
                ),
                evidence: Some(port_evidence(port_state)),
            });
        }
    }

    (states.into_values().collect(), findings, runtime_issues)
}

#[derive(Debug, Clone)]
struct RawSocketEntry {
    protocol: String,
    listen_address: String,
    port: u16,
    inode: u64,
}

fn read_socket_table(
    path: &str,
    protocol: &str,
    tcp_only: bool,
) -> std::io::Result<Vec<RawSocketEntry>> {
    let raw = fs::read_to_string(path)?;
    let mut results = Vec::new();
    for line in raw.lines().skip(1) {
        let columns = line.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 10 {
            continue;
        }
        let local = columns[1];
        let state = columns[3];
        if tcp_only && state != "0A" {
            continue;
        }
        let Ok(inode) = columns[9].parse::<u64>() else {
            continue;
        };
        let Some((address_hex, port_hex)) = local.split_once(':') else {
            continue;
        };
        let Ok(port) = u16::from_str_radix(port_hex, 16) else {
            continue;
        };
        if port == 0 {
            continue;
        }
        results.push(RawSocketEntry {
            protocol: protocol.to_string(),
            listen_address: decode_socket_address(address_hex),
            port,
            inode,
        });
    }
    Ok(results)
}

fn decode_socket_address(address_hex: &str) -> String {
    match address_hex.len() {
        8 => decode_ipv4_address(address_hex).unwrap_or_else(|| address_hex.to_string()),
        32 => {
            if address_hex.chars().all(|value| value == '0') {
                "::".to_string()
            } else if address_hex == "00000000000000000000000000000001"
                || address_hex == "00000000000000000000000001000000"
            {
                "::1".to_string()
            } else {
                address_hex.to_string()
            }
        }
        _ => address_hex.to_string(),
    }
}

fn decode_ipv4_address(address_hex: &str) -> Option<String> {
    let bytes = (0..4)
        .map(|idx| u8::from_str_radix(&address_hex[idx * 2..idx * 2 + 2], 16).ok())
        .collect::<Option<Vec<_>>>()?;
    Some(format!(
        "{}.{}.{}.{}",
        bytes[3], bytes[2], bytes[1], bytes[0]
    ))
}

fn classify_exposure(address: &str) -> &'static str {
    match address {
        "0.0.0.0" | "::" => "all_interfaces",
        "127.0.0.1" | "::1" => "loopback",
        _ => "host_local",
    }
}

#[derive(Debug, Clone)]
struct SocketOwner {
    pid: u32,
    process_name: String,
    executable_path: Option<String>,
    service_unit: Option<String>,
    owner_uid: Option<u32>,
}

impl SocketOwner {
    fn detail(&self) -> String {
        let mut parts = vec![
            format!("pid={}", self.pid),
            format!("process={}", self.process_name),
        ];
        if let Some(path) = &self.executable_path {
            parts.push(format!("exe={path}"));
        }
        if let Some(unit) = &self.service_unit {
            parts.push(format!("unit={unit}"));
        }
        if let Some(uid) = self.owner_uid {
            parts.push(format!("uid={uid}"));
        }
        parts.join(", ")
    }
}

fn resolve_socket_owners(
    inodes: &[u64],
    runtime_issues: &mut Vec<SecurityRuntimeIssue>,
) -> BTreeMap<u64, SocketOwner> {
    let target_inodes = inodes.iter().copied().collect::<BTreeSet<_>>();
    if target_inodes.is_empty() {
        return BTreeMap::new();
    }

    let mut owners = BTreeMap::new();
    let proc_dir = Path::new("/proc");
    let Ok(entries) = fs::read_dir(proc_dir) else {
        runtime_issues.push(SecurityRuntimeIssue {
            issue_id: stable_id("issue", &["ports", "proc-unreadable"]),
            issue_kind: "proc_scan_unavailable".to_string(),
            severity: SecuritySeverity::Low,
            detail: "failed to enumerate /proc while resolving port owners".to_string(),
        });
        return owners;
    };

    let mut permission_denied_count = 0_u32;
    for entry in entries.flatten() {
        if owners.len() == target_inodes.len() {
            break;
        }
        let file_name = entry.file_name();
        let Some(pid_text) = file_name.to_str() else {
            continue;
        };
        let Ok(pid) = pid_text.parse::<u32>() else {
            continue;
        };

        let fd_dir = entry.path().join("fd");
        let fd_entries = match fs::read_dir(&fd_dir) {
            Ok(entries) => entries,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::PermissionDenied {
                    permission_denied_count = permission_denied_count.saturating_add(1);
                }
                continue;
            }
        };

        for fd_entry in fd_entries.flatten() {
            let Ok(target) = fs::read_link(fd_entry.path()) else {
                continue;
            };
            let target_text = target.to_string_lossy();
            let Some(inode) = parse_socket_inode(&target_text) else {
                continue;
            };
            if !target_inodes.contains(&inode) || owners.contains_key(&inode) {
                continue;
            }

            owners.insert(
                inode,
                SocketOwner {
                    pid,
                    process_name: fs::read_to_string(entry.path().join("comm"))
                        .ok()
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| "unknown".to_string()),
                    executable_path: fs::read_link(entry.path().join("exe"))
                        .ok()
                        .map(|path| path.to_string_lossy().into_owned()),
                    service_unit: read_service_unit(&entry.path().join("cgroup")),
                    owner_uid: read_process_uid(&entry.path().join("status")),
                },
            );
        }
    }

    if permission_denied_count > 0 {
        runtime_issues.push(SecurityRuntimeIssue {
            issue_id: stable_id("issue", &["ports", "proc-permissions"]),
            issue_kind: "port_owner_partial_visibility".to_string(),
            severity: SecuritySeverity::Info,
            detail: format!(
                "permission denied while reading {} process fd directories; port owner attribution may be partial",
                permission_denied_count
            ),
        });
    }

    owners
}

fn parse_socket_inode(target: &str) -> Option<u64> {
    target
        .strip_prefix("socket:[")
        .and_then(|value| value.strip_suffix(']'))
        .and_then(|value| value.parse::<u64>().ok())
}

fn read_process_uid(path: &Path) -> Option<u32> {
    let raw = fs::read_to_string(path).ok()?;
    raw.lines()
        .find(|line| line.starts_with("Uid:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u32>().ok())
}

fn read_service_unit(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    for line in raw.lines() {
        for segment in line.split('/') {
            if segment.ends_with(".service") || segment.ends_with(".scope") {
                return Some(segment.to_string());
            }
        }
    }
    None
}

fn stable_id(kind: &str, parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    for part in parts {
        digest.update([0_u8]);
        digest.update(part.as_bytes());
    }
    let hash = digest.finalize();
    format!("{kind}-{:x}", hash)[..(kind.len() + 1 + 16)].to_string()
}

fn port_evidence(port_state: &SecurityPortState) -> String {
    let mut evidence = vec![format!(
        "listener={}://{}:{}",
        port_state.protocol, port_state.listen_address, port_state.port
    )];
    if let Some(pid) = port_state.process_id {
        evidence.push(format!("pid={pid}"));
    }
    if let Some(name) = &port_state.process_name {
        evidence.push(format!("process={name}"));
    }
    if let Some(unit) = &port_state.service_unit {
        evidence.push(format!("unit={unit}"));
    }
    evidence.join(", ")
}

fn scan_packages(
    config: &SecurityScanConfig,
    rules: &RulesCatalog,
) -> (
    Vec<SecurityAssetVersion>,
    Vec<SecurityFinding>,
    Vec<SecurityRuntimeIssue>,
) {
    if config.package_watchlist.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    let queue = Arc::new(Mutex::new(VecDeque::from(config.package_watchlist.clone())));
    let results = Arc::new(Mutex::new(Vec::new()));
    let worker_count = config
        .max_parallel_checks
        .max(1)
        .min(config.package_watchlist.len().max(1));
    let rules = Arc::new(rules.clone());
    let timeout = Duration::from_secs(config.timeout_sec.min(30));

    thread::scope(|scope| {
        for _ in 0..worker_count {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);
            let rules = Arc::clone(&rules);
            scope.spawn(move || loop {
                let package_name = {
                    let mut queue = queue.lock().expect("security queue lock poisoned");
                    queue.pop_front()
                };
                let Some(package_name) = package_name else {
                    break;
                };
                let observation = inspect_package_asset(&package_name, &rules, timeout);
                let mut results = results.lock().expect("security result lock poisoned");
                results.push(observation);
            });
        }
    });

    let mut observations = results
        .lock()
        .expect("security result lock poisoned")
        .clone();
    observations.sort_by(|left, right| left.asset.requested_name.cmp(&right.asset.requested_name));

    let mut assets = Vec::with_capacity(observations.len());
    let mut findings = Vec::new();
    let mut runtime_issues = Vec::new();
    for observation in observations {
        assets.push(observation.asset);
        if let Some(finding) = observation.finding {
            findings.push(finding);
        }
        if let Some(issue) = observation.runtime_issue {
            runtime_issues.push(issue);
        }
    }
    (assets, findings, runtime_issues)
}

fn inspect_package_asset(
    requested_name: &str,
    rules: &RulesCatalog,
    timeout: Duration,
) -> PackageObservation {
    let requested_name = requested_name.trim().to_string();
    let rule = rules
        .rules_by_name
        .get(&requested_name.to_ascii_lowercase())
        .cloned();
    let candidate_names = package_candidates(&requested_name, rule.as_ref());
    let package_result = detect_package_version(&candidate_names, timeout);
    let binary_result = if package_result.is_none() {
        detect_binary_version(&requested_name, rule.as_ref(), timeout)
    } else {
        None
    };
    let result = package_result.or(binary_result);

    let (installed, resolved_name, source_kind, locator, version, detection_detail) = match result {
        Some(result) => (
            true,
            Some(result.resolved_name),
            result.source_kind,
            Some(result.locator),
            result.version,
            result.detail,
        ),
        None => (
            false,
            None,
            "unavailable".to_string(),
            None,
            None,
            "package or binary not detected on host".to_string(),
        ),
    };

    let (evaluation, finding, runtime_issue) = match (installed, version.clone(), rule.as_ref()) {
        (false, _, Some(rule)) => (
            "not_installed".to_string(),
            Some(SecurityFinding {
                finding_id: stable_id("finding", &["packages", "missing", &requested_name]),
                finding_fingerprint: Some(stable_id(
                    "fingerprint",
                    &["packages", "missing", &requested_name],
                )),
                category: "packages".to_string(),
                severity: rule.severity,
                title: format!("watched package `{requested_name}` is missing"),
                detail: format!(
                    "package or binary `{requested_name}` is absent while it is present in the watchlist"
                ),
                asset_type: "package".to_string(),
                asset_name: requested_name.clone(),
                observed_value: None,
                expected_value: Some(rule.min_secure_version.clone()),
                check_id: format!("package-present-{}", requested_name),
                remediation: Some("install the expected package or remove it from the watchlist if it is not required on this host".to_string()),
                evidence: Some(detection_detail.clone()),
            }),
            None,
        ),
        (true, Some(current), Some(rule)) => match compare_asset_versions(
            &current,
            &rule.min_secure_version,
            &source_kind,
            timeout,
        ) {
            Ordering::Less => (
                "outdated".to_string(),
                Some(SecurityFinding {
                    finding_id: stable_id(
                        "finding",
                        &["packages", "outdated", &requested_name, &current],
                    ),
                    finding_fingerprint: Some(stable_id(
                        "fingerprint",
                        &["packages", "outdated", &requested_name, &current],
                    )),
                    category: "packages".to_string(),
                    severity: rule.severity,
                    title: format!("package `{requested_name}` is below the secure version"),
                    detail: rule.summary.clone().unwrap_or_else(|| {
                        format!(
                            "detected version `{current}` is lower than `{}`",
                            rule.min_secure_version
                        )
                    }),
                    asset_type: "package".to_string(),
                    asset_name: requested_name.clone(),
                    observed_value: Some(current),
                    expected_value: Some(rule.min_secure_version.clone()),
                    check_id: format!("package-version-{}", requested_name),
                    remediation: Some("upgrade the installed package or binary to a version that satisfies the security rules".to_string()),
                    evidence: Some(detection_detail.clone()),
                }),
                None,
            ),
            _ => ("ok".to_string(), None, None),
        },
        (true, None, Some(rule)) => (
            "version_unknown".to_string(),
            None,
            Some(SecurityRuntimeIssue {
                issue_id: Uuid::new_v4().to_string(),
                issue_kind: "package_version_unparsed".to_string(),
                severity: rule.severity,
                detail: format!(
                    "package `{requested_name}` is installed but the version string could not be parsed"
                ),
            }),
        ),
        (true, _, None) => ("observed".to_string(), None, None),
        (false, _, None) => ("not_installed".to_string(), None, None),
    };

    PackageObservation {
        asset: SecurityAssetVersion {
            asset_id: stable_id("asset", &["package", &requested_name]),
            requested_name,
            resolved_name,
            source_kind,
            locator,
            installed,
            version,
            min_secure_version: rule.as_ref().map(|entry| entry.min_secure_version.clone()),
            evaluation,
            detail: Some(detection_detail),
        },
        finding,
        runtime_issue,
    }
}

#[derive(Debug, Clone)]
struct DetectionResult {
    resolved_name: String,
    source_kind: String,
    locator: String,
    version: Option<String>,
    detail: String,
}

fn package_candidates(requested_name: &str, rule: Option<&SecurityPackageRule>) -> Vec<String> {
    let mut names = BTreeSet::new();
    names.insert(requested_name.to_ascii_lowercase());
    for alias in built_in_package_aliases(requested_name) {
        names.insert(alias.to_string());
    }
    if let Some(rule) = rule {
        names.insert(rule.name.to_ascii_lowercase());
        for alias in &rule.aliases {
            names.insert(alias.to_ascii_lowercase());
        }
    }
    names.into_iter().collect()
}

fn built_in_package_aliases(requested_name: &str) -> &'static [&'static str] {
    match requested_name {
        "openssh" => &["openssh", "openssh-server", "openssh-clients"],
        "docker" => &["docker", "docker.io", "docker-ce"],
        "openssl" => &["openssl"],
        "nginx" => &["nginx"],
        _ => &[],
    }
}

fn detect_package_version(
    candidate_names: &[String],
    timeout: Duration,
) -> Option<DetectionResult> {
    for candidate in candidate_names {
        if let Some(version) = query_dpkg_version(candidate, timeout) {
            return Some(DetectionResult {
                resolved_name: candidate.clone(),
                source_kind: "dpkg".to_string(),
                locator: "dpkg-query".to_string(),
                version: Some(version),
                detail: format!("resolved from dpkg package `{candidate}`"),
            });
        }
        if let Some(version) = query_rpm_version(candidate, timeout) {
            return Some(DetectionResult {
                resolved_name: candidate.clone(),
                source_kind: "rpm".to_string(),
                locator: "rpm".to_string(),
                version: Some(version),
                detail: format!("resolved from rpm package `{candidate}`"),
            });
        }
        if let Some(version) = query_apk_version(candidate, timeout) {
            return Some(DetectionResult {
                resolved_name: candidate.clone(),
                source_kind: "apk".to_string(),
                locator: "apk".to_string(),
                version: Some(version),
                detail: format!("resolved from apk package `{candidate}`"),
            });
        }
    }
    None
}

fn detect_binary_version(
    requested_name: &str,
    rule: Option<&SecurityPackageRule>,
    timeout: Duration,
) -> Option<DetectionResult> {
    let mut candidates = BTreeSet::new();
    candidates.insert(requested_name.to_string());
    if let Some(rule) = rule {
        candidates.insert(rule.name.clone());
        for alias in &rule.aliases {
            candidates.insert(alias.clone());
        }
    }
    for probe in built_in_binary_probes(requested_name, &candidates) {
        let Ok(output) = run_command(&probe.program, &probe.args, timeout) else {
            continue;
        };
        if !output.success {
            continue;
        }
        let version = extract_version_token(&format!("{} {}", output.stdout, output.stderr));
        return Some(DetectionResult {
            resolved_name: probe.name,
            source_kind: "binary".to_string(),
            locator: probe.program,
            version,
            detail: "resolved from local binary version output".to_string(),
        });
    }
    None
}

#[derive(Debug, Clone)]
struct BinaryProbe {
    name: String,
    program: String,
    args: Vec<String>,
}

fn built_in_binary_probes(requested_name: &str, candidates: &BTreeSet<String>) -> Vec<BinaryProbe> {
    let mut probes = Vec::new();
    match requested_name {
        "openssl" => probes.push(BinaryProbe {
            name: "openssl".to_string(),
            program: "openssl".to_string(),
            args: vec!["version".to_string()],
        }),
        "openssh" => {
            probes.push(BinaryProbe {
                name: "ssh".to_string(),
                program: "ssh".to_string(),
                args: vec!["-V".to_string()],
            });
            probes.push(BinaryProbe {
                name: "sshd".to_string(),
                program: "sshd".to_string(),
                args: vec!["-V".to_string()],
            });
        }
        "nginx" => probes.push(BinaryProbe {
            name: "nginx".to_string(),
            program: "nginx".to_string(),
            args: vec!["-v".to_string()],
        }),
        "docker" => probes.push(BinaryProbe {
            name: "docker".to_string(),
            program: "docker".to_string(),
            args: vec!["--version".to_string()],
        }),
        _ => {}
    }

    for candidate in candidates {
        probes.push(BinaryProbe {
            name: candidate.clone(),
            program: candidate.clone(),
            args: vec!["--version".to_string()],
        });
        probes.push(BinaryProbe {
            name: candidate.clone(),
            program: candidate.clone(),
            args: vec!["version".to_string()],
        });
    }

    let mut seen = BTreeSet::new();
    probes
        .into_iter()
        .filter(|probe| seen.insert(format!("{} {:?}", probe.program, probe.args)))
        .collect()
}

fn query_dpkg_version(candidate: &str, timeout: Duration) -> Option<String> {
    let output = run_command(
        "dpkg-query",
        &[
            "-W".to_string(),
            "-f=${Version}".to_string(),
            candidate.to_string(),
        ],
        timeout,
    )
    .ok()?;
    if !output.success {
        return None;
    }
    let version = output.stdout.trim();
    (!version.is_empty()).then(|| version.to_string())
}

fn query_rpm_version(candidate: &str, timeout: Duration) -> Option<String> {
    let output = run_command(
        "rpm",
        &[
            "-q".to_string(),
            "--qf".to_string(),
            "%{VERSION}-%{RELEASE}".to_string(),
            candidate.to_string(),
        ],
        timeout,
    )
    .ok()?;
    if !output.success {
        return None;
    }
    let version = output.stdout.trim();
    (!version.is_empty()).then(|| version.to_string())
}

fn query_apk_version(candidate: &str, timeout: Duration) -> Option<String> {
    let output = run_command(
        "apk",
        &[
            String::from("info"),
            String::from("-v"),
            candidate.to_string(),
        ],
        timeout,
    )
    .ok()?;
    if !output.success {
        return None;
    }
    output.stdout.lines().find_map(|line| {
        line.trim()
            .strip_prefix(&format!("{candidate}-"))
            .map(ToOwned::to_owned)
    })
}

fn run_command(
    program: &str,
    args: &[String],
    timeout: Duration,
) -> Result<CommandCapture, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| error.to_string())?;
                return Ok(CommandCapture {
                    success: output.status.success(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "command `{program}` timed out after {} seconds",
                        timeout.as_secs()
                    ));
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn extract_version_token(raw: &str) -> Option<String> {
    let mut token = String::new();
    for chunk in raw.split_whitespace() {
        if !chunk.chars().any(|value| value.is_ascii_digit()) {
            continue;
        }
        let cleaned = chunk
            .trim_matches(|value: char| {
                !value.is_ascii_alphanumeric()
                    && value != '.'
                    && value != '_'
                    && value != '-'
                    && value != '/'
            })
            .rsplit('/')
            .next()
            .unwrap_or(chunk);
        let trimmed = cleaned
            .trim_start_matches(|value: char| !value.is_ascii_digit())
            .trim_end_matches(|value: char| {
                !value.is_ascii_alphanumeric() && value != '.' && value != '_' && value != '-'
            });
        if trimmed.chars().any(|value| value.is_ascii_digit()) {
            token = trimmed.to_string();
            break;
        }
    }
    (!token.is_empty()).then_some(token)
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left = version_tokens(left);
    let right = version_tokens(right);
    let max_len = left.len().max(right.len());
    for idx in 0..max_len {
        let ordering = match (left.get(idx), right.get(idx)) {
            (Some(VersionToken::Number(left)), Some(VersionToken::Number(right))) => {
                left.cmp(right)
            }
            (Some(VersionToken::Text(left)), Some(VersionToken::Text(right))) => left.cmp(right),
            (Some(VersionToken::Number(_)), Some(VersionToken::Text(_))) => Ordering::Greater,
            (Some(VersionToken::Text(_)), Some(VersionToken::Number(_))) => Ordering::Less,
            (Some(VersionToken::Number(left)), None) => left.cmp(&0),
            (Some(VersionToken::Text(left)), None) => {
                if left.is_empty() {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            }
            (None, Some(VersionToken::Number(right))) => 0.cmp(right),
            (None, Some(VersionToken::Text(right))) => {
                if right.is_empty() {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            }
            (None, None) => Ordering::Equal,
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

fn compare_asset_versions(
    current: &str,
    minimum: &str,
    source_kind: &str,
    timeout: Duration,
) -> Ordering {
    match source_kind {
        "dpkg" => compare_dpkg_versions(current, minimum, timeout)
            .unwrap_or_else(|| compare_versions(current, minimum)),
        "rpm" => compare_rpm_versions(current, minimum, timeout)
            .unwrap_or_else(|| compare_versions(current, minimum)),
        "apk" => compare_apk_versions(current, minimum, timeout)
            .unwrap_or_else(|| compare_versions(current, minimum)),
        _ => compare_versions(current, minimum),
    }
}

fn compare_dpkg_versions(left: &str, right: &str, timeout: Duration) -> Option<Ordering> {
    if run_command(
        "dpkg",
        &[
            "--compare-versions".to_string(),
            left.to_string(),
            "lt".to_string(),
            right.to_string(),
        ],
        timeout,
    )
    .ok()?
    .success
    {
        return Some(Ordering::Less);
    }
    if run_command(
        "dpkg",
        &[
            "--compare-versions".to_string(),
            left.to_string(),
            "gt".to_string(),
            right.to_string(),
        ],
        timeout,
    )
    .ok()?
    .success
    {
        return Some(Ordering::Greater);
    }
    Some(Ordering::Equal)
}

fn compare_rpm_versions(left: &str, right: &str, timeout: Duration) -> Option<Ordering> {
    let expression = format!(
        "%{{lua:print(rpm.vercmp('{}','{}'))}}",
        escape_rpm_lua(left),
        escape_rpm_lua(right)
    );
    let output = run_command("rpm", &[String::from("--eval"), expression], timeout).ok()?;
    if !output.success {
        return None;
    }
    match output.stdout.trim() {
        "-1" => Some(Ordering::Less),
        "0" => Some(Ordering::Equal),
        "1" => Some(Ordering::Greater),
        _ => None,
    }
}

fn compare_apk_versions(left: &str, right: &str, timeout: Duration) -> Option<Ordering> {
    let output = run_command(
        "apk",
        &[
            String::from("version"),
            String::from("-t"),
            left.to_string(),
            right.to_string(),
        ],
        timeout,
    )
    .ok()?;
    if !output.success {
        return None;
    }
    match output.stdout.trim() {
        "<" => Some(Ordering::Less),
        "=" => Some(Ordering::Equal),
        ">" => Some(Ordering::Greater),
        _ => None,
    }
}

fn escape_rpm_lua(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionToken {
    Number(u64),
    Text(String),
}

fn version_tokens(raw: &str) -> Vec<VersionToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut numeric = None;

    for value in raw.chars() {
        if value.is_ascii_alphanumeric() {
            let is_digit = value.is_ascii_digit();
            match numeric {
                Some(current_numeric) if current_numeric != is_digit => {
                    push_version_token(&mut tokens, &mut current, current_numeric);
                    current.push(value.to_ascii_lowercase());
                    numeric = Some(is_digit);
                }
                _ => {
                    current.push(value.to_ascii_lowercase());
                    numeric = Some(is_digit);
                }
            }
        } else if !current.is_empty() {
            push_version_token(&mut tokens, &mut current, numeric.unwrap_or(false));
            numeric = None;
        }
    }

    if !current.is_empty() {
        push_version_token(&mut tokens, &mut current, numeric.unwrap_or(false));
    }

    tokens
}

fn push_version_token(tokens: &mut Vec<VersionToken>, current: &mut String, numeric: bool) {
    if numeric {
        tokens.push(VersionToken::Number(
            current.parse::<u64>().unwrap_or_default(),
        ));
        current.clear();
    } else {
        tokens.push(VersionToken::Text(std::mem::take(current)));
    }
}

fn scan_misconfigurations(
    profile: SecurityScanProfile,
) -> (
    Vec<SecurityMisconfigurationCheck>,
    Vec<SecurityFinding>,
    Vec<SecurityRuntimeIssue>,
) {
    let mut checks = Vec::new();
    let mut findings = Vec::new();
    let mut runtime_issues = Vec::new();

    for path in critical_files(profile) {
        if !path.exists() {
            checks.push(SecurityMisconfigurationCheck {
                check_id: format!("world-writable-{}", path.display()),
                check_name: "world_writable_critical_file".to_string(),
                target: path.display().to_string(),
                status: "not_found".to_string(),
                severity: SecuritySeverity::Info,
                detail: format!("critical file `{}` is not present", path.display()),
                observed_value: None,
                expected_value: Some("strict file permissions".to_string()),
            });
            continue;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            match fs::metadata(path) {
                Ok(metadata) => {
                    let world_writable = metadata.permissions().mode() & 0o002 != 0;
                    checks.push(SecurityMisconfigurationCheck {
                        check_id: format!("world-writable-{}", path.display()),
                        check_name: "world_writable_critical_file".to_string(),
                        target: path.display().to_string(),
                        status: if world_writable { "failed" } else { "passed" }.to_string(),
                        severity: if world_writable {
                            SecuritySeverity::High
                        } else {
                            SecuritySeverity::Info
                        },
                        detail: if world_writable {
                            format!("critical file `{}` is world-writable", path.display())
                        } else {
                            format!("critical file `{}` is not world-writable", path.display())
                        },
                        observed_value: Some(format!(
                            "{:o}",
                            metadata.permissions().mode() & 0o7777
                        )),
                        expected_value: Some("others must not have write permissions".to_string()),
                    });
                    if world_writable {
                        findings.push(SecurityFinding {
                            finding_id: stable_id(
                                "finding",
                                &["misconfiguration", "world-writable", &path.display().to_string()],
                            ),
                            finding_fingerprint: Some(stable_id(
                                "fingerprint",
                                &["misconfiguration", "world-writable", &path.display().to_string()],
                            )),
                            category: "misconfiguration".to_string(),
                            severity: SecuritySeverity::High,
                            title: format!("critical file `{}` is world-writable", path.display()),
                            detail: format!(
                                "permission bits on `{}` allow write access for others",
                                path.display()
                            ),
                            asset_type: "file".to_string(),
                            asset_name: path.display().to_string(),
                            observed_value: Some(format!(
                                "{:o}",
                                metadata.permissions().mode() & 0o7777
                            )),
                            expected_value: Some(
                                "others must not have write permissions".to_string(),
                            ),
                            check_id: format!("world-writable-{}", path.display()),
                            remediation: Some("remove write permission for others and restore the expected owner/group on this file".to_string()),
                            evidence: Some(format!(
                                "mode={:o}",
                                metadata.permissions().mode() & 0o7777
                            )),
                        });
                    }
                }
                Err(error) => runtime_issues.push(SecurityRuntimeIssue {
                    issue_id: Uuid::new_v4().to_string(),
                    issue_kind: "critical_file_metadata_failed".to_string(),
                    severity: SecuritySeverity::Low,
                    detail: format!("failed to inspect `{}`: {error}", path.display()),
                }),
            }
        }
    }

    let firewall = firewall_status();
    checks.push(SecurityMisconfigurationCheck {
        check_id: "firewall-enabled".to_string(),
        check_name: "firewall_enabled".to_string(),
        target: "host_firewall".to_string(),
        status: firewall.status.clone(),
        severity: firewall.severity,
        detail: firewall.detail.clone(),
        observed_value: firewall.observed_value.clone(),
        expected_value: Some("input firewall should be active with restrictive rules".to_string()),
    });
    if let Some(finding) = firewall.finding {
        findings.push(finding);
    }
    if let Some(issue) = firewall.runtime_issue {
        runtime_issues.push(issue);
    }

    let root_login = root_ssh_login_policy();
    checks.push(SecurityMisconfigurationCheck {
        check_id: "ssh-root-login".to_string(),
        check_name: "ssh_root_login_policy".to_string(),
        target: "/etc/ssh/sshd_config".to_string(),
        status: root_login.status.clone(),
        severity: root_login.severity,
        detail: root_login.detail.clone(),
        observed_value: root_login.observed_value.clone(),
        expected_value: Some("PermitRootLogin no".to_string()),
    });
    if let Some(finding) = root_login.finding {
        findings.push(finding);
    }
    if let Some(issue) = root_login.runtime_issue {
        runtime_issues.push(issue);
    }

    (checks, findings, runtime_issues)
}

fn critical_files(profile: SecurityScanProfile) -> Vec<&'static Path> {
    match profile {
        SecurityScanProfile::Light => vec![
            Path::new("/etc/passwd"),
            Path::new("/etc/shadow"),
            Path::new("/etc/ssh/sshd_config"),
        ],
        SecurityScanProfile::Balanced => vec![
            Path::new("/etc/passwd"),
            Path::new("/etc/shadow"),
            Path::new("/etc/sudoers"),
            Path::new("/etc/ssh/sshd_config"),
        ],
        SecurityScanProfile::Deep => vec![
            Path::new("/etc/passwd"),
            Path::new("/etc/shadow"),
            Path::new("/etc/group"),
            Path::new("/etc/hosts"),
            Path::new("/etc/sudoers"),
            Path::new("/etc/ssh/sshd_config"),
        ],
    }
}

#[derive(Debug)]
struct HardeningResult {
    status: String,
    severity: SecuritySeverity,
    detail: String,
    observed_value: Option<String>,
    finding: Option<SecurityFinding>,
    runtime_issue: Option<SecurityRuntimeIssue>,
}

fn firewall_status() -> HardeningResult {
    let timeout = Duration::from_secs(5);
    if let Some(inspection) = inspect_nftables(timeout) {
        return inspection;
    }
    if let Some(inspection) = inspect_iptables(timeout) {
        return inspection;
    }

    let firewalld = run_command("firewall-cmd", &[String::from("--state")], timeout).ok();
    if let Some(result) = firewalld {
        let normalized = format!("{} {}", result.stdout, result.stderr).to_ascii_lowercase();
        if result.success && normalized.contains("running") {
            return HardeningResult {
                status: "passed".to_string(),
                severity: SecuritySeverity::Info,
                detail: "firewalld is active".to_string(),
                observed_value: Some("firewalld=running".to_string()),
                finding: None,
                runtime_issue: None,
            };
        }
    }

    let ufw = run_command("ufw", &[String::from("status")], timeout).ok();
    if let Some(result) = ufw {
        let normalized = format!("{} {}", result.stdout, result.stderr).to_ascii_lowercase();
        if normalized.contains("status: active") {
            return HardeningResult {
                status: "passed".to_string(),
                severity: SecuritySeverity::Info,
                detail: "ufw is active".to_string(),
                observed_value: Some("ufw=active".to_string()),
                finding: None,
                runtime_issue: None,
            };
        }
        if normalized.contains("status: inactive") {
            return firewall_disabled_result(
                "ufw is installed but inactive",
                Some("ufw=inactive".to_string()),
            );
        }
    }

    for service in ["firewalld", "ufw"] {
        let result = run_command(
            "systemctl",
            &[String::from("is-active"), service.to_string()],
            timeout,
        )
        .ok();
        if let Some(result) = result {
            if result.success && result.stdout.trim() == "active" {
                return HardeningResult {
                    status: "passed".to_string(),
                    severity: SecuritySeverity::Info,
                    detail: format!("{service} service is active"),
                    observed_value: Some(format!("{service}=active")),
                    finding: None,
                    runtime_issue: None,
                };
            }
            if result.stdout.trim() == "inactive" {
                return firewall_disabled_result(
                    &format!("{service} service is inactive"),
                    Some(format!("{service}=inactive")),
                );
            }
        }
    }

    HardeningResult {
        status: "unknown".to_string(),
        severity: SecuritySeverity::Low,
        detail: "unable to verify firewall state from nftables, iptables, firewalld, ufw, or systemctl".to_string(),
        observed_value: None,
        finding: None,
        runtime_issue: Some(SecurityRuntimeIssue {
            issue_id: stable_id("issue", &["firewall", "state-unknown"]),
            issue_kind: "firewall_state_unknown".to_string(),
            severity: SecuritySeverity::Low,
            detail: "unable to verify firewall state from nftables, iptables, firewalld, ufw, or systemctl"
                .to_string(),
        }),
    }
}

fn inspect_nftables(timeout: Duration) -> Option<HardeningResult> {
    let output = run_command(
        "nft",
        &[String::from("list"), String::from("ruleset")],
        timeout,
    )
    .ok()?;
    if !output.success {
        return None;
    }
    let ruleset = output.stdout.trim();
    if ruleset.is_empty() {
        return Some(firewall_disabled_result(
            "nftables ruleset is empty",
            Some("nftables=empty".to_string()),
        ));
    }

    let normalized = ruleset.to_ascii_lowercase();
    if normalized.contains("hook input")
        && (normalized.contains("policy drop") || normalized.contains("policy reject"))
    {
        return Some(HardeningResult {
            status: "passed".to_string(),
            severity: SecuritySeverity::Info,
            detail: "nftables input hook has a restrictive default policy".to_string(),
            observed_value: Some("nftables=restrictive".to_string()),
            finding: None,
            runtime_issue: None,
        });
    }
    if normalized.contains("hook input")
        && (normalized.contains(" drop") || normalized.contains(" reject"))
    {
        return Some(HardeningResult {
            status: "passed".to_string(),
            severity: SecuritySeverity::Info,
            detail: "nftables input hook contains explicit drop or reject rules".to_string(),
            observed_value: Some("nftables=explicit-drop".to_string()),
            finding: None,
            runtime_issue: None,
        });
    }
    if normalized.contains("hook input") {
        return Some(HardeningResult {
            status: "warning".to_string(),
            severity: SecuritySeverity::Medium,
            detail: "nftables input hook exists but does not expose a clear restrictive policy".to_string(),
            observed_value: Some("nftables=permissive".to_string()),
            finding: Some(SecurityFinding {
                finding_id: stable_id("finding", &["firewall", "nftables", "permissive"]),
                finding_fingerprint: Some(stable_id("fingerprint", &["firewall", "nftables", "permissive"])),
                category: "misconfiguration".to_string(),
                severity: SecuritySeverity::Medium,
                title: "nftables input policy appears permissive".to_string(),
                detail: "nftables input hook exists but no clear default drop/reject policy was detected".to_string(),
                asset_type: "firewall".to_string(),
                asset_name: "nftables".to_string(),
                observed_value: Some("permissive".to_string()),
                expected_value: Some("restrictive".to_string()),
                check_id: "firewall-enabled".to_string(),
                remediation: Some("review nftables input chain and set a restrictive default or explicit drop/reject rules".to_string()),
                evidence: Some("derived from `nft list ruleset`".to_string()),
            }),
            runtime_issue: None,
        });
    }
    None
}

fn inspect_iptables(timeout: Duration) -> Option<HardeningResult> {
    let output = run_command("iptables-save", &[], timeout).ok()?;
    if !output.success {
        return None;
    }
    let rules = output.stdout;
    if rules.trim().is_empty() {
        return Some(firewall_disabled_result(
            "iptables-save returned an empty ruleset",
            Some("iptables=empty".to_string()),
        ));
    }

    let normalized = rules.to_ascii_lowercase();
    if normalized.contains(":input drop")
        || normalized.contains(":input reject")
        || normalized.contains("-a input -j drop")
        || normalized.contains("-a input -j reject")
    {
        return Some(HardeningResult {
            status: "passed".to_string(),
            severity: SecuritySeverity::Info,
            detail: "iptables input chain contains a restrictive policy".to_string(),
            observed_value: Some("iptables=restrictive".to_string()),
            finding: None,
            runtime_issue: None,
        });
    }

    if normalized.contains(":input accept") {
        return Some(firewall_disabled_result(
            "iptables INPUT chain default policy is ACCEPT without clear restrictive rules",
            Some("iptables=accept".to_string()),
        ));
    }

    None
}

fn firewall_disabled_result(detail: &str, observed_value: Option<String>) -> HardeningResult {
    HardeningResult {
        status: "failed".to_string(),
        severity: SecuritySeverity::High,
        detail: detail.to_string(),
        observed_value: observed_value.clone(),
        finding: Some(SecurityFinding {
            finding_id: stable_id("finding", &["firewall", detail]),
            finding_fingerprint: Some(stable_id("fingerprint", &["firewall", detail])),
            category: "misconfiguration".to_string(),
            severity: SecuritySeverity::High,
            title: "host firewall is disabled or permissive".to_string(),
            detail: detail.to_string(),
            asset_type: "firewall".to_string(),
            asset_name: "host_firewall".to_string(),
            observed_value,
            expected_value: Some("restrictive input firewall".to_string()),
            check_id: "firewall-enabled".to_string(),
            remediation: Some(
                "enable a host firewall and enforce a restrictive INPUT policy".to_string(),
            ),
            evidence: Some("derived from firewall inspection commands".to_string()),
        }),
        runtime_issue: None,
    }
}

fn read_effective_sshd_option(option: &str, timeout: Duration) -> Option<String> {
    let output = run_command(
        "sshd",
        &[
            String::from("-T"),
            String::from("-f"),
            String::from("/etc/ssh/sshd_config"),
        ],
        timeout,
    )
    .ok()?;
    if !output.success {
        return None;
    }
    output.stdout.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let key = parts.next()?;
        if key.eq_ignore_ascii_case(option) {
            parts.next().map(ToOwned::to_owned)
        } else {
            None
        }
    })
}

fn root_ssh_login_policy() -> HardeningResult {
    let path = Path::new("/etc/ssh/sshd_config");
    let timeout = Duration::from_secs(5);
    if !path.exists() {
        return HardeningResult {
            status: "not_found".to_string(),
            severity: SecuritySeverity::Info,
            detail: "sshd_config is not present".to_string(),
            observed_value: None,
            finding: None,
            runtime_issue: None,
        };
    }

    if let Some(result) = read_effective_sshd_option("permitrootlogin", timeout) {
        return match result.as_str() {
            "no" => HardeningResult {
                status: "passed".to_string(),
                severity: SecuritySeverity::Info,
                detail: "PermitRootLogin is disabled in effective sshd configuration".to_string(),
                observed_value: Some(result),
                finding: None,
                runtime_issue: None,
            },
            "prohibit-password" | "without-password" | "forced-commands-only" => HardeningResult {
                status: "warning".to_string(),
                severity: SecuritySeverity::Medium,
                detail: format!(
                    "effective sshd PermitRootLogin is `{}`; root key-based access remains possible",
                    result
                ),
                observed_value: Some(result.clone()),
                finding: Some(SecurityFinding {
                    finding_id: stable_id("finding", &["ssh", "root-login", &result]),
                    finding_fingerprint: Some(stable_id("fingerprint", &["ssh", "root-login", &result])),
                    category: "misconfiguration".to_string(),
                    severity: SecuritySeverity::Medium,
                    title: "root SSH login is partially allowed".to_string(),
                    detail: format!("effective PermitRootLogin is `{result}` instead of `no`"),
                    asset_type: "config".to_string(),
                    asset_name: path.display().to_string(),
                    observed_value: Some(result),
                    expected_value: Some("no".to_string()),
                    check_id: "ssh-root-login".to_string(),
                    remediation: Some("set `PermitRootLogin no` and reload sshd".to_string()),
                    evidence: Some("derived from `sshd -T`".to_string()),
                }),
                runtime_issue: None,
            },
            "yes" => HardeningResult {
                status: "failed".to_string(),
                severity: SecuritySeverity::High,
                detail: "effective sshd configuration allows direct root login".to_string(),
                observed_value: Some(result.clone()),
                finding: Some(SecurityFinding {
                    finding_id: stable_id("finding", &["ssh", "root-login", "yes"]),
                    finding_fingerprint: Some(stable_id("fingerprint", &["ssh", "root-login", "yes"])),
                    category: "misconfiguration".to_string(),
                    severity: SecuritySeverity::High,
                    title: "root SSH login is enabled".to_string(),
                    detail: "effective PermitRootLogin is `yes`".to_string(),
                    asset_type: "config".to_string(),
                    asset_name: path.display().to_string(),
                    observed_value: Some("yes".to_string()),
                    expected_value: Some("no".to_string()),
                    check_id: "ssh-root-login".to_string(),
                    remediation: Some("set `PermitRootLogin no` and reload sshd".to_string()),
                    evidence: Some("derived from `sshd -T`".to_string()),
                }),
                runtime_issue: None,
            },
            other => HardeningResult {
                status: "warning".to_string(),
                severity: SecuritySeverity::Medium,
                detail: format!("effective sshd PermitRootLogin has unrecognized value `{other}`"),
                observed_value: Some(other.to_string()),
                finding: None,
                runtime_issue: Some(SecurityRuntimeIssue {
                    issue_id: stable_id("issue", &["ssh", "root-login", "unknown"]),
                    issue_kind: "sshd_root_login_unknown".to_string(),
                    severity: SecuritySeverity::Medium,
                    detail: format!("effective sshd PermitRootLogin has unrecognized value `{other}`"),
                }),
            },
        };
    }

    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            return HardeningResult {
                status: "unknown".to_string(),
                severity: SecuritySeverity::Low,
                detail: format!("failed to read `{}`: {error}", path.display()),
                observed_value: None,
                finding: None,
                runtime_issue: Some(SecurityRuntimeIssue {
                    issue_id: stable_id("issue", &["ssh", "read-failed"]),
                    issue_kind: "sshd_config_read_failed".to_string(),
                    severity: SecuritySeverity::Low,
                    detail: format!("failed to read `{}`: {error}", path.display()),
                }),
            }
        }
    };

    let mut permit_root_login = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.to_ascii_lowercase().starts_with("match ") {
            break;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        if key.eq_ignore_ascii_case("PermitRootLogin") {
            permit_root_login = parts.next().map(|value| value.to_ascii_lowercase());
        }
    }

    match permit_root_login.as_deref() {
        Some("no") => HardeningResult {
            status: "passed".to_string(),
            severity: SecuritySeverity::Info,
            detail: "PermitRootLogin is disabled".to_string(),
            observed_value: Some("no".to_string()),
            finding: None,
            runtime_issue: None,
        },
        Some("prohibit-password") | Some("without-password") | Some("forced-commands-only") => {
            HardeningResult {
                status: "warning".to_string(),
                severity: SecuritySeverity::Medium,
                detail: format!(
                    "PermitRootLogin is `{}`; root key-based access remains possible",
                    permit_root_login.clone().unwrap_or_default()
                ),
                observed_value: permit_root_login.clone(),
                finding: Some(SecurityFinding {
                    finding_id: stable_id(
                        "finding",
                        &[
                            "ssh",
                            "root-login",
                            permit_root_login.as_deref().unwrap_or_default(),
                        ],
                    ),
                    finding_fingerprint: Some(stable_id(
                        "fingerprint",
                        &[
                            "ssh",
                            "root-login",
                            permit_root_login.as_deref().unwrap_or_default(),
                        ],
                    )),
                    category: "misconfiguration".to_string(),
                    severity: SecuritySeverity::Medium,
                    title: "root SSH login is partially allowed".to_string(),
                    detail: format!(
                        "PermitRootLogin is `{}` instead of `no`",
                        permit_root_login.clone().unwrap_or_default()
                    ),
                    asset_type: "config".to_string(),
                    asset_name: path.display().to_string(),
                    observed_value: permit_root_login.clone(),
                    expected_value: Some("no".to_string()),
                    check_id: "ssh-root-login".to_string(),
                    remediation: Some("set `PermitRootLogin no` and reload sshd".to_string()),
                    evidence: Some("derived from /etc/ssh/sshd_config".to_string()),
                }),
                runtime_issue: None,
            }
        }
        Some("yes") => HardeningResult {
            status: "failed".to_string(),
            severity: SecuritySeverity::High,
            detail: "PermitRootLogin allows direct root SSH login".to_string(),
            observed_value: Some("yes".to_string()),
            finding: Some(SecurityFinding {
                finding_id: stable_id("finding", &["ssh", "root-login", "yes"]),
                finding_fingerprint: Some(stable_id("fingerprint", &["ssh", "root-login", "yes"])),
                category: "misconfiguration".to_string(),
                severity: SecuritySeverity::High,
                title: "root SSH login is enabled".to_string(),
                detail: "PermitRootLogin is set to `yes`".to_string(),
                asset_type: "config".to_string(),
                asset_name: path.display().to_string(),
                observed_value: Some("yes".to_string()),
                expected_value: Some("no".to_string()),
                check_id: "ssh-root-login".to_string(),
                remediation: Some("set `PermitRootLogin no` and reload sshd".to_string()),
                evidence: Some("derived from /etc/ssh/sshd_config".to_string()),
            }),
            runtime_issue: None,
        },
        Some(other) => HardeningResult {
            status: "warning".to_string(),
            severity: SecuritySeverity::Medium,
            detail: format!("unrecognized PermitRootLogin value `{other}`"),
            observed_value: Some(other.to_string()),
            finding: None,
            runtime_issue: Some(SecurityRuntimeIssue {
                issue_id: stable_id("issue", &["ssh", "root-login", "unknown"]),
                issue_kind: "sshd_root_login_unknown".to_string(),
                severity: SecuritySeverity::Medium,
                detail: format!("unrecognized PermitRootLogin value `{other}`"),
            }),
        },
        None => HardeningResult {
            status: "unknown".to_string(),
            severity: SecuritySeverity::Low,
            detail: "PermitRootLogin is not explicitly configured".to_string(),
            observed_value: None,
            finding: Some(SecurityFinding {
                finding_id: stable_id("finding", &["ssh", "root-login", "implicit"]),
                finding_fingerprint: Some(stable_id(
                    "fingerprint",
                    &["ssh", "root-login", "implicit"],
                )),
                category: "misconfiguration".to_string(),
                severity: SecuritySeverity::Low,
                title: "root SSH login policy is implicit".to_string(),
                detail: "PermitRootLogin is not explicitly configured in sshd_config".to_string(),
                asset_type: "config".to_string(),
                asset_name: path.display().to_string(),
                observed_value: None,
                expected_value: Some("no".to_string()),
                check_id: "ssh-root-login".to_string(),
                remediation: Some(
                    "set `PermitRootLogin no` explicitly and reload sshd".to_string(),
                ),
                evidence: Some("derived from /etc/ssh/sshd_config".to_string()),
            }),
            runtime_issue: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use tempfile::TempDir;

    use super::{
        compare_versions, extract_version_token, persist_report, summarize_findings, version_tokens,
    };
    use crate::security::{SecurityFinding, SecuritySeverity};

    #[test]
    fn compares_versions_numerically() {
        assert_eq!(compare_versions("1.2.10", "1.2.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.2.0", "1.2"), Ordering::Equal);
        assert_eq!(compare_versions("8.9p1", "9.0p1"), Ordering::Less);
    }

    #[test]
    fn extracts_version_from_mixed_banner() {
        assert_eq!(
            extract_version_token("OpenSSL 3.0.2 15 Mar 2022").as_deref(),
            Some("3.0.2")
        );
        assert_eq!(
            extract_version_token("OpenSSH_8.9p1 Ubuntu-3ubuntu0.7").as_deref(),
            Some("8.9p1")
        );
        assert_eq!(
            extract_version_token("nginx version: nginx/1.24.0").as_deref(),
            Some("1.24.0")
        );
    }

    #[test]
    fn persists_report_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("security").join("last-report.json");
        persist_report(&path, "{\"ok\":true}").unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "{\"ok\":true}");
    }

    #[test]
    fn summarizes_findings_by_severity() {
        let summary = summarize_findings(&[
            SecurityFinding {
                finding_id: "1".to_string(),
                finding_fingerprint: None,
                category: "ports".to_string(),
                severity: SecuritySeverity::Critical,
                title: "a".to_string(),
                detail: "a".to_string(),
                asset_type: "socket".to_string(),
                asset_name: "a".to_string(),
                observed_value: None,
                expected_value: None,
                check_id: "a".to_string(),
                remediation: None,
                evidence: None,
            },
            SecurityFinding {
                finding_id: "2".to_string(),
                finding_fingerprint: None,
                category: "ports".to_string(),
                severity: SecuritySeverity::Low,
                title: "b".to_string(),
                detail: "b".to_string(),
                asset_type: "socket".to_string(),
                asset_name: "b".to_string(),
                observed_value: None,
                expected_value: None,
                check_id: "b".to_string(),
                remediation: None,
                evidence: None,
            },
        ]);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.critical, 1);
        assert_eq!(summary.low, 1);
    }

    #[test]
    fn tokenizes_mixed_versions() {
        let tokens = version_tokens("1.2.3-ubuntu4");
        assert!(!tokens.is_empty());
    }
}
