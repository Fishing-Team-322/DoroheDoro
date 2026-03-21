use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::{
    config::AgentConfig,
    error::AppResult,
    metadata::{
        can_read_file, detect_source_paths, directory_write_access, path_exists,
        RuntimeMetadataContext, SourcePathStatus, CANONICAL_LOG_DIR,
    },
    transport::client::{
        build_base_url, derive_server_name, endpoint_uses_tls, load_ca_certificate,
        load_client_identity, EdgeGrpcTransport,
    },
};

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub config_path: PathBuf,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone)]
pub struct DoctorCheck {
    pub status: DoctorStatus,
    pub name: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

impl DoctorReport {
    pub fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == DoctorStatus::Fail)
    }

    pub fn warning_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.status == DoctorStatus::Warn)
            .count()
    }

    pub fn print(&self) {
        println!("doro-agent doctor");
        println!("config: {}", self.config_path.display());
        for check in &self.checks {
            println!(
                "[{}] {}: {}",
                check.status.label(),
                check.name,
                check.detail
            );
        }
        println!(
            "summary: {} checks, {} warning(s), {} failure(s)",
            self.checks.len(),
            self.warning_count(),
            self.checks
                .iter()
                .filter(|check| check.status == DoctorStatus::Fail)
                .count()
        );
    }
}

impl DoctorStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

pub fn run(config_path: &Path) -> AppResult<DoctorReport> {
    let config = AgentConfig::load(config_path)?;
    let hostname = crate::app::resolve_hostname();
    let context = RuntimeMetadataContext::detect(&config, config_path, &hostname)?;
    let mut checks = Vec::new();

    checks.push(DoctorCheck {
        status: DoctorStatus::Pass,
        name: "config".to_string(),
        detail: "configuration parsed successfully".to_string(),
    });
    checks.push(DoctorCheck {
        status: if context.install.resolved_mode == "unknown"
            || !context.install.warnings.is_empty()
        {
            DoctorStatus::Warn
        } else {
            DoctorStatus::Pass
        },
        name: "install-mode".to_string(),
        detail: format!(
            "configured={}, resolved={}, source={}",
            context.install.configured_mode,
            context.install.resolved_mode,
            context.install.resolution_source
        ),
    });
    checks.push(DoctorCheck {
        status: DoctorStatus::Pass,
        name: "build".to_string(),
        detail: format!(
            "version={}, build_id={}, target={}, profile={}",
            context.build.agent_version,
            context.build.build_id,
            context.build.target_triple,
            context.build.build_profile
        ),
    });
    checks.push(DoctorCheck {
        status: if context.platform.systemd_detected || !context.install.systemd_expected {
            DoctorStatus::Pass
        } else {
            DoctorStatus::Warn
        },
        name: "systemd".to_string(),
        detail: format!(
            "detected={}, expected={}, service_manager={}",
            context.platform.systemd_detected,
            context.install.systemd_expected,
            context.platform.service_manager
        ),
    });

    checks.push(check_directory(
        "state-dir",
        &config.state_dir,
        "runtime state",
        true,
    ));
    if config.spool.enabled {
        checks.push(check_directory(
            "spool-dir",
            &config.spool.dir,
            "fallback spool",
            true,
        ));
    } else {
        checks.push(DoctorCheck {
            status: DoctorStatus::Pass,
            name: "spool-dir".to_string(),
            detail: "spool is disabled".to_string(),
        });
    }

    checks.push(check_state_db(&config.state_dir.join("state.db")));
    checks.push(check_transport(&config));
    checks.push(check_logs_directory(&context));
    checks.extend(check_tls(&config));

    for source in detect_source_paths(&config) {
        checks.push(match source.status {
            SourcePathStatus::Readable => DoctorCheck {
                status: DoctorStatus::Pass,
                name: "source".to_string(),
                detail: format!("{} is readable", source.path.display()),
            },
            SourcePathStatus::Missing => DoctorCheck {
                status: DoctorStatus::Warn,
                name: "source".to_string(),
                detail: format!(
                    "{} is missing; file source will start in waiting mode",
                    source.path.display()
                ),
            },
            SourcePathStatus::Unreadable => DoctorCheck {
                status: DoctorStatus::Fail,
                name: "source".to_string(),
                detail: format!(
                    "{} is unreadable: {}",
                    source.path.display(),
                    source
                        .message
                        .unwrap_or_else(|| "permission denied".to_string())
                ),
            },
        });
    }

    for issue in &context.compatibility.permission_issues {
        checks.push(DoctorCheck {
            status: DoctorStatus::Fail,
            name: "permissions".to_string(),
            detail: issue.clone(),
        });
    }
    for issue in &context.compatibility.warnings {
        checks.push(DoctorCheck {
            status: DoctorStatus::Warn,
            name: "compatibility".to_string(),
            detail: issue.clone(),
        });
    }

    if context.cluster.configured_cluster_id.is_some()
        || !context.cluster.effective_cluster_tags.is_empty()
        || !context.cluster.host_labels.is_empty()
    {
        checks.push(DoctorCheck {
            status: DoctorStatus::Pass,
            name: "cluster-scope".to_string(),
            detail: format!(
                "cluster_id={:?}, effective_cluster_tags={}, host_labels={}",
                context.cluster.configured_cluster_id,
                context.cluster.effective_cluster_tags.len(),
                context.cluster.host_labels.len()
            ),
        });
    }

    Ok(DoctorReport {
        config_path: config_path.to_path_buf(),
        checks,
    })
}

