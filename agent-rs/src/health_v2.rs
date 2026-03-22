use std::{fs, path::Path};

use chrono::{SecondsFormat, Utc};

use crate::{
    config::AgentConfig,
    ops::{CheckStatus, OperationalReport, OverallStatus, ReportCheck, ReportSummary},
    runtime::{diagnostics::local_diagnostics_snapshot_path, DiagnosticsSnapshot, RuntimePhase},
};

const DEFAULT_HEALTH_FRESHNESS_SEC: u64 = 90;

pub fn run(config_path: &Path) -> OperationalReport {
    let config_path_text = config_path.display().to_string();
    let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    let config = match AgentConfig::load(config_path) {
        Ok(config) => config,
        Err(error) => {
            return build_report(
                "health",
                config_path_text,
                generated_at,
                vec![ReportCheck::new(
                    "config",
                    CheckStatus::Fail,
                    "config",
                    format!("failed to load configuration: {error}"),
                    Some(
                        "fix the config file path and rerun `doro-agent preflight` before health"
                            .to_string(),
                    ),
                )],
            );
        }
    };

    let snapshot_path = local_diagnostics_snapshot_path(&config.state_dir);
    let mut checks = Vec::new();
    checks.push(pass_check(
        "config",
        "config",
        "configuration parsed successfully",
        None,
    ));
    checks.push(if config.state_dir.exists() {
        pass_check(
            "state-dir",
            "state",
            format!("state dir `{}` exists", config.state_dir.display()),
            None,
        )
    } else {
        fail_check(
            "state-dir",
            "state",
            format!(
                "state dir `{}` does not exist; the runtime has not initialized local state yet",
                config.state_dir.display()
            ),
            Some(
                "run `doro-agent preflight` first and verify the container can write state_dir"
                    .to_string(),
            ),
        )
    });

    match load_snapshot(&snapshot_path) {
        Ok(snapshot) => checks.extend(snapshot_to_checks(&config, snapshot, &snapshot_path)),
        Err(error) => checks.push(fail_check(
            "diagnostics-snapshot",
            "health",
            format!(
                "failed to read local diagnostics snapshot `{}`: {error}",
                snapshot_path.display()
            ),
            Some("the runtime must produce and refresh the local diagnostics snapshot before health can pass".to_string()),
        )),
    }

    build_report("health", config_path_text, generated_at, checks)
}

fn load_snapshot(path: &Path) -> Result<DiagnosticsSnapshot, String> {
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&raw).map_err(|error| error.to_string())
}

fn snapshot_to_checks(
    config: &AgentConfig,
    snapshot: DiagnosticsSnapshot,
    snapshot_path: &Path,
) -> Vec<ReportCheck> {
    let mut checks = Vec::new();
    let freshness_sec = health_freshness_sec(config);
    let now_unix_ms = Utc::now().timestamp_millis();

    let generated_at_unix_ms = chrono::DateTime::parse_from_rfc3339(&snapshot.generated_at)
        .map(|value| value.timestamp_millis())
        .unwrap_or_default();
    let snapshot_age_sec = age_sec_from_unix_ms(now_unix_ms, Some(generated_at_unix_ms));

    checks.push(if snapshot_age_sec <= freshness_sec {
        pass_check(
            "diagnostics-snapshot",
            "health",
            format!(
                "local diagnostics snapshot `{}` is fresh (age={}s, threshold={}s)",
                snapshot_path.display(),
                snapshot_age_sec,
                freshness_sec
            ),
            None,
        )
    } else {
        fail_check(
            "diagnostics-snapshot",
            "health",
            format!(
                "local diagnostics snapshot `{}` is stale (age={}s, threshold={}s)",
                snapshot_path.display(),
                snapshot_age_sec,
                freshness_sec
            ),
            Some("a stale snapshot usually means the runtime or diagnostics loop is no longer progressing".to_string()),
        )
    });

    checks.push(runtime_phase_check(&snapshot));
    checks.push(transport_connectivity_check(
        &snapshot,
        freshness_sec,
        now_unix_ms,
    ));
    checks.push(delivery_check(&snapshot));
    checks.push(loop_check(
        "heartbeat-loop",
        "runtime",
        "heartbeat",
        snapshot.heartbeat_state.scheduler_running,
        snapshot.heartbeat_state.last_attempt_at,
        snapshot.heartbeat_state.last_success_at,
        snapshot.heartbeat_state.last_error.clone(),
        freshness_sec,
        now_unix_ms,
    ));
    checks.push(loop_check(
        "diagnostics-loop",
        "runtime",
        "diagnostics",
        snapshot.diagnostics_state.scheduler_running,
        snapshot.diagnostics_state.last_attempt_at,
        snapshot.diagnostics_state.last_success_at,
        snapshot.diagnostics_state.last_error.clone(),
        freshness_sec,
        now_unix_ms,
    ));
    checks.push(if snapshot.state.state_db_accessible {
        pass_check(
            "state-db",
            "state",
            format!("state db `{}` is accessible", snapshot.state.state_db_path),
            None,
        )
    } else {
        fail_check(
            "state-db",
            "state",
            format!(
                "state db `{}` is not accessible",
                snapshot.state.state_db_path
            ),
            Some("health cannot be healthy when local persisted state is unavailable".to_string()),
        )
    });
    checks.push(source_check(&snapshot));

    checks
}

