use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    config::{DegradedConfig, QueueConfig},
    error::AppResult,
    runtime::{
        state_writer::{RuntimeFlagsUpdate, StateWriterHandle},
        RuntimeStatusHandle,
    },
};

pub fn spawn_degraded_controller(
    status: RuntimeStatusHandle,
    state_writer: StateWriterHandle,
    shutdown: CancellationToken,
    degraded: DegradedConfig,
    queues: QueueConfig,
    spool_enabled: bool,
    spool_max_disk_bytes: u64,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(500));
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                _ = ticker.tick() => {
                    let snapshot = status.controller_snapshot();
                    let queue_pressure_pct = max_queue_pressure_pct(&snapshot, &queues);
                    let storage_pressure = spool_enabled && snapshot.spooled_bytes >= spool_max_disk_bytes;
                    status.set_storage_pressure(storage_pressure);

                    let reason = degraded_reason(
                        &snapshot,
                        queue_pressure_pct,
                        storage_pressure,
                        &degraded,
                    );
                    let should_enable = reason.is_some();
                    let should_disable = snapshot.degraded_mode
                        && snapshot.consecutive_send_failures == 0
                        && snapshot.server_unavailable_for_sec == 0
                        && queue_pressure_pct <= degraded.queue_recover_pct
                        && snapshot.total_unacked_lag_bytes < degraded.unacked_lag_bytes / 2
                        && snapshot.spooled_batches == 0
                        && !storage_pressure;

                    let target_mode = if should_enable {
                        true
                    } else if should_disable {
                        false
                    } else {
                        snapshot.degraded_mode
                    };
                    let target_reason = if target_mode { reason } else { None };

                    if status.set_degraded_mode(target_mode, target_reason.clone()) {
                        state_writer
                            .update_runtime_flags(RuntimeFlagsUpdate {
                                degraded_mode: target_mode,
                                spool_enabled,
                                consecutive_send_failures: status.current_consecutive_failures(),
                                last_successful_send_at_unix_ms: None,
                            })
                            .await?;
                        if target_mode {
                            warn!(reason = ?target_reason, "agent entered degraded mode");
                        } else {
                            info!("agent left degraded mode");
                        }
                    }
                }
            }
        }
    })
}

fn degraded_reason(
    snapshot: &crate::runtime::ControllerSnapshot,
    queue_pressure_pct: u8,
    storage_pressure: bool,
    degraded: &DegradedConfig,
) -> Option<String> {
    if storage_pressure {
        return Some("spool storage pressure".to_string());
    }
    if snapshot.consecutive_send_failures >= degraded.failure_threshold {
        return Some("consecutive send failures".to_string());
    }
    if snapshot.server_unavailable_for_sec >= degraded.server_unavailable_sec {
        return Some("server unavailable".to_string());
    }
    if queue_pressure_pct >= degraded.queue_pressure_pct {
        return Some("queue pressure".to_string());
    }
    if snapshot.total_unacked_lag_bytes >= degraded.unacked_lag_bytes {
        return Some("unacked lag".to_string());
    }
    None
}

fn max_queue_pressure_pct(
    snapshot: &crate::runtime::ControllerSnapshot,
    queues: &QueueConfig,
) -> u8 {
    let event_len_pct = if queues.event_capacity == 0 {
        0
    } else {
        ((snapshot.event_queue_len.saturating_mul(100)) / queues.event_capacity) as u8
    };
    let send_len_pct = if queues.send_capacity == 0 {
        0
    } else {
        ((snapshot.send_queue_len.saturating_mul(100)) / queues.send_capacity) as u8
    };
    let event_bytes_pct = if queues.event_bytes_soft_limit == 0 {
        0
    } else {
        ((snapshot.event_queue_bytes.saturating_mul(100)) / queues.event_bytes_soft_limit) as u8
    };
    let send_bytes_pct = if queues.send_bytes_soft_limit == 0 {
        0
    } else {
        ((snapshot.send_queue_bytes.saturating_mul(100)) / queues.send_bytes_soft_limit) as u8
    };

    *[event_len_pct, send_len_pct, event_bytes_pct, send_bytes_pct]
        .iter()
        .max()
        .unwrap_or(&0)
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{DegradedConfig, QueueConfig},
        runtime::ControllerSnapshot,
    };

    use super::{degraded_reason, max_queue_pressure_pct};

    #[test]
    fn computes_queue_pressure_from_counts_and_bytes() {
        let pct = max_queue_pressure_pct(
            &ControllerSnapshot {
                degraded_mode: false,
                consecutive_send_failures: 0,
                server_unavailable_for_sec: 0,
                event_queue_len: 80,
                event_queue_bytes: 1024,
                send_queue_len: 4,
                send_queue_bytes: 1024,
                total_unacked_lag_bytes: 0,
                spooled_batches: 0,
                spooled_bytes: 0,
            },
            &QueueConfig {
                event_capacity: 100,
                send_capacity: 10,
                event_bytes_soft_limit: 10_000,
                send_bytes_soft_limit: 10_000,
            },
        );

        assert_eq!(pct, 80);
    }

    #[test]
    fn enters_degraded_on_unacked_lag() {
        let reason = degraded_reason(
            &ControllerSnapshot {
                degraded_mode: false,
                consecutive_send_failures: 0,
                server_unavailable_for_sec: 0,
                event_queue_len: 0,
                event_queue_bytes: 0,
                send_queue_len: 0,
                send_queue_bytes: 0,
                total_unacked_lag_bytes: 1_000,
                spooled_batches: 0,
                spooled_bytes: 0,
            },
            0,
            false,
            &DegradedConfig {
                failure_threshold: 3,
                server_unavailable_sec: 30,
                queue_pressure_pct: 80,
                queue_recover_pct: 40,
                unacked_lag_bytes: 100,
                shutdown_spool_grace_sec: 5,
            },
        );

        assert_eq!(reason.as_deref(), Some("unacked lag"));
    }
}
