pub mod degraded;
pub mod diagnostics;
pub mod heartbeat;
pub mod sender;
pub mod state_writer;

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    config::SourceConfig,
    error::TransportErrorKind,
    state::{FileOffsetRecord, SpoolStats},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsSnapshot {
    pub agent_id: String,
    pub hostname: String,
    pub version: String,
    pub uptime_sec: u64,
    pub current_policy_revision: Option<String>,
    pub degraded_mode: bool,
    pub degraded_reason: Option<String>,
    pub runtime_mode: String,
    pub event_queue_len: usize,
    pub event_queue_bytes: usize,
    pub send_queue_len: usize,
    pub send_queue_bytes: usize,
    pub spool_enabled: bool,
    pub spooled_batches: usize,
    pub spooled_bytes: u64,
    pub last_error: Option<String>,
    pub last_error_kind: Option<String>,
    pub last_successful_send_at: Option<i64>,
    pub consecutive_send_failures: u32,
    pub transport_state: TransportStateSnapshot,
    pub source_statuses: Vec<SourceStatusSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportStateSnapshot {
    pub mode: String,
    pub server_unavailable_for_sec: u64,
    pub last_error_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatusSnapshot {
    pub source_id: String,
    pub path: String,
    pub source: String,
    pub service: String,
    pub status: String,
    pub inode: Option<u64>,
    pub read_offset: u64,
    pub acked_offset: u64,
    pub pending_bytes: u64,
    pub last_read_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatusHandle {
    inner: Arc<Mutex<RuntimeStatus>>,
}

#[derive(Debug, Clone)]
pub struct ControllerSnapshot {
    pub degraded_mode: bool,
    pub consecutive_send_failures: u32,
    pub server_unavailable_for_sec: u64,
    pub event_queue_len: usize,
    pub event_queue_bytes: usize,
    pub send_queue_len: usize,
    pub send_queue_bytes: usize,
    pub total_unacked_lag_bytes: u64,
    pub spooled_batches: usize,
    pub spooled_bytes: u64,
}

#[derive(Debug)]
struct RuntimeStatus {
    agent_id: String,
    hostname: String,
    version: String,
    transport_mode: String,
    current_policy_revision: Option<String>,
    degraded_mode: bool,
    degraded_reason: Option<String>,
    storage_pressure: bool,
    spool_enabled: bool,
    last_error: Option<String>,
    last_error_kind: Option<TransportErrorKind>,
    last_successful_send_at: Option<i64>,
    consecutive_send_failures: u32,
    server_unavailable_since: Option<i64>,
    started_at: Instant,
    event_queue_len: usize,
    event_queue_bytes: usize,
    send_queue_len: usize,
    send_queue_bytes: usize,
    spool_stats: SpoolStats,
    sources: BTreeMap<String, SourceRuntimeState>,
}

#[derive(Debug, Clone)]
struct SourceRuntimeState {
    source_id: String,
    path: String,
    source: String,
    service: String,
    status: String,
    file_key: Option<String>,
    live_read_offset: u64,
    durable_read_offset: u64,
    acked_offset: u64,
    last_read_at: Option<i64>,
    last_error: Option<String>,
}

impl RuntimeStatusHandle {
    pub fn new(
        agent_id: String,
        hostname: String,
        version: String,
        transport_mode: String,
        spool_enabled: bool,
        sources: &[SourceConfig],
        persisted_offsets: &[FileOffsetRecord],
        last_successful_send_at: Option<i64>,
    ) -> Self {
        let mut source_statuses = BTreeMap::new();
        for source in sources {
            source_statuses.insert(
                source.path.to_string_lossy().into_owned(),
                SourceRuntimeState {
                    source_id: source.source_id().to_string(),
                    path: source.path.to_string_lossy().into_owned(),
                    source: source.source.clone(),
                    service: source.service.clone(),
                    status: "idle".to_string(),
                    file_key: None,
                    live_read_offset: 0,
                    durable_read_offset: 0,
                    acked_offset: 0,
                    last_read_at: None,
                    last_error: None,
                },
            );
        }

        for offset in persisted_offsets {
            if let Some(source) = source_statuses.get_mut(&offset.path) {
                source.file_key = offset.file_key.clone();
                source.durable_read_offset = offset.read_offset;
                source.acked_offset = offset.acked_offset;
                source.live_read_offset = offset.read_offset.max(offset.acked_offset);
            }
        }

        Self {
            inner: Arc::new(Mutex::new(RuntimeStatus {
                agent_id,
                hostname,
                version,
                transport_mode,
                current_policy_revision: None,
                degraded_mode: false,
                degraded_reason: None,
                storage_pressure: false,
                spool_enabled,
                last_error: None,
                last_error_kind: None,
                last_successful_send_at,
                consecutive_send_failures: 0,
                server_unavailable_since: None,
                started_at: Instant::now(),
                event_queue_len: 0,
                event_queue_bytes: 0,
                send_queue_len: 0,
                send_queue_bytes: 0,
                spool_stats: SpoolStats::default(),
                sources: source_statuses,
            })),
        }
    }

    pub fn set_agent_id(&self, agent_id: String) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.agent_id = agent_id;
        }
    }

    pub fn set_policy_revision(&self, revision: Option<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_policy_revision = revision;
        }
    }

    pub fn update_spool_stats(&self, stats: SpoolStats) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.spool_stats = stats;
        }
    }

    pub fn set_storage_pressure(&self, enabled: bool) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.storage_pressure = enabled;
        }
    }

    pub fn record_send_success(&self, timestamp_unix_ms: i64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_successful_send_at = Some(timestamp_unix_ms);
            inner.consecutive_send_failures = 0;
            inner.server_unavailable_since = None;
            inner.last_error = None;
            inner.last_error_kind = None;
        }
    }

    pub fn record_send_failure(&self, error: impl Into<String>, error_kind: TransportErrorKind) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.consecutive_send_failures = inner.consecutive_send_failures.saturating_add(1);
            inner.last_error = Some(error.into());
            inner.last_error_kind = Some(error_kind);
            if inner.server_unavailable_since.is_none() {
                inner.server_unavailable_since = Some(Utc::now().timestamp_millis());
            }
        }
    }

    pub fn record_error(&self, error: impl Into<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_error = Some(error.into());
        }
    }

    pub fn set_degraded_mode(&self, enabled: bool, reason: Option<String>) -> bool {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.degraded_mode == enabled && inner.degraded_reason == reason {
                return false;
            }
            inner.degraded_mode = enabled;
            inner.degraded_reason = reason;
            return true;
        }
        false
    }

    pub fn is_degraded_mode(&self) -> bool {
        self.inner
            .lock()
            .map(|inner| inner.degraded_mode)
            .unwrap_or(false)
    }

    pub fn reader_backoff_duration(&self) -> Duration {
        self.inner
            .lock()
            .map(|inner| {
                if inner.storage_pressure {
                    Duration::from_secs(2)
                } else if inner.degraded_mode {
                    Duration::from_millis(500)
                } else {
                    Duration::from_millis(100)
                }
            })
            .unwrap_or_else(|_| Duration::from_millis(500))
    }

    pub fn current_event_queue_bytes(&self) -> usize {
        self.inner
            .lock()
            .map(|inner| inner.event_queue_bytes)
            .unwrap_or_default()
    }

    pub fn current_send_queue_bytes(&self) -> usize {
        self.inner
            .lock()
            .map(|inner| inner.send_queue_bytes)
            .unwrap_or_default()
    }

    pub fn current_send_queue_len(&self) -> usize {
        self.inner
            .lock()
            .map(|inner| inner.send_queue_len)
            .unwrap_or_default()
    }

    pub fn current_consecutive_failures(&self) -> u32 {
        self.inner
            .lock()
            .map(|inner| inner.consecutive_send_failures)
            .unwrap_or_default()
    }

    pub fn record_event_queue_push(&self, approx_bytes: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.event_queue_len = inner.event_queue_len.saturating_add(1);
            inner.event_queue_bytes = inner.event_queue_bytes.saturating_add(approx_bytes);
        }
    }

    pub fn record_event_queue_pop(&self, approx_bytes: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.event_queue_len = inner.event_queue_len.saturating_sub(1);
            inner.event_queue_bytes = inner.event_queue_bytes.saturating_sub(approx_bytes);
        }
    }

    pub fn record_event_queue_full(&self) {
        self.record_error("event queue is saturated");
    }

    pub fn record_send_queue_push(&self, approx_bytes: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.send_queue_len = inner.send_queue_len.saturating_add(1);
            inner.send_queue_bytes = inner.send_queue_bytes.saturating_add(approx_bytes);
        }
    }

    pub fn record_send_queue_pop(&self, approx_bytes: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.send_queue_len = inner.send_queue_len.saturating_sub(1);
            inner.send_queue_bytes = inner.send_queue_bytes.saturating_sub(approx_bytes);
        }
    }

    pub fn record_send_queue_full(&self) {
        self.record_error("send queue is saturated");
    }

    pub fn record_source_ready(
        &self,
        path: &str,
        source_id: &str,
        file_key: Option<String>,
        read_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.source_id = source_id.to_string();
            source.file_key = file_key.clone();
            source.live_read_offset = read_offset;
            source.last_error = None;
        });
    }

    pub fn record_source_rotation_detected(
        &self,
        path: &str,
        source_id: &str,
        file_key: Option<String>,
    ) {
        self.update_source(path, |source| {
            source.status = "rotating".to_string();
            source.source_id = source_id.to_string();
            source.file_key = file_key.clone();
            source.last_error = None;
        });
    }

    pub fn record_source_replaced(
        &self,
        path: &str,
        source_id: &str,
        file_key: Option<String>,
        read_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.source_id = source_id.to_string();
            source.file_key = file_key.clone();
            source.live_read_offset = read_offset;
            source.durable_read_offset = read_offset.max(source.durable_read_offset);
            source.last_error = None;
        });
    }

    pub fn record_source_read(
        &self,
        path: &str,
        source_id: &str,
        file_key: Option<String>,
        read_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.source_id = source_id.to_string();
            source.file_key = file_key.clone();
            source.live_read_offset = read_offset;
            source.last_read_at = Some(Utc::now().timestamp_millis());
            source.last_error = None;
        });
    }

    pub fn record_source_durable_read(
        &self,
        path: &str,
        file_key: Option<String>,
        durable_read_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.file_key = file_key.clone();
            source.durable_read_offset = durable_read_offset;
            if source.live_read_offset < durable_read_offset {
                source.live_read_offset = durable_read_offset;
            }
        });
    }

    pub fn record_source_commit(&self, path: &str, file_key: Option<String>, acked_offset: u64) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.file_key = file_key.clone();
            source.acked_offset = acked_offset;
            if source.durable_read_offset < acked_offset {
                source.durable_read_offset = acked_offset;
            }
            if source.live_read_offset < acked_offset {
                source.live_read_offset = acked_offset;
            }
        });
    }

    pub fn record_source_missing(&self, path: &str, error: String) {
        self.update_source(path, |source| {
            source.status = "waiting".to_string();
            source.last_error = Some(error.clone());
        });
        self.record_error(error);
    }

    pub fn record_source_error(&self, path: &str, error: String) {
        self.update_source(path, |source| {
            source.status = "error".to_string();
            source.last_error = Some(error.clone());
        });
        self.record_error(error);
    }

    pub fn controller_snapshot(&self) -> ControllerSnapshot {
        let inner = self.inner.lock().expect("runtime status lock poisoned");
        ControllerSnapshot {
            degraded_mode: inner.degraded_mode,
            consecutive_send_failures: inner.consecutive_send_failures,
            server_unavailable_for_sec: inner
                .server_unavailable_since
                .map(|started| {
                    Utc::now().timestamp_millis().saturating_sub(started).max(0) as u64 / 1_000
                })
                .unwrap_or_default(),
            event_queue_len: inner.event_queue_len,
            event_queue_bytes: inner.event_queue_bytes,
            send_queue_len: inner.send_queue_len,
            send_queue_bytes: inner.send_queue_bytes,
            total_unacked_lag_bytes: inner
                .sources
                .values()
                .map(|source| source.live_read_offset.saturating_sub(source.acked_offset))
                .sum(),
            spooled_batches: inner.spool_stats.batch_count,
            spooled_bytes: inner.spool_stats.total_bytes,
        }
    }

    pub fn snapshot(&self) -> DiagnosticsSnapshot {
        let inner = self.inner.lock().expect("runtime status lock poisoned");
        let source_statuses = inner
            .sources
            .values()
            .map(|source| SourceStatusSnapshot {
                source_id: source.source_id.clone(),
                path: source.path.clone(),
                source: source.source.clone(),
                service: source.service.clone(),
                status: source.status.clone(),
                inode: parse_inode(source.file_key.as_deref()),
                read_offset: source.live_read_offset,
                acked_offset: source.acked_offset,
                pending_bytes: source.live_read_offset.saturating_sub(source.acked_offset),
                last_read_at: source.last_read_at,
                last_error: source.last_error.clone(),
            })
            .collect::<Vec<_>>();

        DiagnosticsSnapshot {
            agent_id: inner.agent_id.clone(),
            hostname: inner.hostname.clone(),
            version: inner.version.clone(),
            uptime_sec: inner.started_at.elapsed().as_secs(),
            current_policy_revision: inner.current_policy_revision.clone(),
            degraded_mode: inner.degraded_mode,
            degraded_reason: inner.degraded_reason.clone(),
            runtime_mode: if inner.degraded_mode {
                "degraded".to_string()
            } else {
                "normal".to_string()
            },
            event_queue_len: inner.event_queue_len,
            event_queue_bytes: inner.event_queue_bytes,
            send_queue_len: inner.send_queue_len,
            send_queue_bytes: inner.send_queue_bytes,
            spool_enabled: inner.spool_enabled,
            spooled_batches: inner.spool_stats.batch_count,
            spooled_bytes: inner.spool_stats.total_bytes,
            last_error: inner.last_error.clone(),
            last_error_kind: inner.last_error_kind.map(|kind| kind.to_string()),
            last_successful_send_at: inner.last_successful_send_at,
            consecutive_send_failures: inner.consecutive_send_failures,
            transport_state: TransportStateSnapshot {
                mode: inner.transport_mode.clone(),
                server_unavailable_for_sec: inner
                    .server_unavailable_since
                    .map(|started| {
                        Utc::now().timestamp_millis().saturating_sub(started).max(0) as u64 / 1_000
                    })
                    .unwrap_or_default(),
                last_error_kind: inner.last_error_kind.map(|kind| kind.to_string()),
            },
            source_statuses,
        }
    }

    fn update_source<F>(&self, path: &str, update: F)
    where
        F: FnOnce(&mut SourceRuntimeState),
    {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(source) = inner.sources.get_mut(path) {
                update(source);
            }
        }
    }
}

