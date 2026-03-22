pub mod degraded;
pub mod diagnostics;
pub mod heartbeat;
pub mod sender;
pub mod state_writer;

pub mod v1 {
    pub use crate::proto::runtime::v1::*;
}

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    config::SourceConfig,
    error::{AppError, TransportErrorKind},
    metadata::{
        BuildMetadata, ClusterMetadata, CompatibilitySnapshot, IdentityStatusSnapshot,
        InstallMetadata, PathMetadata, PlatformMetadata, RuntimeMetadataContext,
    },
    security::SecurityPostureStatusSnapshot,
    state::{FileOffsetRecord, RuntimeStateRecord, SpoolStats},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePhase {
    Starting,
    Enrolling,
    PolicySyncing,
    Online,
    Degraded,
    Stopping,
    Error,
}

impl RuntimePhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Enrolling => "enrolling",
            Self::PolicySyncing => "policy_syncing",
            Self::Online => "online",
            Self::Degraded => "degraded",
            Self::Stopping => "stopping",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "starting" => Some(Self::Starting),
            "enrolling" => Some(Self::Enrolling),
            "policy_syncing" => Some(Self::PolicySyncing),
            "online" => Some(Self::Online),
            "degraded" => Some(Self::Degraded),
            "stopping" => Some(Self::Stopping),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsSnapshot {
    pub agent_id: String,
    pub hostname: String,
    pub version: String,
    pub uptime_sec: u64,
    pub current_policy_revision: Option<String>,
    pub runtime_status: String,
    pub runtime_status_reason: Option<String>,
    pub degraded_mode: bool,
    pub degraded_reason: Option<String>,
    pub blocked_delivery: bool,
    pub blocked_reason: Option<String>,
    pub runtime_mode: String,
    pub active_sources: usize,
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
    pub policy_state: PolicyStateSnapshot,
    pub connectivity_state: ConnectivityStateSnapshot,
    pub source_statuses: Vec<SourceStatusSnapshot>,
    pub platform: PlatformMetadata,
    pub build: BuildMetadata,
    pub install: InstallMetadata,
    pub paths: PathMetadata,
    pub state: StateSnapshot,
    pub compatibility: CompatibilitySnapshot,
    pub cluster: ClusterMetadata,
    pub identity_status: IdentityStatusSnapshot,
    pub security_posture: SecurityPostureStatusSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportStateSnapshot {
    pub mode: String,
    pub server_unavailable_for_sec: u64,
    pub last_error_kind: Option<String>,
    pub blocked_delivery: bool,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyStateSnapshot {
    pub current_policy_revision: Option<String>,
    pub last_policy_fetch_at: Option<i64>,
    pub last_policy_apply_at: Option<i64>,
    pub last_policy_error: Option<String>,
    pub active_source_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityStateSnapshot {
    pub endpoint: String,
    pub tls_enabled: bool,
    pub mtls_enabled: bool,
    pub server_name: Option<String>,
    pub ca_path: Option<String>,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ca_path_present: bool,
    pub cert_path_present: bool,
    pub key_path_present: bool,
    pub last_connect_error: Option<String>,
    pub last_tls_error: Option<String>,
    pub last_handshake_success_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatusSnapshot {
    pub source_id: String,
    pub path: String,
    pub source: String,
    pub service: String,
    pub status: String,
    pub inode: Option<u64>,
    pub live_read_offset: u64,
    pub durable_read_offset: u64,
    pub acked_offset: u64,
    pub live_pending_bytes: u64,
    pub durable_pending_bytes: u64,
    pub last_read_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub state_db_path: String,
    pub state_db_exists: bool,
    pub state_db_accessible: bool,
    pub persisted_identity_present: bool,
    pub current_policy_revision: Option<String>,
    pub last_known_edge_url: Option<String>,
    pub spool_enabled: bool,
    pub spooled_batches: usize,
    pub spooled_bytes: u64,
    pub last_successful_send_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RuntimeStaticContext {
    pub metadata: RuntimeMetadataContext,
    pub state_db_exists: bool,
    pub state_db_accessible: bool,
    pub persisted_identity_present: bool,
    pub last_known_edge_url: Option<String>,
    pub identity_status: IdentityStatusSnapshot,
    pub connectivity: ConnectivityStaticContext,
}

#[derive(Debug, Clone)]
pub struct ConnectivityStaticContext {
    pub endpoint: String,
    pub tls_enabled: bool,
    pub mtls_enabled: bool,
    pub server_name: Option<String>,
    pub ca_path: Option<String>,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatusHandle {
    inner: Arc<Mutex<RuntimeStatus>>,
}

#[derive(Debug, Clone)]
pub struct ControllerSnapshot {
    pub degraded_mode: bool,
    pub blocked_delivery: bool,
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
    build: BuildMetadata,
    platform: PlatformMetadata,
    install: InstallMetadata,
    paths: PathMetadata,
    cluster: ClusterMetadata,
    compatibility: CompatibilitySnapshot,
    identity_status: IdentityStatusSnapshot,
    state_db_exists: bool,
    state_db_accessible: bool,
    persisted_identity_present: bool,
    last_known_edge_url: Option<String>,
    runtime_phase: RuntimePhase,
    runtime_phase_reason: Option<String>,
    last_policy_fetch_at: Option<i64>,
    last_policy_apply_at: Option<i64>,
    last_policy_error: Option<String>,
    connectivity: ConnectivityStaticContext,
    last_connect_error: Option<String>,
    last_tls_error: Option<String>,
    last_handshake_success_at: Option<i64>,
    degraded_mode: bool,
    degraded_reason: Option<String>,
    blocked_delivery: bool,
    blocked_reason: Option<String>,
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
    security_posture: SecurityPostureStatusSnapshot,
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
        static_context: RuntimeStaticContext,
        spool_enabled: bool,
        sources: &[SourceConfig],
        persisted_offsets: &[FileOffsetRecord],
        last_successful_send_at: Option<i64>,
    ) -> Self {
        let source_statuses = build_source_states(sources, persisted_offsets);

        Self {
            inner: Arc::new(Mutex::new(RuntimeStatus {
                agent_id,
                hostname,
                version,
                transport_mode,
                current_policy_revision: None,
                build: static_context.metadata.build,
                platform: static_context.metadata.platform,
                install: static_context.metadata.install,
                paths: static_context.metadata.paths,
                cluster: static_context.metadata.cluster,
                compatibility: static_context.metadata.compatibility,
                identity_status: static_context.identity_status,
                state_db_exists: static_context.state_db_exists,
                state_db_accessible: static_context.state_db_accessible,
                persisted_identity_present: static_context.persisted_identity_present,
                last_known_edge_url: static_context.last_known_edge_url,
                runtime_phase: RuntimePhase::Starting,
                runtime_phase_reason: None,
                last_policy_fetch_at: None,
                last_policy_apply_at: None,
                last_policy_error: None,
                connectivity: static_context.connectivity,
                last_connect_error: None,
                last_tls_error: None,
                last_handshake_success_at: None,
                degraded_mode: false,
                degraded_reason: None,
                blocked_delivery: false,
                blocked_reason: None,
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
                security_posture: SecurityPostureStatusSnapshot::default(),
            })),
        }
    }

    pub fn set_agent_id(&self, agent_id: String) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.agent_id = agent_id;
        }
    }

    pub fn current_agent_id(&self) -> String {
        self.inner
            .lock()
            .map(|inner| inner.agent_id.clone())
            .unwrap_or_default()
    }

    pub fn set_policy_revision(&self, revision: Option<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_policy_revision = revision;
        }
    }

    pub fn set_identity_status(&self, status: IdentityStatusSnapshot) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.identity_status = status;
        }
    }

    pub fn set_last_known_edge_url(&self, edge_url: Option<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_known_edge_url = edge_url;
        }
    }

    pub fn set_runtime_phase(&self, phase: RuntimePhase, reason: Option<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.runtime_phase = phase;
            inner.runtime_phase_reason = reason;
        }
    }

    pub fn restore_runtime_state(&self, runtime_state: &RuntimeStateRecord) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_policy_revision = runtime_state.applied_policy_revision.clone();
            inner.last_known_edge_url = runtime_state.last_known_edge_url.clone();
            inner.last_successful_send_at = runtime_state.last_successful_send_at_unix_ms;
            inner.last_policy_fetch_at = runtime_state.last_policy_fetch_at_unix_ms;
            inner.last_policy_apply_at = runtime_state.last_policy_apply_at_unix_ms;
            inner.last_policy_error = runtime_state.last_policy_error.clone();
            inner.last_connect_error = runtime_state.last_connect_error.clone();
            inner.last_tls_error = runtime_state.last_tls_error.clone();
            inner.last_handshake_success_at = runtime_state.last_handshake_success_at_unix_ms;
            if let Some(phase) = runtime_state
                .runtime_status
                .as_deref()
                .and_then(RuntimePhase::parse)
            {
                inner.runtime_phase = phase;
            }
            inner.runtime_phase_reason = runtime_state.runtime_status_reason.clone();
            if let Some(status) = runtime_state.identity_status.clone() {
                inner.identity_status.status = status;
            }
            inner.identity_status.reason = runtime_state.identity_status_reason.clone();
        }
    }

    pub fn restore_security_posture(&self, snapshot: SecurityPostureStatusSnapshot) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.security_posture = snapshot;
        }
    }

    pub fn set_security_posture_snapshot(&self, snapshot: SecurityPostureStatusSnapshot) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.security_posture = snapshot;
        }
    }

    pub fn current_runtime_phase(&self) -> RuntimePhase {
        self.inner
            .lock()
            .map(|inner| inner.runtime_phase)
            .unwrap_or(RuntimePhase::Error)
    }

    pub fn set_policy_fetch_result(&self, timestamp_unix_ms: i64, error: Option<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_policy_fetch_at = Some(timestamp_unix_ms);
            if error.is_some() {
                inner.last_policy_error = error;
            }
        }
    }

    pub fn set_policy_apply_success(
        &self,
        revision: Option<String>,
        timestamp_unix_ms: i64,
        _source_count: usize,
    ) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_policy_revision = revision;
            inner.last_policy_apply_at = Some(timestamp_unix_ms);
            inner.last_policy_error = None;
        }
    }

    pub fn set_policy_error(&self, error: impl Into<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_policy_error = Some(error.into());
        }
    }

    pub fn record_connectivity_success(&self, timestamp_unix_ms: i64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_handshake_success_at = Some(timestamp_unix_ms);
        }
    }

    pub fn record_connectivity_error(&self, error: &AppError) {
        let message = error.to_string();
        if let Ok(mut inner) = self.inner.lock() {
            if is_tls_error(error) {
                inner.last_tls_error = Some(message);
            } else if is_connect_error(error) {
                inner.last_connect_error = Some(message);
            }
        }
    }

    pub fn set_configured_sources(
        &self,
        sources: &[SourceConfig],
        persisted_offsets: &[FileOffsetRecord],
    ) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.sources = merge_source_states(&inner.sources, sources, persisted_offsets);
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
            inner.last_handshake_success_at = Some(timestamp_unix_ms);
            inner.consecutive_send_failures = 0;
            inner.server_unavailable_since = None;
            inner.blocked_delivery = false;
            inner.blocked_reason = None;
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

    pub fn set_blocked_delivery(&self, enabled: bool, reason: Option<String>) -> bool {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.blocked_delivery == enabled && inner.blocked_reason == reason {
                return false;
            }
            inner.blocked_delivery = enabled;
            inner.blocked_reason = reason;
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

    pub fn is_blocked_delivery(&self) -> bool {
        self.inner
            .lock()
            .map(|inner| inner.blocked_delivery)
            .unwrap_or(false)
    }

    pub fn blocked_reason(&self) -> Option<String> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.blocked_reason.clone())
    }

    pub fn reader_backoff_duration(&self) -> Duration {
        self.inner
            .lock()
            .map(|inner| {
                if inner.storage_pressure || inner.blocked_delivery {
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
        live_read_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.source_id = source_id.to_string();
            source.file_key = file_key.clone();
            source.live_read_offset = live_read_offset;
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
        live_read_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.source_id = source_id.to_string();
            source.file_key = file_key.clone();
            source.live_read_offset = live_read_offset;
            source.durable_read_offset = live_read_offset.max(source.durable_read_offset);
            source.last_error = None;
        });
    }

    pub fn record_source_read(
        &self,
        path: &str,
        source_id: &str,
        file_key: Option<String>,
        live_read_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.source_id = source_id.to_string();
            source.file_key = file_key.clone();
            source.live_read_offset = live_read_offset;
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
            blocked_delivery: inner.blocked_delivery,
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
                live_read_offset: source.live_read_offset,
                durable_read_offset: source.durable_read_offset,
                acked_offset: source.acked_offset,
                live_pending_bytes: source.live_read_offset.saturating_sub(source.acked_offset),
                durable_pending_bytes: source
                    .durable_read_offset
                    .saturating_sub(source.acked_offset),
                last_read_at: source.last_read_at,
                last_error: source.last_error.clone(),
            })
            .collect::<Vec<_>>();
        let compatibility = build_dynamic_compatibility(&inner.compatibility, &source_statuses);

        DiagnosticsSnapshot {
            agent_id: inner.agent_id.clone(),
            hostname: inner.hostname.clone(),
            version: inner.version.clone(),
            uptime_sec: inner.started_at.elapsed().as_secs(),
            current_policy_revision: inner.current_policy_revision.clone(),
            runtime_status: inner.runtime_phase.as_str().to_string(),
            runtime_status_reason: inner.runtime_phase_reason.clone(),
            degraded_mode: inner.degraded_mode,
            degraded_reason: inner.degraded_reason.clone(),
            blocked_delivery: inner.blocked_delivery,
            blocked_reason: inner.blocked_reason.clone(),
            runtime_mode: if inner.blocked_delivery {
                "blocked_delivery".to_string()
            } else if inner.degraded_mode {
                "degraded".to_string()
            } else {
                "normal".to_string()
            },
            active_sources: inner.sources.len(),
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
                blocked_delivery: inner.blocked_delivery,
                blocked_reason: inner.blocked_reason.clone(),
            },
            policy_state: PolicyStateSnapshot {
                current_policy_revision: inner.current_policy_revision.clone(),
                last_policy_fetch_at: inner.last_policy_fetch_at,
                last_policy_apply_at: inner.last_policy_apply_at,
                last_policy_error: inner.last_policy_error.clone(),
                active_source_count: inner.sources.len(),
            },
            connectivity_state: ConnectivityStateSnapshot {
                endpoint: inner.connectivity.endpoint.clone(),
                tls_enabled: inner.connectivity.tls_enabled,
                mtls_enabled: inner.connectivity.mtls_enabled,
                server_name: inner.connectivity.server_name.clone(),
                ca_path: inner.connectivity.ca_path.clone(),
                cert_path: inner.connectivity.cert_path.clone(),
                key_path: inner.connectivity.key_path.clone(),
                ca_path_present: path_present(inner.connectivity.ca_path.as_deref()),
                cert_path_present: path_present(inner.connectivity.cert_path.as_deref()),
                key_path_present: path_present(inner.connectivity.key_path.as_deref()),
                last_connect_error: inner.last_connect_error.clone(),
                last_tls_error: inner.last_tls_error.clone(),
                last_handshake_success_at: inner.last_handshake_success_at,
            },
            source_statuses,
            platform: inner.platform.clone(),
            build: inner.build.clone(),
            install: inner.install.clone(),
            paths: inner.paths.clone(),
            state: StateSnapshot {
                state_db_path: inner.paths.state_db_path.clone(),
                state_db_exists: inner.state_db_exists,
                state_db_accessible: inner.state_db_accessible,
                persisted_identity_present: inner.persisted_identity_present,
                current_policy_revision: inner.current_policy_revision.clone(),
                last_known_edge_url: inner.last_known_edge_url.clone(),
                spool_enabled: inner.spool_enabled,
                spooled_batches: inner.spool_stats.batch_count,
                spooled_bytes: inner.spool_stats.total_bytes,
                last_successful_send_at: inner.last_successful_send_at,
            },
            compatibility,
            cluster: inner.cluster.clone(),
            identity_status: inner.identity_status.clone(),
            security_posture: inner.security_posture.clone(),
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

