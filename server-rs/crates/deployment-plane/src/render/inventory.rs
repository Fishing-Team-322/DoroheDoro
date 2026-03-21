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
    json!({
        "host_id": host.host_id,
        "hostname": host.hostname,
        "ip": host.ip,
        "ssh_port": host.ssh_port,
        "remote_user": host.remote_user,
        "doro_agent_deployment_job_type": job_type.as_str(),
        "doro_agent_state": if matches!(job_type, DeploymentJobType::Uninstall) { "absent" } else { "present" },
        "doro_agent_install_mode": if artifact.package_type == "deb" { "deb" } else if artifact.package_type == "tar.gz" { "tar.gz" } else { "auto" },
        "doro_agent_selected_artifact": artifact,
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
