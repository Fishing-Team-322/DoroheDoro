use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};

use crate::{
    config::AgentConfig,
    error::AppResult,
    ops::{CheckStatus, OperationalReport, OverallStatus, ReportCheck, ReportSummary},
    runtime::{diagnostics::local_diagnostics_snapshot_path, DiagnosticsSnapshot, RuntimePhase},
};

const DEFAULT_HEALTH_FRESHNESS_SEC: u64 = 90;

pub fn run(config_path: &Path) -> AppResult<OperationalReport> {
    let config = AgentConfig::load(config_path)?;
    let snapshot_path = local_diagnostics_snapshot_path(&config.state_dir);
    let mut checks = Vec::new();

    checks.push(pass_check(
        "config",
        "config",
        "configuration parsed successfully".to_string(),
        None,
    ));

    if !config.state_dir.exists() {
        checks.push(fail_check(
            "state-dir",
            "state",
            format!(
                "state dir `{}` does not exist; the runtime has not initialized local state yet",
                config.state_dir.display()
            ),
            Some("run `doro-agent preflight` first and verify the container can write state_dir".to_string()),
        ));
    } else {
        checks.push(pass_check(
            "state-dir",
            "state",
            format!("state dir `{}` exists", config.state_dir.display()),
            None,
        ));
    }

    checks.push(match load_snapshot(&snapshot_path) {
        Ok(snapshot) => snapshot_to_checks(&config, snapshot, &snapshot_path),
        Err(error) => {
            let mut pending_checks = Vec::new();
            pending_checks.push(fail_check(
                "diagnostics-snapshot",
                "health",
                format!(
                    "failed to read local diagnostics snapshot `{}`: {error}",
                    snapshot_path.display()
                ),
                Some("the runtime must produce and refresh the local diagnostics snapshot before health can pass".to_string()),
            ));
            pending_checks
        }
    });

    let checks = checks.into_iter().flatten().collect::<Vec<_>>();
    Ok(OperationalReport {
        report_kind: "health".to_string(),
        config_path: config_path.display().to_string(),
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        summary: summarize_health(&checks),
        checks,
    })
}

fn load_snapshot(path: &Path) -> AppResult<DiagnosticsSnapshot> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn snapshot_to_checks(
    config: &AgentConfig,
    snapshot: DiagnosticsSnapshot,
    snapshot_path: &Path,
) -> Vec<ReportCheck> {
    let mut checks = Vec::new();
    let freshness_sec = health_freshness_sec(config);
    let now = Utc::now().timestamp();
    let generated_at = chrono::DateTime::parse_from_rfc3339(&snapshot.generated_at)
        .map(|value| value.timestamp())
        .unwrap_or_default();
    let age_sec = now.saturating_sub(generated_at).max(0) as u64;

    checks.push(if age_sec <= freshness_sec {
        pass_check(
            "diagnostics-snapshot",
            "health",
            format!(
                "local diagnostics snapshot `{}` is fresh (age={}s, threshold={}s)",
                snapshot_path.display(),
                age_sec,
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
                age_sec,
                freshness_sec
            ),
            Some("a stale snapshot usually means the runtime or diagnostics loop is no longer progressing".to_string()),
        )
    });

    let phase = RuntimePhase::parse(&snapshot.runtime_status).unwrap_or(RuntimePhase::Error);
    checks.push(match phase {
        RuntimePhase::Online => pass_check(
            "runtime-phase",
            "runtime",
            "runtime phase is `online`".to_string(),
            None,
        ),
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
            Some("the agent is alive, but operators should inspect degraded_reason and failure_list".to_string()),
        ),
        RuntimePhase::Starting | RuntimePhase::Enrolling | RuntimePhase::PolicySyncing => fail_check(
            "runtime-phase",
            "runtime",
            format!(
                "runtime phase is `{}`; the agent is still warming up",
                phase.as_str()
            ),
            Some("health returns success only after the runtime is fully started".to_string()),
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
    });

    checks.push(if snapshot.connectivity_state.last_handshake_success_at.is_some()
        || snapshot.transport_mode == "mock"
    {
        pass_check(
            "transport-connectivity",
            "transport",
            format!(
                "transport mode `{}` has a successful handshake history or does not require edge connectivity",
                snapshot.transport_mode
            ),
            None,
        )
    } else {
        fail_check(
            "transport-connectivity",
            "transport",
            "no successful connectivity handshake has been recorded yet".to_string(),
            Some("check enrollment, TLS, edge reachability, and bootstrap token validity".to_string()),
        )
    });

    checks.push(if snapshot.blocked_delivery {
        fail_check(
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
        )
    } else {
        pass_check(
            "delivery",
            "transport",
            "delivery is not blocked".to_string(),
            None,
        )
    });

    checks.push(if snapshot.heartbeat_state.scheduler_running
        && snapshot.heartbeat_state.last_attempt_at.is_some()
    {
        pass_check(
            "heartbeat-loop",
            "runtime",
            format!(
                "heartbeat scheduler is running; last_success_at={:?}",
                snapshot.heartbeat_state.last_success_at
            ),
            None,
        )
    } else {
        fail_check(
            "heartbeat-loop",
            "runtime",
            "heartbeat scheduler has no recorded activity".to_string(),
            Some("the runtime should keep updating heartbeat_state even when the edge is temporarily unavailable".to_string()),
        )
    });

    checks.push(if snapshot.diagnostics_state.scheduler_running
        && snapshot.diagnostics_state.last_local_snapshot_at.is_some()
    {
        pass_check(
            "diagnostics-loop",
            "runtime",
            format!(
                "diagnostics scheduler is running; local snapshot path={}",
                snapshot
                    .diagnostics_state
                    .local_snapshot_path
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            None,
        )
    } else {
        fail_check(
            "diagnostics-loop",
            "runtime",
            "diagnostics scheduler has no recorded local snapshot activity".to_string(),
            Some("diagnostics must keep producing local snapshots even when remote send fails".to_string()),
        )
    });

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
            format!("state db `{}` is not accessible", snapshot.state.state_db_path),
            Some("health cannot be healthy when local persisted state is unavailable".to_string()),
        )
    });

    checks.push(if snapshot.source_summary.total == 0 {
        warn_check(
            "sources",
            "sources",
            "no active sources are configured yet".to_string(),
            Some("this is expected before the first edge policy fetch, but it also explains why logs are not shipping".to_string()),
        )
    } else if snapshot.source_summary.readable == 0 {
        fail_check(
            "sources",
            "sources",
            format!(
                "no readable sources are active (total={}, missing={}, unreadable={})",
                snapshot.source_summary.total,
                snapshot.source_summary.missing,
                snapshot.source_summary.unreadable
            ),
            Some("inspect source_statuses, permission_issues, and warning_list to find the broken paths".to_string()),
        )
    } else {
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
    });

    checks
}

