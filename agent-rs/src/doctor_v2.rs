use std::{
    net::{TcpStream, ToSocketAddrs},
    path::Path,
    time::Duration,
};

use chrono::{SecondsFormat, Utc};
use reqwest::Url;
use rusqlite::{Connection, OpenFlags};

use crate::{
    config::AgentConfig,
    metadata::{
        can_read_file, detect_source_paths, directory_write_access, path_exists,
        RuntimeMetadataContext, SourcePathStatus, CANONICAL_LOG_DIR,
    },
    ops::{CheckStatus, OperationalReport, OverallStatus, ReportCheck, ReportSummary},
    security::{SecurityRulesFile, SECURITY_SCHEMA_VERSION},
    transport::client::{
        build_base_url, derive_server_name, endpoint_uses_tls, load_ca_certificate,
        load_client_identity, EdgeGrpcTransport,
    },
};

const CATEGORY_CONFIG: &str = "config";
const CATEGORY_INSTALL: &str = "install";
const CATEGORY_RUNTIME: &str = "runtime";
const CATEGORY_ENVIRONMENT: &str = "environment";
const CATEGORY_STORAGE: &str = "storage";
const CATEGORY_SECURITY: &str = "security";
const CATEGORY_TRANSPORT: &str = "transport";
const CATEGORY_TLS: &str = "tls";
const CATEGORY_SOURCES: &str = "sources";
const CATEGORY_COMPATIBILITY: &str = "compatibility";
const CATEGORY_SCOPE: &str = "scope";

const REACHABILITY_TIMEOUT: Duration = Duration::from_millis(750);

pub fn run(config_path: &Path) -> OperationalReport {
    let config_path_text = config_path.display().to_string();
    let generated_at = iso_timestamp();

    let config = match AgentConfig::load(config_path) {
        Ok(config) => config,
        Err(error) => {
            return build_report(
                "doctor",
                config_path_text,
                generated_at,
                vec![ReportCheck::new(
                    "config",
                    CheckStatus::Fail,
                    CATEGORY_CONFIG,
                    format!("failed to load configuration: {error}"),
                    Some(
                        "fix the config file or the AGENT_CONFIG path and rerun preflight"
                            .to_string(),
                    ),
                )],
            );
        }
    };

    let hostname = crate::app::resolve_hostname();
    let context = match RuntimeMetadataContext::detect(&config, config_path, &hostname) {
        Ok(context) => context,
        Err(error) => {
            return build_report(
                "doctor",
                config_path_text,
                generated_at,
                vec![
                    pass_check(
                        "config",
                        CATEGORY_CONFIG,
                        "configuration parsed successfully",
                        None,
                    ),
                    ReportCheck::new(
                        "runtime-metadata",
                        CheckStatus::Fail,
                        CATEGORY_RUNTIME,
                        format!("failed to detect runtime metadata: {error}"),
                        Some(
                            "verify the runtime user can inspect local platform and path metadata"
                                .to_string(),
                        ),
                    ),
                ],
            );
        }
    };

    let mut checks = Vec::new();
    checks.push(pass_check(
        "config",
        CATEGORY_CONFIG,
        "configuration parsed successfully",
        None,
    ));
    checks.push(check_install_mode(&context));
    checks.push(pass_check(
        "build-runtime-metadata",
        CATEGORY_RUNTIME,
        format!(
            "version={}, build_id={}, target={}, profile={}",
            context.build.agent_version,
            context.build.build_id,
            context.build.target_triple,
            context.build.build_profile
        ),
        None,
    ));
    checks.push(check_runtime_environment(&context));
    checks.push(check_directory(
        "state-dir",
        &config.state_dir,
        "runtime state",
        CATEGORY_STORAGE,
        true,
        Some("ensure the runtime user can create and write inside state_dir"),
    ));
    checks.push(check_directory(
        "runtime-artifacts-dir",
        &config.state_dir.join("runtime"),
        "local runtime artifacts",
        CATEGORY_STORAGE,
        true,
        Some("the agent writes health and diagnostics snapshots here during runtime"),
    ));
    if config.spool.enabled {
        checks.push(check_directory(
            "spool-dir",
            &config.spool.dir,
            "fallback spool",
            CATEGORY_STORAGE,
            true,
            Some("ensure spool dir is on persistent writable storage"),
        ));
    } else {
        checks.push(pass_check(
            "spool-dir",
            CATEGORY_STORAGE,
            "spool is disabled",
            Some("enable spool for safer recovery during edge outages"),
        ));
    }
    checks.push(check_state_db(&config.state_dir.join("state.db")));
    checks.extend(check_security_scan(&config));
    checks.push(check_transport(&config));
    checks.push(check_transport_reachability(&config));
    checks.push(check_logs_directory(&context));
    checks.extend(check_tls(&config));
    checks.extend(check_sources(&config));
    checks.extend(check_compatibility(&context));
    checks.push(pass_check(
        "cluster-scope",
        CATEGORY_SCOPE,
        format!(
            "cluster_id={:?}, effective_cluster_tags={}, host_labels={}",
            context.cluster.configured_cluster_id,
            context.cluster.effective_cluster_tags.len(),
            context.cluster.host_labels.len()
        ),
        Some("set scope fields if the agent should report into a named cluster or service"),
    ));

    build_report("doctor", config_path_text, generated_at, checks)
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

    OperationalReport {
        report_kind: report_kind.to_string(),
        config_path,
        generated_at,
        summary: ReportSummary {
            check_count: checks.len(),
            warning_count,
            failure_count,
            overall_status: if failure_count > 0 {
                OverallStatus::Fail
            } else if warning_count > 0 {
                OverallStatus::Warn
            } else {
                OverallStatus::Pass
            },
        },
        checks,
    }
}

