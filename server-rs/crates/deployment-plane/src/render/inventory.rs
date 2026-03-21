use serde_json::{json, Value};

use crate::models::{DeploymentTargetSnapshot, DeploymentTargetStatus, ResolvedHost};

pub fn render_inventory_ini(target: &DeploymentTargetSnapshot) -> String {
    format!(
        "[targets]\n{} ansible_host={} ansible_port={} ansible_user={}\n",
        target.host.hostname, target.host.ip, target.host.ssh_port, target.host.remote_user
    )
}

pub fn render_target_vars(host: &ResolvedHost, action: &str, bootstrap_path: &str) -> Value {
    json!({
        "host_id": host.host_id,
        "hostname": host.hostname,
        "ip": host.ip,
        "ssh_port": host.ssh_port,
        "remote_user": host.remote_user,
        "action": action,
        "bootstrap_path": bootstrap_path,
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
