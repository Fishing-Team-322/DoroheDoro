use common::AppResult;

use crate::models::{
    default_source_id, default_source_name, BatchConfigYaml, DegradedConfigYaml,
    DeploymentBootstrapConfig, FileSourceConfig, IntervalConfigYaml, QueueConfigYaml,
    SpoolConfigYaml, TlsConfigYaml, TransportConfigYaml,
};

pub fn bootstrap_file_name() -> &'static str {
    "config.yaml"
}

pub fn build_bootstrap_config(
    edge_public_url: &str,
    edge_grpc_addr: &str,
    bootstrap_token: &str,
    state_dir: &str,
    log_level: &str,
    source_paths: &[String],
    tls: Option<TlsConfigYaml>,
) -> DeploymentBootstrapConfig {
    DeploymentBootstrapConfig {
        edge_url: edge_public_url.to_string(),
        edge_grpc_addr: edge_grpc_addr.to_string(),
        bootstrap_token: bootstrap_token.to_string(),
        state_dir: state_dir.to_string(),
        log_level: log_level.to_string(),
        transport: TransportConfigYaml {
            mode: "edge".to_string(),
        },
        heartbeat: IntervalConfigYaml { interval_sec: 30 },
        diagnostics: IntervalConfigYaml { interval_sec: 30 },
        batch: BatchConfigYaml {
            max_events: 500,
            max_bytes: 524_288,
            flush_interval_ms: 2_000,
            compress_threshold_bytes: 16_384,
        },
        queues: QueueConfigYaml {
            event_capacity: 4_096,
            send_capacity: 32,
            event_bytes_soft_limit: 8 * 1024 * 1024,
            send_bytes_soft_limit: 16 * 1024 * 1024,
        },
        degraded: DegradedConfigYaml {
            failure_threshold: 3,
            server_unavailable_sec: 30,
            queue_pressure_pct: 80,
            queue_recover_pct: 40,
            unacked_lag_bytes: 16 * 1024 * 1024,
            shutdown_spool_grace_sec: 5,
        },
        spool: SpoolConfigYaml {
            enabled: true,
            dir: format!("{state_dir}/spool"),
            max_disk_bytes: 268_435_456,
        },
        tls,
        sources: source_paths
            .iter()
            .map(|path| FileSourceConfig {
                kind: "file".to_string(),
                source_id: default_source_id(path),
                path: path.clone(),
                start_at: "end".to_string(),
                source: default_source_name(path),
                service: "host".to_string(),
                severity_hint: "info".to_string(),
            })
            .collect(),
    }
}

pub fn render_bootstrap_yaml(config: &DeploymentBootstrapConfig) -> AppResult<String> {
    serde_yaml::to_string(config)
        .map_err(|error| common::AppError::internal(format!("render bootstrap yaml: {error}")))
}

#[cfg(test)]
mod tests {
    use crate::models::TlsConfigYaml;

    use super::{build_bootstrap_config, render_bootstrap_yaml};

    #[test]
    fn renders_bootstrap_yaml() {
        let config = build_bootstrap_config(
            "https://edge.example.local",
            "edge.example.local:9090",
            "token-1",
            "/var/lib/doro-agent",
            "info",
            &[String::from("/var/log/syslog")],
            Some(TlsConfigYaml {
                ca_path: Some("/etc/doro-agent/pki/ca.pem".to_string()),
                cert_path: Some("/etc/doro-agent/pki/agent.pem".to_string()),
                key_path: Some("/etc/doro-agent/pki/agent.key".to_string()),
                server_name: Some("edge.example.local".to_string()),
            }),
        );
        let yaml = render_bootstrap_yaml(&config).unwrap();
        assert!(yaml.contains("edge_grpc_addr: edge.example.local:9090"));
        assert!(yaml.contains("bootstrap_token: token-1"));
        assert!(yaml.contains("path: /var/log/syslog"));
        assert!(yaml.contains("server_name: edge.example.local"));
    }
}