fn check_install_mode(context: &RuntimeMetadataContext) -> ReportCheck {
    let has_warnings =
        context.install.resolved_mode == "unknown" || !context.install.warnings.is_empty();
    ReportCheck::new(
        "install-mode",
        if has_warnings {
            CheckStatus::Warn
        } else {
            CheckStatus::Pass
        },
        CATEGORY_INSTALL,
        format!(
            "configured={}, resolved={}, source={}",
            context.install.configured_mode,
            context.install.resolved_mode,
            context.install.resolution_source
        ),
        Some(
            "pin install.mode if the layout is intentional and auto-detection stays ambiguous"
                .to_string(),
        ),
    )
}

fn check_runtime_environment(context: &RuntimeMetadataContext) -> ReportCheck {
    let container_runtime = detect_container_runtime();
    let detail = format!(
        "service_manager={}, systemd_detected={}, systemd_expected={}, container_runtime={}",
        context.platform.service_manager,
        context.platform.systemd_detected,
        context.install.systemd_expected,
        container_runtime.as_deref().unwrap_or("none")
    );

    if context.install.systemd_expected && !context.platform.systemd_detected {
        return ReportCheck::new(
            "runtime-environment",
            CheckStatus::Warn,
            CATEGORY_ENVIRONMENT,
            detail,
            Some(
                "package/ansible installs expect systemd; inside a container, run the binary entrypoint directly instead"
                    .to_string(),
            ),
        );
    }

    pass_check("runtime-environment", CATEGORY_ENVIRONMENT, detail, None)
}

fn check_directory(
    name: &str,
    path: &Path,
    label: &str,
    category: &str,
    require_write: bool,
    hint: Option<&str>,
) -> ReportCheck {
    if path_exists(path) {
        if !path.is_dir() {
            return ReportCheck::new(
                name,
                CheckStatus::Fail,
                category,
                format!(
                    "{} path `{}` exists but is not a directory",
                    label,
                    path.display()
                ),
                hint.map(str::to_string),
            );
        }

        if require_write && !directory_write_access(path) {
            return ReportCheck::new(
                name,
                CheckStatus::Fail,
                category,
                format!("{} path `{}` is not writable", label, path.display()),
                hint.map(str::to_string),
            );
        }

        return ReportCheck::new(
            name,
            CheckStatus::Pass,
            category,
            format!("{} path `{}` is available", label, path.display()),
            None,
        );
    }

    let parent = path.parent().unwrap_or(path);
    if directory_write_access(parent) {
        ReportCheck::new(
            name,
            CheckStatus::Warn,
            category,
            format!(
                "{} path `{}` does not exist yet but parent `{}` is writable",
                label,
                path.display(),
                parent.display()
            ),
            hint.map(str::to_string),
        )
    } else {
        ReportCheck::new(
            name,
            CheckStatus::Fail,
            category,
            format!(
                "{} path `{}` does not exist and parent `{}` is not writable",
                label,
                path.display(),
                parent.display()
            ),
            hint.map(str::to_string),
        )
    }
}