fn check_directory(name: &str, path: &Path, label: &str, require_write: bool) -> DoctorCheck {
    if path_exists(path) {
        if !path.is_dir() {
            return DoctorCheck {
                status: DoctorStatus::Fail,
                name: name.to_string(),
                detail: format!(
                    "{} path `{}` exists but is not a directory",
                    label,
                    path.display()
                ),
            };
        }

        if require_write && !directory_write_access(path) {
            return DoctorCheck {
                status: DoctorStatus::Fail,
                name: name.to_string(),
                detail: format!("{} path `{}` is not writable", label, path.display()),
            };
        }

        return DoctorCheck {
            status: DoctorStatus::Pass,
            name: name.to_string(),
            detail: format!("{} path `{}` is available", label, path.display()),
        };
    }

    let parent = path.parent().unwrap_or(path);
    if directory_write_access(parent) {
        DoctorCheck {
            status: DoctorStatus::Warn,
            name: name.to_string(),
            detail: format!(
                "{} path `{}` does not exist yet but parent `{}` is writable",
                label,
                path.display(),
                parent.display()
            ),
        }
    } else {
        DoctorCheck {
            status: DoctorStatus::Fail,
            name: name.to_string(),
            detail: format!(
                "{} path `{}` does not exist and parent `{}` is not writable",
                label,
                path.display(),
                parent.display()
            ),
        }
    }
}

fn check_state_db(state_db_path: &Path) -> DoctorCheck {
    if !path_exists(state_db_path) {
        let parent = state_db_path.parent().unwrap_or(state_db_path);
        return if directory_write_access(parent) {
            DoctorCheck {
                status: DoctorStatus::Warn,
                name: "state-db".to_string(),
                detail: format!(
                    "state database `{}` does not exist yet; runtime should create it",
                    state_db_path.display()
                ),
            }
        } else {
            DoctorCheck {
                status: DoctorStatus::Fail,
                name: "state-db".to_string(),
                detail: format!(
                    "state database `{}` is missing and parent `{}` is not writable",
                    state_db_path.display(),
                    parent.display()
                ),
            }
        };
    }

    match Connection::open_with_flags(state_db_path, OpenFlags::SQLITE_OPEN_READ_WRITE) {
        Ok(_) => DoctorCheck {
            status: DoctorStatus::Pass,
            name: "state-db".to_string(),
            detail: format!("state database `{}` is accessible", state_db_path.display()),
        },
        Err(error) => DoctorCheck {
            status: DoctorStatus::Fail,
            name: "state-db".to_string(),
            detail: format!(
                "state database `{}` is not accessible: {error}",
                state_db_path.display()
            ),
        },
    }
}

