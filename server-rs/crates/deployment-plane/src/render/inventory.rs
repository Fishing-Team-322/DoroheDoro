use serde_json::{json, Value};

use crate::models::{
    DeploymentBootstrapConfig, DeploymentJobType, DeploymentTargetSnapshot, DeploymentTargetStatus,
    ResolvedArtifact, ResolvedHost,
};

pub fn render_inventory_ini(target: &DeploymentTargetSnapshot) -> String {
    format!(
        "[targets]\n{} ansible_host={} ansible_port={} ansible_user={}\n",
        target.host.hostname, target.host.ip, target.host.ssh_port, target.host.remote_user
    )
}

pub fn render_target_vars(
    host: &ResolvedHost,
    job_type: DeploymentJobType,
    state_dir: &str,
    bootstrap: &DeploymentBootstrapConfig,
    artifact: &ResolvedArtifact,
) -> Value {
    let install_mode = resolved_install_mode(job_type, artifact);
    let (image_repository, image_tag, image_digest, image_reference, image_digest_reference) =
        derive_container_metadata(artifact);

    json!({
        "host_id": host.host_id,
        "hostname": host.hostname,
        "ip": host.ip,
        "ssh_port": host.ssh_port,
        "remote_user": host.remote_user,
        "doro_agent_deployment_job_type": job_type.as_str(),
        "doro_agent_state": if matches!(job_type, DeploymentJobType::Uninstall) { "absent" } else { "present" },
        "doro_agent_install_mode": install_mode,
        "doro_agent_selected_artifact": artifact,
        "doro_agent_image_repository": image_repository,
        "doro_agent_image_tag": image_tag,
        "doro_agent_image_digest": image_digest,
        "doro_agent_image_reference": image_reference,
        "doro_agent_image_digest_reference": image_digest_reference,
        "doro_agent_edge_url": bootstrap.edge_url,
        "doro_agent_edge_grpc_addr": bootstrap.edge_grpc_addr,
        "doro_agent_bootstrap_token": bootstrap.bootstrap_token,
        "doro_agent_state_dir": state_dir,
        "doro_agent_log_level": bootstrap.log_level,
        "doro_agent_transport_mode": bootstrap.transport.mode,
        "doro_agent_heartbeat_interval_sec": bootstrap.heartbeat.interval_sec,
        "doro_agent_diagnostics_interval_sec": bootstrap.diagnostics.interval_sec,
        "doro_agent_batch_max_events": bootstrap.batch.max_events,
        "doro_agent_batch_max_bytes": bootstrap.batch.max_bytes,
        "doro_agent_batch_flush_interval_ms": bootstrap.batch.flush_interval_ms,
        "doro_agent_batch_compress_threshold_bytes": bootstrap.batch.compress_threshold_bytes,
        "doro_agent_queue_event_capacity": bootstrap.queues.event_capacity,
        "doro_agent_queue_send_capacity": bootstrap.queues.send_capacity,
        "doro_agent_queue_event_bytes_soft_limit": bootstrap.queues.event_bytes_soft_limit,
        "doro_agent_queue_send_bytes_soft_limit": bootstrap.queues.send_bytes_soft_limit,
        "doro_agent_degraded_failure_threshold": bootstrap.degraded.failure_threshold,
        "doro_agent_degraded_server_unavailable_sec": bootstrap.degraded.server_unavailable_sec,
        "doro_agent_degraded_queue_pressure_pct": bootstrap.degraded.queue_pressure_pct,
        "doro_agent_degraded_queue_recover_pct": bootstrap.degraded.queue_recover_pct,
        "doro_agent_degraded_unacked_lag_bytes": bootstrap.degraded.unacked_lag_bytes,
        "doro_agent_degraded_shutdown_spool_grace_sec": bootstrap.degraded.shutdown_spool_grace_sec,
        "doro_agent_spool_enabled": bootstrap.spool.enabled,
        "doro_agent_spool_dir": bootstrap.spool.dir,
        "doro_agent_spool_max_disk_bytes": bootstrap.spool.max_disk_bytes,
        "doro_agent_sources": bootstrap.sources,
        "doro_agent_tls_ca_path": bootstrap.tls.as_ref().and_then(|tls| tls.ca_path.clone()),
        "doro_agent_tls_cert_path": bootstrap.tls.as_ref().and_then(|tls| tls.cert_path.clone()),
        "doro_agent_tls_key_path": bootstrap.tls.as_ref().and_then(|tls| tls.key_path.clone()),
        "doro_agent_tls_server_name": bootstrap.tls.as_ref().and_then(|tls| tls.server_name.clone()),
    })
}

pub fn aggregate_job_status(
    succeeded: usize,
    failed: usize,
    cancelled: usize,
    total: usize,
) -> DeploymentTargetStatus {
    if cancelled == total && total > 0 {
        DeploymentTargetStatus::Cancelled
    } else if failed > 0 {
        DeploymentTargetStatus::Failed
    } else if succeeded == total {
        DeploymentTargetStatus::Succeeded
    } else {
        DeploymentTargetStatus::Running
    }
}

fn resolved_install_mode(job_type: DeploymentJobType, artifact: &ResolvedArtifact) -> String {
    if matches!(job_type, DeploymentJobType::Uninstall) {
        return "absent".to_string();
    }
    match artifact.install_mode.as_str() {
        "docker_image" => "docker_image".to_string(),
        "package" => "deb".to_string(),
        "tarball" => "tar.gz".to_string(),
        other => match artifact.package_type.as_str() {
            "container" => "docker_image".to_string(),
            "deb" => "deb".to_string(),
            "tar.gz" => "tar.gz".to_string(),
            _ => other.to_string(),
        },
    }
}