fn check_state_db(state_db_path: &Path) -> ReportCheck {
    if !path_exists(state_db_path) {
        let parent = state_db_path.parent().unwrap_or(state_db_path);
        return if directory_write_access(parent) {
            ReportCheck::new(
                "state-db",
                CheckStatus::Warn,
                CATEGORY_STORAGE,
                format!(
                    "state database `{}` does not exist yet; runtime should create it",
                    state_db_path.display()
                ),
                Some(
                    "the first successful runtime start should create state.db automatically"
                        .to_string(),
                ),
            )
        } else {
            ReportCheck::new(
                "state-db",
                CheckStatus::Fail,
                CATEGORY_STORAGE,
                format!(
                    "state database `{}` is missing and parent `{}` is not writable",
                    state_db_path.display(),
                    parent.display()
                ),
                Some("fix state_dir permissions before starting the agent".to_string()),
            )
        };
    }

    match Connection::open_with_flags(state_db_path, OpenFlags::SQLITE_OPEN_READ_WRITE) {
        Ok(_) => pass_check(
            "state-db",
            CATEGORY_STORAGE,
            format!("state database `{}` is accessible", state_db_path.display()),
            None,
        ),
        Err(error) => ReportCheck::new(
            "state-db",
            CheckStatus::Fail,
            CATEGORY_STORAGE,
            format!(
                "state database `{}` is not accessible: {error}",
                state_db_path.display()
            ),
            Some(
                "fix SQLite file ownership, permissions, or corruption before starting the agent"
                    .to_string(),
            ),
        ),
    }
}