fn check_transport(config: &AgentConfig) -> DoctorCheck {
    if can_read_file(Path::new(&config.edge_url)) {
        return DoctorCheck {
            status: DoctorStatus::Warn,
            name: "transport".to_string(),
            detail: "edge_url looks like a local path, expected HTTP(S) URL".to_string(),
        };
    }

    if !config.transport.mode.is_edge() {
        return DoctorCheck {
            status: DoctorStatus::Pass,
            name: "transport".to_string(),
            detail: format!(
                "transport mode `{}` is configured for local/mock execution",
                config.transport.mode.as_str()
            ),
        };
    }

    let endpoint = match build_base_url(&config.edge_url, &config.edge_grpc_addr) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            return DoctorCheck {
                status: DoctorStatus::Fail,
                name: "transport".to_string(),
                detail: error.to_string(),
            };
        }
    };
    let tls_enabled = match endpoint_uses_tls(&config.edge_url, &config.edge_grpc_addr) {
        Ok(value) => value,
        Err(error) => {
            return DoctorCheck {
                status: DoctorStatus::Fail,
                name: "transport".to_string(),
                detail: error.to_string(),
            };
        }
    };
    let server_name = match derive_server_name(
        &config.edge_url,
        &config.edge_grpc_addr,
        config.tls.server_name.as_deref(),
    ) {
        Ok(value) => value,
        Err(error) => {
            return DoctorCheck {
                status: DoctorStatus::Fail,
                name: "transport".to_string(),
                detail: error.to_string(),
            };
        }
    };

    if !tls_enabled {
        return DoctorCheck {
            status: DoctorStatus::Warn,
            name: "transport".to_string(),
            detail: format!(
                "edge endpoint `{endpoint}` uses plain HTTP; this is suitable only for local/dev smoke runs"
            ),
        };
    }

    let server_name_detail = server_name.unwrap_or_else(|| "none".to_string());
    let server_name_status = if server_name_detail.parse::<std::net::IpAddr>().is_ok() {
        DoctorStatus::Warn
    } else {
        DoctorStatus::Pass
    };

    DoctorCheck {
        status: server_name_status,
        name: "transport".to_string(),
        detail: format!(
            "edge endpoint `{endpoint}` uses TLS with server_name `{server_name_detail}`"
        ),
    }
}