fn build_source_states(
    sources: &[SourceConfig],
    persisted_offsets: &[FileOffsetRecord],
) -> BTreeMap<String, SourceRuntimeState> {
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
            source.durable_read_offset = offset.durable_read_offset;
            source.acked_offset = offset.acked_offset;
            source.live_read_offset = offset.durable_read_offset.max(offset.acked_offset);
        }
    }

    source_statuses
}

fn merge_source_states(
    current: &BTreeMap<String, SourceRuntimeState>,
    sources: &[SourceConfig],
    persisted_offsets: &[FileOffsetRecord],
) -> BTreeMap<String, SourceRuntimeState> {
    let fresh = build_source_states(sources, persisted_offsets);
    let mut merged = BTreeMap::new();
    for (path, new_state) in fresh {
        if let Some(existing) = current.get(&path) {
            let mut state = existing.clone();
            state.source_id = new_state.source_id;
            state.source = new_state.source;
            state.service = new_state.service;
            state.path = new_state.path;
            if state.file_key.is_none() {
                state.file_key = new_state.file_key;
            }
            if state.durable_read_offset == 0 {
                state.durable_read_offset = new_state.durable_read_offset;
            }
            if state.acked_offset == 0 {
                state.acked_offset = new_state.acked_offset;
            }
            if state.live_read_offset == 0 {
                state.live_read_offset = new_state.live_read_offset;
            }
            merged.insert(path, state);
        } else {
            merged.insert(path, new_state);
        }
    }
    merged
}