fn check_security_scan(config: &AgentConfig) -> Vec<ReportCheck> {
    let mut checks = Vec::new();
    if !config.security_scan.enabled {
        checks.push(pass_check(
            "security-scan",
            CATEGORY_SECURITY,
            "security posture scan is disabled",
            Some("enable security_scan when you want local Linux posture checks"),
        ));
        return checks;
    }

    checks.push(ReportCheck::new(
        "security-scan",
        if cfg!(target_os = "linux") {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        CATEGORY_SECURITY,
        format!(
            "enabled=true, profile={}, interval={}s, jitter={}s, timeout={}s, max_parallel_checks={}",
            config.security_scan.profile.as_str(),
            config.security_scan.interval_sec,
            config.security_scan.jitter_sec,
            config.security_scan.timeout_sec,
            config.security_scan.max_parallel_checks
        ),
        if cfg!(target_os = "linux") {
            None
        } else {
            Some("the runtime feature is intended for Linux hosts; non-Linux validation is inherently partial".to_string())
        },
    ));

    let report_dir = config.state_dir.join("security");
    if config.security_scan.persist_last_report {
        checks.push(check_directory(
            "security-cache",
            &report_dir,
            "security report cache",
            CATEGORY_SECURITY,
            true,
            Some("the last security report is persisted here for troubleshooting"),
        ));
    } else {
        checks.push(pass_check(
            "security-cache",
            CATEGORY_SECURITY,
            "persist_last_report is disabled",
            Some("enable persist_last_report to keep the latest posture artifact on disk"),
        ));
    }

    let rules_path = &config.security_scan.version_rules_path;
    if !rules_path.exists() {
        checks.push(ReportCheck::new(
            "security-rules",
            CheckStatus::Warn,
            CATEGORY_SECURITY,
            format!(
                "security rules file `{}` is missing; package checks will run without minimum secure versions",
                rules_path.display()
            ),
            Some("install the security rules file on Linux hosts to enable package version thresholds".to_string()),
        ));
        return checks;
    }

    match std::fs::read_to_string(rules_path) {
        Ok(raw) => match serde_yaml::from_str::<SecurityRulesFile>(&raw) {
            Ok(file) if file.schema_version == SECURITY_SCHEMA_VERSION => {
                checks.push(pass_check(
                    "security-rules",
                    CATEGORY_SECURITY,
                    format!(
                        "security rules file `{}` loaded with {} package rule(s)",
                        rules_path.display(),
                        file.packages.len()
                    ),
                    None,
                ));
            }
            Ok(file) => checks.push(ReportCheck::new(
                "security-rules",
                CheckStatus::Fail,
                CATEGORY_SECURITY,
                format!(
                    "security rules file `{}` uses unsupported schema version `{}`",
                    rules_path.display(),
                    file.schema_version
                ),
                Some("update the rules file to the agent-supported schema version".to_string()),
            )),
            Err(error) => checks.push(ReportCheck::new(
                "security-rules",
                CheckStatus::Fail,
                CATEGORY_SECURITY,
                format!(
                    "failed to parse security rules file `{}`: {error}",
                    rules_path.display()
                ),
                Some("fix YAML syntax or schema mismatches in the security rules file".to_string()),
            )),
        },
        Err(error) => checks.push(ReportCheck::new(
            "security-rules",
            CheckStatus::Fail,
            CATEGORY_SECURITY,
            format!(
                "failed to read security rules file `{}`: {error}",
                rules_path.display()
            ),
            Some("check file ownership and container volume mounts for the rules file".to_string()),
        )),
    }

    checks
}

fn check_transport(config: &AgentConfig) -> ReportCheck {
    if can_read_file(Path::new(&config.edge_url)) {
        return ReportCheck::new(
            "transport",
            CheckStatus::Warn,
            CATEGORY_TRANSPORT,
            "edge_url looks like a local path, expected HTTP(S) URL",
            Some(
                "set edge_url to the public Edge API URL instead of a filesystem path".to_string(),
            ),
        );
    }

    if !config.transport.mode.is_edge() {
        return pass_check(
            "transport",
            CATEGORY_TRANSPORT,
            format!(
                "transport mode `{}` is configured for local/mock execution",
                config.transport.mode.as_str()
            ),
            None,
        );
    }

    let endpoint = match build_base_url(&config.edge_url, &config.edge_grpc_addr) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            return ReportCheck::new(
                "transport",
                CheckStatus::Fail,
                CATEGORY_TRANSPORT,
                error.to_string(),
                Some(
                    "fix edge_url / edge_grpc_addr so the agent can derive a valid gRPC endpoint"
                        .to_string(),
                ),
            );
        }
    };
    let tls_enabled = match endpoint_uses_tls(&config.edge_url, &config.edge_grpc_addr) {
        Ok(value) => value,
        Err(error) => {
            return ReportCheck::new(
                "transport",
                CheckStatus::Fail,
                CATEGORY_TRANSPORT,
                error.to_string(),
                Some(
                    "fix edge_url / edge_grpc_addr so the agent can determine the transport scheme"
                        .to_string(),
                ),
            );
        }
    };
    let server_name = match derive_server_name(
        &config.edge_url,
        &config.edge_grpc_addr,
        config.tls.server_name.as_deref(),
    ) {
        Ok(value) => value,
        Err(error) => {
            return ReportCheck::new(
                "transport",
                CheckStatus::Fail,
                CATEGORY_TRANSPORT,
                error.to_string(),
                Some("fix tls.server_name or gRPC endpoint host overrides".to_string()),
            );
        }
    };

    if !tls_enabled {
        return ReportCheck::new(
            "transport",
            CheckStatus::Warn,
            CATEGORY_TRANSPORT,
            format!(
                "edge endpoint `{endpoint}` uses plain HTTP; this is suitable only for local/dev smoke runs"
            ),
            Some("use HTTPS for deployed Linux agents".to_string()),
        );
    }

    let server_name_detail = server_name.unwrap_or_else(|| "none".to_string());
    ReportCheck::new(
        "transport",
        if server_name_detail.parse::<std::net::IpAddr>().is_ok() {
            CheckStatus::Warn
        } else {
            CheckStatus::Pass
        },
        CATEGORY_TRANSPORT,
        format!("edge endpoint `{endpoint}` uses TLS with server_name `{server_name_detail}`"),
        if server_name_detail.parse::<std::net::IpAddr>().is_ok() {
            Some("prefer a DNS server_name for stable TLS verification on Linux hosts".to_string())
        } else {
            None
        },
    )
}