fn runtime_phase_check(snapshot: &DiagnosticsSnapshot) -> ReportCheck {
    let phase = RuntimePhase::parse(&snapshot.runtime_status).unwrap_or(RuntimePhase::Error);
    match phase {
        RuntimePhase::Online => pass_check("runtime-phase", "runtime", "runtime phase is `online`", None),
        RuntimePhase::Degraded => warn_check(
            "runtime-phase",
            "runtime",
            format!(
                "runtime phase is `degraded`: {}",
                snapshot
                    .runtime_status_reason
                    .clone()
                    .unwrap_or_else(|| "no reason recorded".to_string())
            ),
            Some("the agent is alive but partially degraded; inspect degraded_reason, warning_list, and failure_list".to_string()),
        ),
        RuntimePhase::Starting | RuntimePhase::Enrolling | RuntimePhase::PolicySyncing => fail_check(
            "runtime-phase",
            "runtime",
            format!(
                "runtime phase is `{}`; the agent is still warming up",
                phase.as_str()
            ),
            Some("health becomes green only after startup, enrollment, and policy sync finish".to_string()),
        ),
        RuntimePhase::Stopping | RuntimePhase::Error => fail_check(
            "runtime-phase",
            "runtime",
            format!(
                "runtime phase is `{}`{}",
                phase.as_str(),
                snapshot
                    .runtime_status_reason
                    .as_deref()
                    .map(|reason| format!(": {reason}"))
                    .unwrap_or_default()
            ),
            Some("inspect local logs and the persisted diagnostics snapshot for the fatal transition".to_string()),
        ),
    }
}

fn transport_connectivity_check(
    snapshot: &DiagnosticsSnapshot,
    freshness_sec: u64,
    now_unix_ms: i64,
) -> ReportCheck {
    if snapshot.transport_mode == "mock" {
        return pass_check(
            "transport-connectivity",
            "transport",
            "mock transport does not require edge connectivity",
            None,
        );
    }

    let handshake_age_sec = age_sec_from_unix_ms(
        now_unix_ms,
        snapshot.connectivity_state.last_handshake_success_at,
    );
    if snapshot
        .connectivity_state
        .last_handshake_success_at
        .is_some()
        && handshake_age_sec <= freshness_sec
    {
        return pass_check(
            "transport-connectivity",
            "transport",
            format!(
                "last successful edge handshake is fresh (age={}s, threshold={}s)",
                handshake_age_sec, freshness_sec
            ),
            None,
        );
    }

    fail_check(
        "transport-connectivity",
        "transport",
        format!(
            "no recent successful edge handshake is recorded{}",
            snapshot
                .last_transport_error
                .as_deref()
                .map(|error| format!("; last transport error: {error}"))
                .unwrap_or_default()
        ),
        Some("check enrollment, TLS, edge reachability, and bootstrap token validity".to_string()),
    )
}