fn summarize_health(checks: &[ReportCheck]) -> ReportSummary {
    let warning_count = checks
        .iter()
        .filter(|check| check.status == CheckStatus::Warn)
        .count();
    let failure_count = checks
        .iter()
        .filter(|check| check.status == CheckStatus::Fail)
        .count();
    let overall_status = if failure_count > 0 {
        if checks.iter().any(|check| check.name == "runtime-phase") {
            OverallStatus::Unhealthy
        } else {
            OverallStatus::Unhealthy
        }
    } else if warning_count > 0 {
        OverallStatus::Healthy
    } else {
        OverallStatus::Healthy
    };

    ReportSummary {
        check_count: checks.len(),
        warning_count,
        failure_count,
        overall_status,
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

fn pass_check(
    name: &str,
    category: &str,
    detail: String,
    hint: Option<String>,
) -> ReportCheck {
    ReportCheck {
        name: name.to_string(),
        status: CheckStatus::Pass,
        detail,
        category: category.to_string(),
        hint,
    }
}

fn warn_check(
    name: &str,
    category: &str,
    detail: String,
    hint: Option<String>,
) -> ReportCheck {
    ReportCheck {
        name: name.to_string(),
        status: CheckStatus::Warn,
        detail,
        category: category.to_string(),
        hint,
    }
}

fn fail_check(
    name: &str,
    category: &str,
    detail: String,
    hint: Option<String>,
) -> ReportCheck {
    ReportCheck {
        name: name.to_string(),
        status: CheckStatus::Fail,
        detail,
        category: category.to_string(),
        hint,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use crate::runtime::{
        ConnectivityStateSnapshot, DiagnosticsDeliveryStateSnapshot, DiagnosticsSnapshot,
        EnrollmentStateSnapshot, HeartbeatStateSnapshot, PolicyStateSnapshot,
        SourceStatusSnapshot, SourceSummarySnapshot, StateSnapshot, TransportStateSnapshot,
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
state_dir: "{}"
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
        let report = run(&config_path).unwrap();
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
                last_attempt_at: Some(1),
                last_success_at: Some(1),
                last_error: None,
                consecutive_failures: 0,
            },
            diagnostics_state: DiagnosticsDeliveryStateSnapshot {
                interval_sec: 30,
                scheduler_running: true,
                last_attempt_at: Some(1),
                last_success_at: Some(1),
                last_error: None,
                consecutive_failures: 0,
                last_local_snapshot_at: Some(1),
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
                last_handshake_success_at: Some(1),
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

        let report = run(&config_path).unwrap();
        assert_eq!(report.summary.failure_count, 0);
    }
}