fn check_transport_reachability(config: &AgentConfig) -> ReportCheck {
    if !config.transport.mode.is_edge() {
        return pass_check(
            "transport-reachability",
            CATEGORY_TRANSPORT,
            "mock transport does not require edge reachability",
            None,
        );
    }

    let endpoint = match build_base_url(&config.edge_url, &config.edge_grpc_addr) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            return ReportCheck::new(
                "transport-reachability",
                CheckStatus::Fail,
                CATEGORY_TRANSPORT,
                error.to_string(),
                Some("fix edge_url / edge_grpc_addr before validating reachability".to_string()),
            );
        }
    };

    let url = match Url::parse(&endpoint) {
        Ok(url) => url,
        Err(error) => {
            return ReportCheck::new(
                "transport-reachability",
                CheckStatus::Fail,
                CATEGORY_TRANSPORT,
                format!("invalid edge endpoint `{endpoint}`: {error}"),
                Some("fix the transport endpoint format first".to_string()),
            );
        }
    };

    let Some(host) = url.host_str() else {
        return ReportCheck::new(
            "transport-reachability",
            CheckStatus::Fail,
            CATEGORY_TRANSPORT,
            format!("edge endpoint `{endpoint}` does not include a host"),
            Some("set edge_grpc_addr to a host:port pair or full URL".to_string()),
        );
    };
    let Some(port) = url.port_or_known_default() else {
        return ReportCheck::new(
            "transport-reachability",
            CheckStatus::Fail,
            CATEGORY_TRANSPORT,
            format!("edge endpoint `{endpoint}` does not include a usable port"),
            Some("set edge_grpc_addr to an explicit port".to_string()),
        );
    };

    let target = format!("{host}:{port}");
    let socket = match target
        .to_socket_addrs()
        .ok()
        .and_then(|mut values| values.next())
    {
        Some(socket) => socket,
        None => {
            return ReportCheck::new(
                "transport-reachability",
                CheckStatus::Warn,
                CATEGORY_TRANSPORT,
                format!("could not resolve `{target}` from this host"),
                Some("DNS or /etc/hosts must resolve the edge endpoint from the agent container or Linux host".to_string()),
            );
        }
    };

    match TcpStream::connect_timeout(&socket, REACHABILITY_TIMEOUT) {
        Ok(_) => pass_check(
            "transport-reachability",
            CATEGORY_TRANSPORT,
            format!("TCP connectivity to `{target}` succeeded"),
            None,
        ),
        Err(error) => ReportCheck::new(
            "transport-reachability",
            CheckStatus::Warn,
            CATEGORY_TRANSPORT,
            format!("TCP connectivity to `{target}` failed: {error}"),
            Some("the runtime can still start in degraded mode, but edge connectivity must recover for enrollment and delivery".to_string()),
        ),
    }
}