fn delivery_check(snapshot: &DiagnosticsSnapshot) -> ReportCheck {
    if snapshot.blocked_delivery {
        return fail_check(
            "delivery",
            "transport",
            format!(
                "delivery is blocked{}",
                snapshot
                    .blocked_reason
                    .as_deref()
                    .map(|reason| format!(": {reason}"))
                    .unwrap_or_default()
            ),
            Some("blocked delivery means the sender hit a permanent transport failure and stopped live delivery".to_string()),
        );
    }

    pass_check("delivery", "transport", "delivery is not blocked", None)
}

fn loop_check(
    check_name: &str,
    category: &str,
    loop_name: &str,
    scheduler_running: bool,
    last_attempt_at: Option<i64>,
    last_success_at: Option<i64>,
    last_error: Option<String>,
    freshness_sec: u64,
    now_unix_ms: i64,
) -> ReportCheck {
    if !scheduler_running {
        return fail_check(
            check_name,
            category,
            format!("{loop_name} scheduler is not marked as running"),
            Some("the runtime should mark scheduler state transitions explicitly".to_string()),
        );
    }

    let attempt_age_sec = age_sec_from_unix_ms(now_unix_ms, last_attempt_at);
    if last_attempt_at.is_none() || attempt_age_sec > freshness_sec.saturating_mul(2) {
        return fail_check(
            check_name,
            category,
            format!(
                "{loop_name} scheduler has no recent activity (age={}s, threshold={}s)",
                attempt_age_sec,
                freshness_sec.saturating_mul(2)
            ),
            Some("a stalled scheduler usually means the runtime loop died or stopped making progress".to_string()),
        );
    }

    let detail = format!(
        "{loop_name} scheduler is active; last_success_at={:?}, last_error={:?}",
        last_success_at, last_error
    );
    if last_error.is_some() {
        warn_check(
            check_name,
            category,
            detail,
            Some("the loop is alive, but the last delivery attempt failed".to_string()),
        )
    } else {
        pass_check(check_name, category, detail, None)
    }
}

fn source_check(snapshot: &DiagnosticsSnapshot) -> ReportCheck {
    if snapshot.source_summary.total == 0 {
        return warn_check(
            "sources",
            "sources",
            "no active sources are configured yet",
            Some("this is expected before the first edge policy fetch, but it also explains why logs are not shipping".to_string()),
        );
    }

    if snapshot.source_summary.readable == 0 {
        return fail_check(
            "sources",
            "sources",
            format!(
                "no readable sources are active (total={}, missing={}, unreadable={})",
                snapshot.source_summary.total,
                snapshot.source_summary.missing,
                snapshot.source_summary.unreadable
            ),
            Some("inspect source_statuses, permission_issues, and warning_list to find the broken paths".to_string()),
        );
    }

    if snapshot.source_summary.missing > 0 || snapshot.source_summary.unreadable > 0 {
        return warn_check(
            "sources",
            "sources",
            format!(
                "runtime has readable sources, but some paths are degraded (readable={}, missing={}, unreadable={})",
                snapshot.source_summary.readable,
                snapshot.source_summary.missing,
                snapshot.source_summary.unreadable
            ),
            Some("the agent can stay alive with partial source degradation, but missing logs need operator attention".to_string()),
        );
    }

    pass_check(
        "sources",
        "sources",
        format!(
            "readable sources are active (total={}, readable={}, running={})",
            snapshot.source_summary.total,
            snapshot.source_summary.readable,
            snapshot.source_summary.running
        ),
        None,
    )
}

fn build_report(
    report_kind: &str,
    config_path: String,
    generated_at: String,
    checks: Vec<ReportCheck>,
) -> OperationalReport {
    let warning_count = checks
        .iter()
        .filter(|check| check.status == CheckStatus::Warn)
        .count();
    let failure_count = checks
        .iter()
        .filter(|check| check.status == CheckStatus::Fail)
        .count();
    let warming_up = checks.iter().any(|check| {
        check.name == "runtime-phase"
            && check.status == CheckStatus::Fail
            && check.detail.contains("warming up")
    });

    OperationalReport {
        report_kind: report_kind.to_string(),
        config_path,
        generated_at,
        summary: ReportSummary {
            check_count: checks.len(),
            warning_count,
            failure_count,
            overall_status: if failure_count > 0 {
                if warming_up {
                    OverallStatus::WarmingUp
                } else {
                    OverallStatus::Unhealthy
                }
            } else {
                OverallStatus::Healthy
            },
        },
        checks,
    }
}

