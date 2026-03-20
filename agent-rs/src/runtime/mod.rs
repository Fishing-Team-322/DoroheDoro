pub mod heartbeat;
pub mod r#loop;

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{config::SourceConfig, state::FileOffsetRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsSnapshot {
    pub hostname: String,
    pub version: String,
    pub current_policy_revision: Option<String>,
    pub last_error: Option<String>,
    pub last_send_timestamp_unix_ms: Option<i64>,
    pub source_statuses: Vec<SourceStatusSnapshot>,
    pub offsets_summary: Vec<OffsetSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatusSnapshot {
    pub path: String,
    pub source: String,
    pub service: String,
    pub status: String,
    pub file_key: Option<String>,
    pub observed_offset: u64,
    pub committed_offset: u64,
    pub last_event_at_unix_ms: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetSummary {
    pub path: String,
    pub observed_offset: u64,
    pub committed_offset: u64,
    pub file_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatusHandle {
    inner: Arc<Mutex<RuntimeStatus>>,
}

#[derive(Debug)]
struct RuntimeStatus {
    hostname: String,
    version: String,
    current_policy_revision: Option<String>,
    last_error: Option<String>,
    last_send_timestamp_unix_ms: Option<i64>,
    sources: BTreeMap<String, SourceStatusSnapshot>,
}

impl RuntimeStatusHandle {
    pub fn new(
        hostname: String,
        version: String,
        sources: &[SourceConfig],
        persisted_offsets: &[FileOffsetRecord],
    ) -> Self {
        let mut source_statuses = BTreeMap::new();
        for source in sources {
            source_statuses.insert(
                source.path.to_string_lossy().into_owned(),
                SourceStatusSnapshot {
                    path: source.path.to_string_lossy().into_owned(),
                    source: source.source.clone(),
                    service: source.service.clone(),
                    status: "idle".to_string(),
                    file_key: None,
                    observed_offset: 0,
                    committed_offset: 0,
                    last_event_at_unix_ms: None,
                    last_error: None,
                },
            );
        }

        for offset in persisted_offsets {
            if let Some(source) = source_statuses.get_mut(&offset.path) {
                source.committed_offset = offset.offset;
                source.observed_offset = offset.offset;
                source.file_key = offset.file_key.clone();
            }
        }

        Self {
            inner: Arc::new(Mutex::new(RuntimeStatus {
                hostname,
                version,
                current_policy_revision: None,
                last_error: None,
                last_send_timestamp_unix_ms: None,
                sources: source_statuses,
            })),
        }
    }

    pub fn set_policy_revision(&self, revision: Option<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_policy_revision = revision;
        }
    }

    pub fn record_last_send(&self, timestamp_unix_ms: i64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_send_timestamp_unix_ms = Some(timestamp_unix_ms);
            inner.last_error = None;
        }
    }

    pub fn record_error(&self, error: impl Into<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_error = Some(error.into());
        }
    }

    pub fn clear_error(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_error = None;
        }
    }

    pub fn record_source_ready(&self, path: &str, file_key: Option<String>, observed_offset: u64) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.file_key = file_key.clone();
            source.observed_offset = observed_offset;
            source.last_error = None;
        });
    }

    pub fn record_source_replaced(
        &self,
        path: &str,
        file_key: Option<String>,
        observed_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.file_key = file_key.clone();
            source.observed_offset = observed_offset;
            source.last_error = None;
        });
    }

    pub fn record_source_read(&self, path: &str, observed_offset: u64) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.observed_offset = observed_offset;
            source.last_event_at_unix_ms = Some(chrono::Utc::now().timestamp_millis());
            source.last_error = None;
        });
    }

    pub fn record_source_commit(
        &self,
        path: &str,
        file_key: Option<String>,
        committed_offset: u64,
    ) {
        self.update_source(path, |source| {
            source.status = "running".to_string();
            source.file_key = file_key.clone();
            source.committed_offset = committed_offset;
            if source.observed_offset < committed_offset {
                source.observed_offset = committed_offset;
            }
        });
    }

    pub fn record_source_error(&self, path: &str, error: String) {
        self.update_source(path, |source| {
            source.status = "error".to_string();
            source.last_error = Some(error.clone());
        });
        self.record_error(error);
    }

    pub fn snapshot(&self) -> DiagnosticsSnapshot {
        let inner = self.inner.lock().expect("runtime status lock poisoned");
        let source_statuses = inner.sources.values().cloned().collect::<Vec<_>>();
        let offsets_summary = source_statuses
            .iter()
            .map(|source| OffsetSummary {
                path: source.path.clone(),
                observed_offset: source.observed_offset,
                committed_offset: source.committed_offset,
                file_key: source.file_key.clone(),
            })
            .collect::<Vec<_>>();

        DiagnosticsSnapshot {
            hostname: inner.hostname.clone(),
            version: inner.version.clone(),
            current_policy_revision: inner.current_policy_revision.clone(),
            last_error: inner.last_error.clone(),
            last_send_timestamp_unix_ms: inner.last_send_timestamp_unix_ms,
            source_statuses,
            offsets_summary,
        }
    }

    fn update_source<F>(&self, path: &str, update: F)
    where
        F: FnOnce(&mut SourceStatusSnapshot),
    {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(source) = inner.sources.get_mut(path) {
                update(source);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{config::SourceConfig, state::FileOffsetRecord};

    use super::RuntimeStatusHandle;

    #[test]
    fn builds_diagnostics_snapshot() {
        let status = RuntimeStatusHandle::new(
            "demo-host".to_string(),
            "0.1.0".to_string(),
            &[SourceConfig {
                kind: "file".to_string(),
                path: PathBuf::from("/tmp/demo.log"),
                source: "syslog".to_string(),
                service: "demo".to_string(),
                severity_hint: "info".to_string(),
            }],
            &[FileOffsetRecord {
                path: "/tmp/demo.log".to_string(),
                file_key: Some("1:2".to_string()),
                offset: 20,
                updated_at_unix_ms: 0,
            }],
        );
        status.set_policy_revision(Some("rev-1".to_string()));
        status.record_source_read("/tmp/demo.log", 42);
        status.record_last_send(99);

        let snapshot = status.snapshot();
        assert_eq!(snapshot.hostname, "demo-host");
        assert_eq!(snapshot.current_policy_revision.as_deref(), Some("rev-1"));
        assert_eq!(snapshot.source_statuses[0].observed_offset, 42);
        assert_eq!(snapshot.offsets_summary[0].committed_offset, 20);
    }
}