fn check_logs_directory(context: &RuntimeMetadataContext) -> ReportCheck {
    let logs_dir = Path::new(CANONICAL_LOG_DIR);
    if !context.install.systemd_expected {
        return pass_check(
            "logs-dir",
            CATEGORY_STORAGE,
            format!(
                "canonical log dir `{}` is only a package/systemd hint for this install mode",
                logs_dir.display()
            ),
            None,
        );
    }

    if path_exists(logs_dir) {
        if !logs_dir.is_dir() {
            return ReportCheck::new(
                "logs-dir",
                CheckStatus::Fail,
                CATEGORY_STORAGE,
                format!(
                    "canonical log dir `{}` exists but is not a directory",
                    logs_dir.display()
                ),
                Some(
                    "fix the package-managed log path before relying on local journal/log files"
                        .to_string(),
                ),
            );
        }

        return ReportCheck::new(
            "logs-dir",
            if directory_write_access(logs_dir) {
                CheckStatus::Pass
            } else {
                CheckStatus::Warn
            },
            CATEGORY_STORAGE,
            if directory_write_access(logs_dir) {
                format!(
                    "canonical log dir `{}` is present for package/systemd installs",
                    logs_dir.display()
                )
            } else {
                format!(
                    "canonical log dir `{}` exists but is not writable to the current user; verify systemd `LogsDirectory=` ownership",
                    logs_dir.display()
                )
            },
            Some(
                "package installs normally rely on systemd to provision `/var/log/doro-agent`"
                    .to_string(),
            ),
        );
    }

    ReportCheck::new(
        "logs-dir",
        CheckStatus::Warn,
        CATEGORY_STORAGE,
        format!(
            "canonical log dir `{}` is missing; package/systemd installs should let systemd create it via `LogsDirectory=doro-agent`",
            logs_dir.display()
        ),
        Some("this warning is expected in non-package layouts".to_string()),
    )
}

fn check_tls(config: &AgentConfig) -> Vec<ReportCheck> {
    if !config.transport.mode.is_edge() {
        return vec![pass_check(
            "tls",
            CATEGORY_TLS,
            "mock transport does not use TLS settings",
            None,
        )];
    }

    let tls_enabled = match endpoint_uses_tls(&config.edge_url, &config.edge_grpc_addr) {
        Ok(value) => value,
        Err(error) => {
            return vec![ReportCheck::new(
                "tls",
                CheckStatus::Fail,
                CATEGORY_TLS,
                error.to_string(),
                Some("fix the endpoint scheme before validating TLS files".to_string()),
            )];
        }
    };
    let tls_configured = config.tls.ca_path.is_some()
        || config.tls.cert_path.is_some()
        || config.tls.key_path.is_some()
        || config
            .tls
            .server_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some();

    if !tls_enabled {
        if tls_configured {
            return vec![ReportCheck::new(
                "tls",
                CheckStatus::Fail,
                CATEGORY_TLS,
                "TLS settings are configured but the gRPC endpoint uses plain HTTP",
                Some("either switch the endpoint to HTTPS or remove TLS settings for local mock usage".to_string()),
            )];
        }
        return vec![ReportCheck::new(
            "tls",
            CheckStatus::Warn,
            CATEGORY_TLS,
            "plaintext HTTP transport configured; no TLS files are required in dev mode",
            Some(
                "deployed Linux agents should use HTTPS and, when required, mTLS material"
                    .to_string(),
            ),
        )];
    }

    vec![
        check_ca_bundle(config),
        check_client_identity(config),
        check_transport_client(config),
    ]
}

fn check_ca_bundle(config: &AgentConfig) -> ReportCheck {
    let Some(path) = config.tls.ca_path.as_deref() else {
        return ReportCheck::new(
            "tls-ca",
            CheckStatus::Warn,
            CATEGORY_TLS,
            "tls.ca_path is not configured; the system trust store will be used",
            Some("pin tls.ca_path when the edge uses a private CA".to_string()),
        );
    };

    match load_ca_certificate(path) {
        Ok(_) => pass_check(
            "tls-ca",
            CATEGORY_TLS,
            format!(
                "TLS CA bundle `{}` is readable and parseable",
                path.display()
            ),
            None,
        ),
        Err(error) => ReportCheck::new(
            "tls-ca",
            CheckStatus::Fail,
            CATEGORY_TLS,
            error.to_string(),
            Some(
                "install a valid PEM CA bundle that matches the edge server certificate chain"
                    .to_string(),
            ),
        ),
    }
}