fn parse_inode(file_key: Option<&str>) -> Option<u64> {
    file_key
        .and_then(|value| value.rsplit(':').next())
        .and_then(|value| value.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        config::{SourceConfig, StartAt},
        error::TransportErrorKind,
        state::FileOffsetRecord,
    };

    use super::RuntimeStatusHandle;

    #[test]
    fn builds_diagnostics_snapshot() {
        let status = RuntimeStatusHandle::new(
            "agent-1".to_string(),
            "demo-host".to_string(),
            "0.1.0".to_string(),
            "mock".to_string(),
            true,
            &[SourceConfig {
                kind: "file".to_string(),
                source_id: Some("file:/tmp/demo.log".to_string()),
                path: PathBuf::from("/tmp/demo.log"),
                start_at: StartAt::End,
                source: "syslog".to_string(),
                service: "demo".to_string(),
                severity_hint: "info".to_string(),
            }],
            &[FileOffsetRecord {
                path: "/tmp/demo.log".to_string(),
                file_key: Some("1:2".to_string()),
                read_offset: 30,
                acked_offset: 20,
                updated_at_unix_ms: 0,
            }],
            Some(99),
        );
        status.set_policy_revision(Some("rev-1".to_string()));
        status.record_source_read(
            "/tmp/demo.log",
            "file:/tmp/demo.log",
            Some("1:2".to_string()),
            42,
        );
        status.record_send_failure("network timeout", TransportErrorKind::TransientNetwork);
        status.set_degraded_mode(true, Some("queue pressure".to_string()));

        let snapshot = status.snapshot();
        assert_eq!(snapshot.hostname, "demo-host");
        assert_eq!(snapshot.current_policy_revision.as_deref(), Some("rev-1"));
        assert_eq!(snapshot.source_statuses[0].read_offset, 42);
        assert_eq!(snapshot.source_statuses[0].acked_offset, 20);
        assert!(snapshot.degraded_mode);
        assert_eq!(
            snapshot.last_error_kind.as_deref(),
            Some("TransientNetwork")
        );
    }
}