fn check_logs_directory(context: &RuntimeMetadataContext) -> DoctorCheck {
    let logs_dir = Path::new(CANONICAL_LOG_DIR);
    if !context.install.systemd_expected {
        return DoctorCheck {
            status: DoctorStatus::Pass,
            name: "logs-dir".to_string(),
            detail: format!(
                "canonical log dir `{}` is only a package/systemd hint for this install mode",
                logs_dir.display()
            ),
        };
    }

    if path_exists(logs_dir) {
        if !logs_dir.is_dir() {
            return DoctorCheck {
                status: DoctorStatus::Fail,
                name: "logs-dir".to_string(),
                detail: format!(
                    "canonical log dir `{}` exists but is not a directory",
                    logs_dir.display()
                ),
            };
        }

        return DoctorCheck {
            status: if directory_write_access(logs_dir) {
                DoctorStatus::Pass
            } else {
                DoctorStatus::Warn
            },
            name: "logs-dir".to_string(),
            detail: if directory_write_access(logs_dir) {
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
        };
    }

    DoctorCheck {
        status: DoctorStatus::Warn,
        name: "logs-dir".to_string(),
        detail: format!(
            "canonical log dir `{}` is missing; package/systemd installs should let systemd create it via `LogsDirectory=doro-agent`",
            logs_dir.display()
        ),
    }
}

fn check_tls(config: &AgentConfig) -> Vec<DoctorCheck> {
    if !config.transport.mode.is_edge() {
        return vec![DoctorCheck {
            status: DoctorStatus::Pass,
            name: "tls".to_string(),
            detail: "mock transport does not use TLS settings".to_string(),
        }];
    }

    let tls_enabled = match endpoint_uses_tls(&config.edge_url, &config.edge_grpc_addr) {
        Ok(value) => value,
        Err(error) => {
            return vec![DoctorCheck {
                status: DoctorStatus::Fail,
                name: "tls".to_string(),
                detail: error.to_string(),
            }];
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
            return vec![DoctorCheck {
                status: DoctorStatus::Fail,
                name: "tls".to_string(),
                detail: "TLS settings are configured but the gRPC endpoint uses plain HTTP"
                    .to_string(),
            }];
        }
        return vec![DoctorCheck {
            status: DoctorStatus::Warn,
            name: "tls".to_string(),
            detail: "plaintext HTTP transport configured; no TLS files are required in dev mode"
                .to_string(),
        }];
    }

    let mut checks = Vec::new();
    checks.push(check_ca_bundle(config));
    checks.push(check_client_identity(config));
    checks.push(check_transport_client(config));
    checks
}

fn check_ca_bundle(config: &AgentConfig) -> DoctorCheck {
    let Some(path) = config.tls.ca_path.as_deref() else {
        return DoctorCheck {
            status: DoctorStatus::Warn,
            name: "tls-ca".to_string(),
            detail: "tls.ca_path is not configured; the system trust store will be used"
                .to_string(),
        };
    };

    match load_ca_certificate(path) {
        Ok(_) => DoctorCheck {
            status: DoctorStatus::Pass,
            name: "tls-ca".to_string(),
            detail: format!(
                "TLS CA bundle `{}` is readable and parseable",
                path.display()
            ),
        },
        Err(error) => DoctorCheck {
            status: DoctorStatus::Fail,
            name: "tls-ca".to_string(),
            detail: error.to_string(),
        },
    }
}

fn check_client_identity(config: &AgentConfig) -> DoctorCheck {
    let (Some(cert_path), Some(key_path)) = (
        config.tls.cert_path.as_deref(),
        config.tls.key_path.as_deref(),
    ) else {
        return DoctorCheck {
            status: DoctorStatus::Warn,
            name: "tls-identity".to_string(),
            detail: "client mTLS certificate/key are not configured; the endpoint must allow server-auth-only TLS".to_string(),
        };
    };

    match load_client_identity(cert_path, key_path) {
        Ok(_) => DoctorCheck {
            status: DoctorStatus::Pass,
            name: "tls-identity".to_string(),
            detail: format!(
                "client mTLS certificate `{}` and key `{}` are parseable",
                cert_path.display(),
                key_path.display()
            ),
        },
        Err(error) => DoctorCheck {
            status: DoctorStatus::Fail,
            name: "tls-identity".to_string(),
            detail: error.to_string(),
        },
    }
}

fn check_transport_client(config: &AgentConfig) -> DoctorCheck {
    match EdgeGrpcTransport::new(&config.edge_url, &config.edge_grpc_addr, &config.tls) {
        Ok(_) => DoctorCheck {
            status: DoctorStatus::Pass,
            name: "transport-client".to_string(),
            detail: "edge gRPC client configuration is internally consistent".to_string(),
        },
        Err(error) => DoctorCheck {
            status: DoctorStatus::Fail,
            name: "transport-client".to_string(),
            detail: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use crate::test_support::lock_agent_env;

    use super::{run, DoctorStatus};

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

        let report = run(&config_path).unwrap();
        assert!(!report.checks.is_empty());
        assert!(report
            .checks
            .iter()
            .any(|check| check.status == DoctorStatus::Warn));
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

        let report = run(&config_path).unwrap();
        assert!(report
            .checks
            .iter()
            .any(|check| check.name == "state-db" && check.status == DoctorStatus::Pass));
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

        let report = run(&config_path).unwrap();
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "tls-ca" && check.status == DoctorStatus::Fail),
            "{:#?}",
            report.checks
        );
    }

    #[test]
    fn doctor_fails_when_tls_is_configured_for_plain_http() {
        let _guard = lock_agent_env();
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("agent.yaml");
        fs::write(
            &config_path,
            format!(
                r#"
edge_url: 'http://edge.example.local:8080'
edge_grpc_addr: 'edge.example.local:9090'
bootstrap_token: 'token'
state_dir: '{}'
transport:
  mode: 'edge'
tls:
  server_name: 'edge.example.local'
"#,
                dir.path().join("state").display(),
            ),
        )
        .unwrap();

        let report = run(&config_path).unwrap();
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "tls" && check.status == DoctorStatus::Fail),
            "{:#?}",
            report.checks
        );
    }
}