fn check_client_identity(config: &AgentConfig) -> ReportCheck {
    let (Some(cert_path), Some(key_path)) = (
        config.tls.cert_path.as_deref(),
        config.tls.key_path.as_deref(),
    ) else {
        return ReportCheck::new(
            "tls-identity",
            CheckStatus::Warn,
            CATEGORY_TLS,
            "client mTLS certificate/key are not configured; the endpoint must allow server-auth-only TLS",
            Some("configure tls.cert_path and tls.key_path when the edge requires mutual TLS".to_string()),
        );
    };

    match load_client_identity(cert_path, key_path) {
        Ok(_) => pass_check(
            "tls-identity",
            CATEGORY_TLS,
            format!(
                "client mTLS certificate `{}` and key `{}` are parseable",
                cert_path.display(),
                key_path.display()
            ),
            None,
        ),
        Err(error) => ReportCheck::new(
            "tls-identity",
            CheckStatus::Fail,
            CATEGORY_TLS,
            error.to_string(),
            Some(
                "install a matching PEM certificate and private key for client authentication"
                    .to_string(),
            ),
        ),
    }
}

fn check_transport_client(config: &AgentConfig) -> ReportCheck {
    match EdgeGrpcTransport::new(&config.edge_url, &config.edge_grpc_addr, &config.tls) {
        Ok(_) => pass_check(
            "transport-client",
            CATEGORY_TLS,
            "edge gRPC client configuration is internally consistent",
            None,
        ),
        Err(error) => ReportCheck::new(
            "transport-client",
            CheckStatus::Fail,
            CATEGORY_TLS,
            error.to_string(),
            Some("fix TLS file paths, scheme mismatches, or server_name overrides".to_string()),
        ),
    }
}

fn check_sources(config: &AgentConfig) -> Vec<ReportCheck> {
    if config.transport.mode.is_edge() && config.sources.is_empty() {
        return vec![pass_check(
            "source-path",
            CATEGORY_SOURCES,
            "edge mode uses server policy; local sources will be known after policy fetch",
            Some("use local mock mode when you want preflight to validate static source paths"),
        )];
    }

    detect_source_paths(config)
        .into_iter()
        .map(|source| match source.status {
            SourcePathStatus::Readable => pass_check(
                "source-path",
                CATEGORY_SOURCES,
                format!("{} is readable", source.path.display()),
                None,
            ),
            SourcePathStatus::Missing => ReportCheck::new(
                "source-path",
                CheckStatus::Warn,
                CATEGORY_SOURCES,
                format!(
                    "{} is missing; file source will start in waiting mode",
                    source.path.display()
                ),
                Some("create the file or verify policy/source path correctness".to_string()),
            ),
            SourcePathStatus::Unreadable => ReportCheck::new(
                "source-path",
                CheckStatus::Fail,
                CATEGORY_SOURCES,
                format!(
                    "{} is unreadable: {}",
                    source.path.display(),
                    source
                        .message
                        .unwrap_or_else(|| "permission denied".to_string())
                ),
                Some("fix file ownership, ACLs, or container volume mounts".to_string()),
            ),
        })
        .collect()
}

fn check_compatibility(context: &RuntimeMetadataContext) -> Vec<ReportCheck> {
    let mut checks = Vec::new();

    for issue in &context.compatibility.permission_issues {
        checks.push(ReportCheck::new(
            "permissions",
            CheckStatus::Fail,
            CATEGORY_COMPATIBILITY,
            issue,
            Some(
                "run the agent with a user that can access state, spool, and source paths"
                    .to_string(),
            ),
        ));
    }
    for issue in &context.compatibility.errors {
        checks.push(ReportCheck::new(
            "compatibility",
            CheckStatus::Fail,
            CATEGORY_COMPATIBILITY,
            issue,
            Some("resolve the runtime compatibility error before starting the agent".to_string()),
        ));
    }
    for issue in &context.compatibility.warnings {
        checks.push(ReportCheck::new(
            "compatibility",
            CheckStatus::Warn,
            CATEGORY_COMPATIBILITY,
            issue,
            Some(
                "review this warning if the current layout is not an intentional local/dev setup"
                    .to_string(),
            ),
        ));
    }

    if checks.is_empty() {
        checks.push(pass_check(
            "compatibility",
            CATEGORY_COMPATIBILITY,
            "no compatibility issues were detected from local metadata",
            None,
        ));
    }

    checks
}

