#[cfg(test)]
use std::sync::{LazyLock, Mutex, MutexGuard};

#[cfg(test)]
static AGENT_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[cfg(test)]
const AGENT_ENV_KEYS: &[&str] = &[
    "EDGE_URL",
    "EDGE_GRPC_ADDR",
    "BOOTSTRAP_TOKEN",
    "STATE_DIR",
    "LOG_LEVEL",
    "HEARTBEAT_INTERVAL_SEC",
    "DIAGNOSTICS_INTERVAL_SEC",
    "POLICY_REFRESH_INTERVAL_SEC",
    "BATCH_MAX_EVENTS",
    "BATCH_MAX_BYTES",
    "BATCH_FLUSH_INTERVAL_MS",
    "BATCH_FLUSH_INTERVAL_SEC",
    "BATCH_COMPRESS_THRESHOLD_BYTES",
    "QUEUE_EVENT_CAPACITY",
    "QUEUE_SEND_CAPACITY",
    "QUEUE_EVENT_BYTES_SOFT_LIMIT",
    "QUEUE_SEND_BYTES_SOFT_LIMIT",
    "DEGRADED_FAILURE_THRESHOLD",
    "DEGRADED_SERVER_UNAVAILABLE_SEC",
    "DEGRADED_QUEUE_PRESSURE_PCT",
    "DEGRADED_QUEUE_RECOVER_PCT",
    "DEGRADED_UNACKED_LAG_BYTES",
    "DEGRADED_SHUTDOWN_SPOOL_GRACE_SEC",
    "SPOOL_ENABLED",
    "SPOOL_DIR",
    "SPOOL_MAX_DISK_BYTES",
    "TRANSPORT_MODE",
    "INSTALL_MODE",
    "ALLOW_MACHINE_ID",
    "TLS_CA_PATH",
    "TLS_CERT_PATH",
    "TLS_KEY_PATH",
    "TLS_SERVER_NAME",
    "CLUSTER_ID",
    "CLUSTER_NAME",
    "SERVICE_NAME",
    "ENVIRONMENT",
];

#[cfg(test)]
pub struct TestEnvGuard {
    _guard: MutexGuard<'static, ()>,
}

#[cfg(test)]
pub fn lock_agent_env() -> TestEnvGuard {
    let guard = AGENT_ENV_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    clear_agent_env();
    TestEnvGuard { _guard: guard }
}

#[cfg(test)]
pub fn clear_agent_env() {
    for key in AGENT_ENV_KEYS {
        std::env::remove_var(key);
    }
}

#[cfg(test)]
impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        clear_agent_env();
    }
}