fn health_freshness_sec(config: &AgentConfig) -> u64 {
    let interval = config
        .heartbeat
        .interval_sec
        .max(config.diagnostics.interval_sec)
        .max(1);
    interval.saturating_mul(3).max(DEFAULT_HEALTH_FRESHNESS_SEC)
}

fn age_sec_from_unix_ms(now_unix_ms: i64, timestamp_unix_ms: Option<i64>) -> u64 {
    timestamp_unix_ms
        .map(|value| now_unix_ms.saturating_sub(value).max(0) as u64 / 1_000)
        .unwrap_or(u64::MAX)
}

fn pass_check(
    name: &str,
    category: &str,
    detail: impl Into<String>,
    hint: Option<&str>,
) -> ReportCheck {
    ReportCheck::new(
        name,
        CheckStatus::Pass,
        category,
        detail.into(),
        hint.map(str::to_string),
    )
}

fn warn_check(
    name: &str,
    category: &str,
    detail: impl Into<String>,
    hint: Option<String>,
) -> ReportCheck {
    ReportCheck::new(name, CheckStatus::Warn, category, detail.into(), hint)
}

fn fail_check(
    name: &str,
    category: &str,
    detail: impl Into<String>,
    hint: Option<String>,
) -> ReportCheck {
    ReportCheck::new(name, CheckStatus::Fail, category, detail.into(), hint)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use chrono::{SecondsFormat, Utc};
    use tempfile::TempDir;

    use crate::runtime::{
        ConnectivityStateSnapshot, DiagnosticsDeliveryStateSnapshot, DiagnosticsSnapshot,
        EnrollmentStateSnapshot, HeartbeatStateSnapshot, PolicyStateSnapshot, SourceStatusSnapshot,
        SourceSummarySnapshot, StateSnapshot, TransportStateSnapshot,
    };

    use super::run;

    fn write_config(dir: &TempDir) -> PathBuf {
        let config_path = dir.path().join("agent.yaml");
        fs::write(
            &config_path,
            format!(
                r#"
edge_url: "https://edge.example.local"
edge_grpc_addr: "edge.example.local:7443"
bootstrap_token: "token"
state_dir: '{}'
transport:
  mode: "mock"
"#,
                dir.path().join("state").display()
            ),
        )
        .unwrap();
        config_path
    }

    #[test]
    fn health_fails_without_local_snapshot() {
        let dir = TempDir::new().unwrap();
        let config_path = write_config(&dir);
        let report = run(&config_path);
        assert!(report.summary.failure_count > 0);
    }

    #[test]
    fn health_reads_local_snapshot() {
        let dir = TempDir::new().unwrap();
        let config_path = write_config(&dir);
        let state_dir = dir.path().join("state");
        fs::create_dir_all(state_dir.join("runtime")).unwrap();
        fs::write(state_dir.join("state.db"), b"placeholder").unwrap();
        let snapshot = DiagnosticsSnapshot {
            generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            agent_id: "agent-1".to_string(),
            hostname: "demo-host".to_string(),
            version: "0.1.0".to_string(),
            install_mode: "dev".to_string(),
            transport_mode: "mock".to_string(),
            uptime_sec: 10,
            current_policy_revision: None,
            runtime_status: "online".to_string(),
            runtime_status_reason: None,
            degraded_mode: false,
            degraded_reason: None,
            blocked_delivery: false,
            blocked_reason: None,
            runtime_mode: "normal".to_string(),
            active_sources: 1,
            event_queue_len: 0,
            event_queue_bytes: 0,
            send_queue_len: 0,
            send_queue_bytes: 0,
            spool_enabled: true,
            spooled_batches: 0,
            spooled_bytes: 0,
            last_error: None,
            last_error_kind: None,
            last_transport_error: None,
            last_successful_send_at: Some(1),
            consecutive_send_failures: 0,
            enrollment_state: EnrollmentStateSnapshot {
                status: "reused".to_string(),
                reason: Some("persisted identity accepted".to_string()),
                agent_id_present: true,
            },
            heartbeat_state: HeartbeatStateSnapshot {
                interval_sec: 30,
                scheduler_running: true,
                last_attempt_at: Some(Utc::now().timestamp_millis()),
                last_success_at: Some(Utc::now().timestamp_millis()),
                last_error: None,
                consecutive_failures: 0,
            },
            diagnostics_state: DiagnosticsDeliveryStateSnapshot {
                interval_sec: 30,
                scheduler_running: true,
                last_attempt_at: Some(Utc::now().timestamp_millis()),
                last_success_at: Some(Utc::now().timestamp_millis()),
                last_error: None,
                consecutive_failures: 0,
                last_local_snapshot_at: Some(Utc::now().timestamp_millis()),
                local_snapshot_path: Some(
                    state_dir
                        .join("runtime")
                        .join("diagnostics-snapshot.json")
                        .display()
                        .to_string(),
                ),
            },
            source_summary: SourceSummarySnapshot {
                total: 1,
                readable: 1,
                missing: 0,
                unreadable: 0,
                running: 1,
                waiting: 0,
                error: 0,
                idle: 0,
            },
            transport_state: TransportStateSnapshot {
                mode: "mock".to_string(),
                server_unavailable_for_sec: 0,
                last_error_kind: None,
                blocked_delivery: false,
                blocked_reason: None,
            },
            policy_state: PolicyStateSnapshot {
                current_policy_revision: None,
                last_policy_fetch_at: None,
                last_policy_apply_at: None,
                last_policy_error: None,
                active_source_count: 1,
            },
            connectivity_state: ConnectivityStateSnapshot {
                endpoint: "http://localhost:9090".to_string(),
                tls_enabled: false,
                mtls_enabled: false,
                server_name: None,
                ca_path: None,
                cert_path: None,
                key_path: None,
                ca_path_present: false,
                cert_path_present: false,
                key_path_present: false,
                last_connect_error: None,
                last_tls_error: None,
                last_handshake_success_at: Some(Utc::now().timestamp_millis()),
            },
            source_statuses: vec![SourceStatusSnapshot {
                source_id: "file:/tmp/demo.log".to_string(),
                path: "/tmp/demo.log".to_string(),
                source: "demo".to_string(),
                service: "svc".to_string(),
                status: "running".to_string(),
                inode: Some(2),
                live_read_offset: 1,
                durable_read_offset: 1,
                acked_offset: 1,
                live_pending_bytes: 0,
                durable_pending_bytes: 0,
                last_read_at: Some(1),
                last_error: None,
            }],
            platform: crate::runtime::test_static_context().metadata.platform,
            build: crate::runtime::test_static_context().metadata.build,
            install: crate::runtime::test_static_context().metadata.install,
            paths: crate::runtime::test_static_context().metadata.paths,
            state: StateSnapshot {
                state_db_path: state_dir.join("state.db").display().to_string(),
                state_db_exists: true,
                state_db_accessible: true,
                persisted_identity_present: true,
                current_policy_revision: None,
                last_known_edge_url: Some("http://localhost:8080".to_string()),
                spool_enabled: true,
                spooled_batches: 0,
                spooled_bytes: 0,
                last_successful_send_at: Some(1),
            },
            compatibility: Default::default(),
            cluster: crate::runtime::test_static_context().metadata.cluster,
            identity_status: crate::metadata::IdentityStatusSnapshot {
                status: "reused".to_string(),
                reason: None,
            },
            security_posture: Default::default(),
            permission_issues: Vec::new(),
            warning_list: Vec::new(),
            failure_list: Vec::new(),
        };
        fs::write(
            state_dir.join("runtime").join("diagnostics-snapshot.json"),
            serde_json::to_string_pretty(&snapshot).unwrap(),
        )
        .unwrap();

        let report = run(&config_path);
        assert_eq!(
            report.summary.failure_count,
            0,
            "{}",
            serde_json::to_string_pretty(&report).unwrap()
        );
    }
}