fn detect_container_runtime() -> Option<String> {
    if path_exists(Path::new("/run/.containerenv")) {
        return Some("podman".to_string());
    }
    if path_exists(Path::new("/.dockerenv")) {
        return Some("docker".to_string());
    }
    if std::env::var_os("KUBERNETES_SERVICE_HOST").is_some() {
        return Some("kubernetes".to_string());
    }
    None
}

fn iso_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
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

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use crate::{ops::CheckStatus, test_support::lock_agent_env};

    use super::run;

    fn write_config(dir: &TempDir) -> PathBuf {
        let config_path = dir.path().join("agent.yaml");
        fs::write(
            &config_path,
            format!(
                r#"
edge_url: 'https://edge.example.local'
edge_grpc_addr: 'edge.example.local:7443'
bootstrap_token: 'token'
state_dir: '{}'
transport:
  mode: 'mock'
install:
  mode: 'dev'
sources:
  - type: 'file'
    path: '{}'
    source: 'demo'
    service: 'svc'
    severity_hint: 'info'
"#,
                dir.path().join("state").display(),
                dir.path().join("demo.log").display()
            ),
        )
        .unwrap();
        config_path
    }

    #[test]
    fn doctor_warns_for_missing_state_db_and_source() {
        let _guard = lock_agent_env();
        let dir = TempDir::new().unwrap();
        let config_path = write_config(&dir);

        let report = run(&config_path);
        assert_eq!(report.report_kind, "doctor");
        assert!(report.summary.warning_count > 0);
        assert!(report
            .checks
            .iter()
            .any(|check| check.category == "sources" && check.status == CheckStatus::Warn));
    }

    #[test]
    fn doctor_passes_when_state_db_exists() {
        let _guard = lock_agent_env();
        let dir = TempDir::new().unwrap();
        let config_path = write_config(&dir);
        let state_dir = dir.path().join("state");
        fs::create_dir_all(&state_dir).unwrap();
        let state_db = state_dir.join("state.db");
        rusqlite::Connection::open(&state_db).unwrap();
        fs::write(dir.path().join("demo.log"), "hello\n").unwrap();

        let report = run(&config_path);
        assert!(report
            .checks
            .iter()
            .any(|check| check.name == "state-db" && check.status == CheckStatus::Pass));
    }

    #[test]
    fn doctor_fails_for_invalid_tls_ca_bundle() {
        let _guard = lock_agent_env();
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("agent.yaml");
        let ca_path = dir.path().join("ca.pem");
        fs::write(&ca_path, "not-a-pem").unwrap();
        fs::write(
            &config_path,
            format!(
                r#"
edge_url: 'https://edge.example.local'
edge_grpc_addr: 'edge.example.local:7443'
bootstrap_token: 'token'
state_dir: '{}'
transport:
  mode: 'edge'
tls:
  ca_path: '{}'
"#,
                dir.path().join("state").display(),
                ca_path.display()
            ),
        )
        .unwrap();

        let report = run(&config_path);
        assert!(report
            .checks
            .iter()
            .any(|check| check.name == "tls-ca" && check.status == CheckStatus::Fail));
        assert!(report.has_failures());
    }

    #[test]
    fn doctor_json_schema_contains_summary_and_categories() {
        let _guard = lock_agent_env();
        let dir = TempDir::new().unwrap();
        let config_path = write_config(&dir);

        let report = run(&config_path);
        let value = serde_json::to_value(&report).unwrap();

        assert_eq!(value["report_kind"], "doctor");
        assert!(value["generated_at"].is_string());
        assert!(value["summary"]["check_count"].is_number());
        assert!(value["checks"][0]["category"].is_string());
    }
}