fn is_tls_error(error: &AppError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    matches!(error, AppError::Http(_))
        && (message.contains("tls")
            || message.contains("certificate")
            || message.contains("rustls")
            || message.contains("ssl"))
}

fn is_connect_error(error: &AppError) -> bool {
    match error {
        AppError::Http(error) => error.is_connect() || error.is_timeout() || error.is_request(),
        AppError::HttpStatus { status, .. } => status.is_server_error(),
        AppError::GrpcStatus { code, .. } => matches!(code, 4 | 8 | 13 | 14),
        _ => false,
    }
}

fn path_present(path: Option<&str>) -> bool {
    path.map(std::path::Path::new)
        .map(std::path::Path::exists)
        .unwrap_or(false)
}

fn parse_inode(file_key: Option<&str>) -> Option<u64> {
    file_key
        .and_then(|value| value.rsplit(':').next())
        .and_then(|value| value.parse::<u64>().ok())
}

fn build_dynamic_compatibility(
    base: &CompatibilitySnapshot,
    source_statuses: &[SourceStatusSnapshot],
) -> CompatibilitySnapshot {
    let mut snapshot = base.clone();
    for source in source_statuses {
        if let Some(error) = &source.last_error {
            snapshot.source_path_issues.push(format!(
                "source `{}` at `{}` reports `{error}`",
                source.source_id, source.path
            ));
        }
    }
    snapshot
}