fn derive_container_metadata(
    artifact: &ResolvedArtifact,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    if artifact.install_mode != "docker_image" && artifact.package_type != "container" {
        return (None, None, None, None, None);
    }

    let image_reference = artifact
        .image_reference
        .clone()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            if artifact.source_uri.is_empty() {
                None
            } else {
                Some(artifact.source_uri.clone())
            }
        });
    let image_digest_reference = artifact
        .image_digest_reference
        .clone()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            artifact
                .image_digest
                .as_ref()
                .filter(|value| !value.is_empty())
                .and_then(|digest| {
                    image_reference
                        .as_ref()
                        .map(|reference| format!("{reference}@{digest}"))
                })
        });

    let repository = artifact
        .image_repository
        .clone()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            image_reference
                .as_ref()
                .map(|value| split_repository(value).0)
        });
    let tag = artifact
        .image_tag
        .clone()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            image_reference
                .as_ref()
                .and_then(|value| split_repository(value).1)
        });
    let digest = artifact
        .image_digest
        .clone()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            image_digest_reference.as_ref().and_then(|reference| {
                reference
                    .split_once('@')
                    .map(|(_, digest)| digest.to_string())
            })
        });

    (
        repository,
        tag,
        digest,
        image_reference,
        image_digest_reference,
    )
}

fn split_repository(reference: &str) -> (String, Option<String>) {
    let (without_digest, _) = reference
        .split_once('@')
        .map(|(repo, _)| (repo, true))
        .unwrap_or((reference, false));
    let last_slash = without_digest.rfind('/');
    let last_colon = without_digest.rfind(':');
    if let Some(colon_idx) = last_colon {
        if last_slash.map(|idx| colon_idx > idx).unwrap_or(true) {
            let repo = without_digest[..colon_idx].to_string();
            let tag = without_digest[colon_idx + 1..].to_string();
            return (repo, Some(tag));
        }
    }
    (without_digest.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            DeploymentJobType, DeploymentTargetStatus, ResolvedArtifact, ResolvedHost,
            TlsConfigYaml,
        },
        render::bootstrap::build_bootstrap_config,
    };
    use uuid::Uuid;

    fn sample_bootstrap() -> DeploymentBootstrapConfig {
        build_bootstrap_config(
            "https://edge.example.local",
            "edge.example.local:9090",
            "token",
            "/var/lib/doro-agent",
            "info",
            &[String::from("/var/log/syslog")],
            Some(TlsConfigYaml {
                ca_path: Some("/etc/doro-agent/pki/ca.pem".to_string()),
                cert_path: Some("/etc/doro-agent/pki/agent.pem".to_string()),
                key_path: Some("/etc/doro-agent/pki/agent.key".to_string()),
                server_name: Some("edge.example.local".to_string()),
            }),
        )
    }

    fn sample_host() -> ResolvedHost {
        ResolvedHost {
            host_id: Uuid::new_v4(),
            hostname: "demo-host".to_string(),
            ip: "10.0.0.10".to_string(),
            ssh_port: 22,
            remote_user: "root".to_string(),
            labels: Default::default(),
        }
    }

    #[test]
    fn render_sets_container_metadata() {
        let artifact = ResolvedArtifact {
            version: "0.2.0".to_string(),
            platform: "linux".to_string(),
            arch: "amd64".to_string(),
            package_type: "container".to_string(),
            distro_family: "generic-glibc".to_string(),
            install_mode: "docker_image".to_string(),
            artifact_name: "doro-agent_0.2.0_linux_amd64.image-ref".to_string(),
            artifact_path: "docker.io/example/doro-agent:0.2.0".to_string(),
            source_uri: "docker.io/example/doro-agent:0.2.0".to_string(),
            checksum_file: "sha256:aaaa".to_string(),
            sha256: "aaaa".to_string(),
            bundle_root: None,
            image_repository: Some("docker.io/example/doro-agent".to_string()),
            image_tag: Some("0.2.0".to_string()),
            image_digest: Some("sha256:bbbb".to_string()),
            image_reference: Some("docker.io/example/doro-agent:0.2.0".to_string()),
            image_digest_reference: Some("docker.io/example/doro-agent@sha256:bbbb".to_string()),
        };
        let vars = render_target_vars(
            &sample_host(),
            DeploymentJobType::Install,
            "/var/lib/doro-agent",
            &sample_bootstrap(),
            &artifact,
        );
        assert_eq!(vars["doro_agent_install_mode"], "docker_image");
        assert_eq!(
            vars["doro_agent_image_repository"],
            "docker.io/example/doro-agent"
        );
        assert_eq!(vars["doro_agent_image_tag"], "0.2.0");
        assert_eq!(vars["doro_agent_image_digest"], "sha256:bbbb");
        assert_eq!(
            vars["doro_agent_image_digest_reference"],
            "docker.io/example/doro-agent@sha256:bbbb"
        );
    }

    #[test]
    fn aggregate_job_status_reports_failed_and_cancelled() {
        assert_eq!(
            aggregate_job_status(0, 0, 1, 1),
            DeploymentTargetStatus::Cancelled
        );
        assert_eq!(
            aggregate_job_status(1, 0, 0, 1),
            DeploymentTargetStatus::Succeeded
        );
        assert_eq!(
            aggregate_job_status(0, 1, 0, 1),
            DeploymentTargetStatus::Failed
        );
        assert_eq!(
            aggregate_job_status(0, 0, 0, 1),
            DeploymentTargetStatus::Running
        );
    }
}