#[cfg(test)]
pub fn test_static_context() -> RuntimeStaticContext {
    RuntimeStaticContext {
        metadata: RuntimeMetadataContext {
            build: crate::metadata::BuildMetadata::current(),
            platform: crate::metadata::PlatformMetadata {
                os_family: "linux".to_string(),
                distro_name: Some("demo".to_string()),
                distro_version: Some("1".to_string()),
                kernel_version: Some("6.0.0".to_string()),
                architecture: "x86_64".to_string(),
                hostname: "demo-host".to_string(),
                machine_id_hash: None,
                service_manager: "systemd".to_string(),
                systemd_detected: true,
            },
            install: crate::metadata::InstallMetadata {
                configured_mode: "dev".to_string(),
                resolved_mode: "dev".to_string(),
                resolution_source: "explicit".to_string(),
                canonical_layout: false,
                systemd_expected: false,
                notes: Vec::new(),
                warnings: Vec::new(),
            },
            paths: crate::metadata::PathMetadata {
                current_exe: "/tmp/doro-agent".to_string(),
                config_path: "/tmp/config.yaml".to_string(),
                state_dir: "/tmp/doro-agent".to_string(),
                spool_dir: "/tmp/doro-agent/spool".to_string(),
                state_db_path: "/tmp/doro-agent/state.db".to_string(),
                canonical_package_bin: "/usr/bin/doro-agent".to_string(),
                canonical_config_path: "/etc/doro-agent/config.yaml".to_string(),
                canonical_env_path: "/etc/doro-agent/agent.env".to_string(),
                canonical_state_dir: "/var/lib/doro-agent".to_string(),
                canonical_spool_dir: "/var/lib/doro-agent/spool".to_string(),
                canonical_log_dir: "/var/log/doro-agent".to_string(),
                service_unit_name: "doro-agent.service".to_string(),
            },
            cluster: ClusterMetadata {
                configured_cluster_id: Some("cluster-a".to_string()),
                cluster_name: Some("prod".to_string()),
                service_name: Some("api".to_string()),
                environment: Some("production".to_string()),
                configured_cluster_tags: Default::default(),
                effective_cluster_tags: Default::default(),
                host_labels: Default::default(),
            },
            compatibility: CompatibilitySnapshot::default(),
            event_enrichment: crate::metadata::EventEnrichmentContext::default(),
        },
        state_db_exists: true,
        state_db_accessible: true,
        persisted_identity_present: true,
        last_known_edge_url: Some("https://edge.example.local".to_string()),
        identity_status: IdentityStatusSnapshot {
            status: "reused".to_string(),
            reason: Some("persisted identity accepted".to_string()),
        },
        connectivity: ConnectivityStaticContext {
            endpoint: "https://edge.example.local:7443".to_string(),
            tls_enabled: true,
            mtls_enabled: true,
            server_name: Some("edge.example.local".to_string()),
            ca_path: Some("/etc/doro-agent/ca.pem".to_string()),
            cert_path: Some("/etc/doro-agent/agent.pem".to_string()),
            key_path: Some("/etc/doro-agent/agent.key".to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        config::{SourceConfig, StartAt},
        error::TransportErrorKind,
        state::FileOffsetRecord,
    };

    use super::{test_static_context, RuntimeStatusHandle};

    #[test]
    fn builds_diagnostics_snapshot() {
        let status = RuntimeStatusHandle::new(
            "agent-1".to_string(),
            "demo-host".to_string(),
            "0.1.0".to_string(),
            "mock".to_string(),
            test_static_context(),
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
                durable_read_offset: 30,
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
        assert_eq!(snapshot.active_sources, 1);
        assert_eq!(snapshot.source_statuses[0].live_read_offset, 42);
        assert_eq!(snapshot.source_statuses[0].durable_read_offset, 30);
        assert_eq!(snapshot.source_statuses[0].acked_offset, 20);
        assert_eq!(snapshot.source_statuses[0].live_pending_bytes, 22);
        assert_eq!(snapshot.source_statuses[0].durable_pending_bytes, 10);
        assert!(snapshot.degraded_mode);
        assert_eq!(
            snapshot.last_error_kind.as_deref(),
            Some("TransientNetwork")
        );
        assert_eq!(snapshot.identity_status.status, "reused");
        assert_eq!(
            snapshot.state.last_known_edge_url.as_deref(),
            Some("https://edge.example.local")
        );
        assert_eq!(snapshot.cluster.cluster_name.as_deref(), Some("prod"));
        assert_eq!(snapshot.security_posture.last_status, "never_run");
    }
}
