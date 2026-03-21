use std::path::{Path, PathBuf};

use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tracing::{info, warn};

use crate::{
    batching::PendingBatch,
    error::{AppError, AppResult},
    security::SecurityScanStateRecord,
    state::{
        decode_spool_payload, encode_spool_payload, remove_spool_payload, write_spool_payload,
        FileOffsetUpdate, RuntimeStatePatch, SourceOffsetMarker, SpoolBatchRecord, SpoolStats,
        SqliteStateStore,
    },
};

#[derive(Debug, Clone)]
pub struct RuntimeFlagsUpdate {
    pub degraded_mode: bool,
    pub blocked_delivery: bool,
    pub blocked_reason: Option<String>,
    pub spool_enabled: bool,
    pub consecutive_send_failures: u32,
    pub last_successful_send_at_unix_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct StateWriterHandle {
    tx: mpsc::Sender<StateCommand>,
}

enum StateCommand {
    SpoolBatch {
        batch: PendingBatch,
        compress_threshold_bytes: usize,
        reply: oneshot::Sender<AppResult<SpoolStats>>,
    },
    AckBatch {
        batch: PendingBatch,
        runtime: RuntimeFlagsUpdate,
        reply: oneshot::Sender<AppResult<SpoolStats>>,
    },
    MarkSpoolRetry {
        batch_id: String,
        attempt_count: u32,
        next_retry_at_unix_ms: i64,
        reply: oneshot::Sender<AppResult<()>>,
    },
    LoadDueSpoolBatch {
        now_unix_ms: i64,
        reply: oneshot::Sender<AppResult<Option<PendingBatch>>>,
    },
    UpdateRuntimeFlags {
        runtime: RuntimeFlagsUpdate,
        reply: oneshot::Sender<AppResult<()>>,
    },
    UpdateRuntimeState {
        patch: RuntimeStatePatch,
        reply: oneshot::Sender<AppResult<()>>,
    },
    SaveSecurityScanState {
        state: SecurityScanStateRecord,
        reply: oneshot::Sender<AppResult<()>>,
    },
}

impl StateWriterHandle {
    pub async fn spool_batch(
        &self,
        batch: PendingBatch,
        compress_threshold_bytes: usize,
    ) -> AppResult<SpoolStats> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StateCommand::SpoolBatch {
                batch,
                compress_threshold_bytes,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AppError::protocol("state writer has stopped"))?;
        reply_rx
            .await
            .map_err(|_| AppError::protocol("state writer dropped spool reply"))?
    }

    pub async fn ack_batch(
        &self,
        batch: PendingBatch,
        runtime: RuntimeFlagsUpdate,
    ) -> AppResult<SpoolStats> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StateCommand::AckBatch {
                batch,
                runtime,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AppError::protocol("state writer has stopped"))?;
        reply_rx
            .await
            .map_err(|_| AppError::protocol("state writer dropped ack reply"))?
    }

    pub async fn mark_spool_retry(
        &self,
        batch_id: String,
        attempt_count: u32,
        next_retry_at_unix_ms: i64,
    ) -> AppResult<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StateCommand::MarkSpoolRetry {
                batch_id,
                attempt_count,
                next_retry_at_unix_ms,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AppError::protocol("state writer has stopped"))?;
        reply_rx
            .await
            .map_err(|_| AppError::protocol("state writer dropped retry reply"))?
    }

    pub async fn load_due_spooled_batch(
        &self,
        now_unix_ms: i64,
    ) -> AppResult<Option<PendingBatch>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StateCommand::LoadDueSpoolBatch {
                now_unix_ms,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AppError::protocol("state writer has stopped"))?;
        reply_rx
            .await
            .map_err(|_| AppError::protocol("state writer dropped load reply"))?
    }

    pub async fn update_runtime_flags(&self, runtime: RuntimeFlagsUpdate) -> AppResult<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StateCommand::UpdateRuntimeFlags {
                runtime,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AppError::protocol("state writer has stopped"))?;
        reply_rx
            .await
            .map_err(|_| AppError::protocol("state writer dropped runtime reply"))?
    }

    pub async fn update_runtime_state(&self, patch: RuntimeStatePatch) -> AppResult<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StateCommand::UpdateRuntimeState {
                patch,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AppError::protocol("state writer has stopped"))?;
        reply_rx
            .await
            .map_err(|_| AppError::protocol("state writer dropped runtime state reply"))?
    }

    pub async fn save_security_scan_state(&self, state: SecurityScanStateRecord) -> AppResult<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StateCommand::SaveSecurityScanState {
                state,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AppError::protocol("state writer has stopped"))?;
        reply_rx
            .await
            .map_err(|_| AppError::protocol("state writer dropped security scan reply"))?
    }
}

pub fn spawn_state_writer(
    store: SqliteStateStore,
    spool_dir: PathBuf,
    max_disk_bytes: u64,
) -> (StateWriterHandle, JoinHandle<AppResult<()>>) {
    let (tx, mut rx) = mpsc::channel(64);
    let handle = tokio::task::spawn_blocking(move || {
        recover_broken_spool_entries(&store)?;
        while let Some(command) = rx.blocking_recv() {
            match command {
                StateCommand::SpoolBatch {
                    batch,
                    compress_threshold_bytes,
                    reply,
                } => {
                    let result = handle_spool_batch(
                        &store,
                        &spool_dir,
                        max_disk_bytes,
                        batch,
                        compress_threshold_bytes,
                    );
                    let _ = reply.send(result);
                }
                StateCommand::AckBatch {
                    batch,
                    runtime,
                    reply,
                } => {
                    let result = handle_ack_batch(&store, batch, runtime);
                    let _ = reply.send(result);
                }
                StateCommand::MarkSpoolRetry {
                    batch_id,
                    attempt_count,
                    next_retry_at_unix_ms,
                    reply,
                } => {
                    let result =
                        store.mark_spool_retry(&batch_id, attempt_count, next_retry_at_unix_ms);
                    let _ = reply.send(result);
                }
                StateCommand::LoadDueSpoolBatch { now_unix_ms, reply } => {
                    let result = handle_load_due_spool_batch(&store, now_unix_ms);
                    let _ = reply.send(result);
                }
                StateCommand::UpdateRuntimeFlags { runtime, reply } => {
                    let result = update_runtime_flags(&store, runtime);
                    let _ = reply.send(result);
                }
                StateCommand::UpdateRuntimeState { patch, reply } => {
                    let result = store.apply_runtime_state_patch(patch).map(|_| ());
                    let _ = reply.send(result);
                }
                StateCommand::SaveSecurityScanState { state, reply } => {
                    let result = store.save_security_scan_state(&state);
                    let _ = reply.send(result);
                }
            }
        }
        Ok(())
    });

    (StateWriterHandle { tx }, handle)
}

fn handle_spool_batch(
    store: &SqliteStateStore,
    spool_dir: &Path,
    max_disk_bytes: u64,
    batch: PendingBatch,
    compress_threshold_bytes: usize,
) -> AppResult<SpoolStats> {
    let current_stats = store.spool_stats()?;
    if current_stats
        .total_bytes
        .saturating_add(batch.approx_bytes as u64)
        > max_disk_bytes
    {
        return Err(AppError::protocol("spool disk limit exceeded"));
    }

    let (codec, payload) = encode_spool_payload(&batch.batch, compress_threshold_bytes)?;
    let payload_path = write_spool_payload(spool_dir, &batch.batch_id, &codec, &payload)?;
    store.insert_spool_batch(&SpoolBatchRecord {
        batch_id: batch.batch_id.clone(),
        payload_path,
        codec,
        created_at_unix_ms: batch.created_at_unix_ms,
        attempt_count: batch.attempt_count,
        next_retry_at_unix_ms: batch.created_at_unix_ms,
        approx_bytes: batch.approx_bytes,
        source_offsets: batch.source_offsets.clone(),
    })?;
    store.commit_file_offsets(&merge_read_offset_updates(store, &batch.source_offsets)?)?;
    store.spool_stats()
}

fn handle_ack_batch(
    store: &SqliteStateStore,
    batch: PendingBatch,
    runtime: RuntimeFlagsUpdate,
) -> AppResult<SpoolStats> {
    if batch.from_spool {
        if let Some(path) = batch.spool_payload_path.as_ref() {
            let _ = remove_spool_payload(path);
        }
        store.delete_spool_batch(&batch.batch_id)?;
    }

    store.commit_file_offsets(&merge_ack_offset_updates(store, &batch.source_offsets)?)?;
    update_runtime_flags(store, runtime)?;
    store.spool_stats()
}

fn handle_load_due_spool_batch(
    store: &SqliteStateStore,
    now_unix_ms: i64,
) -> AppResult<Option<PendingBatch>> {
    loop {
        let Some(record) = store.load_due_spool_batch(now_unix_ms)? else {
            return Ok(None);
        };
        let payload = match std::fs::read(&record.payload_path) {
            Ok(payload) => payload,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                warn!(
                    batch_id = record.batch_id,
                    payload_path = %record.payload_path.display(),
                    "removing broken spool metadata with missing payload"
                );
                store.delete_spool_batch(&record.batch_id)?;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let batch = decode_spool_payload(&record.codec, &payload)?;
        return Ok(Some(PendingBatch {
            batch_id: record.batch_id,
            batch,
            approx_bytes: record.approx_bytes,
            source_offsets: record.source_offsets,
            created_at_unix_ms: record.created_at_unix_ms,
            attempt_count: record.attempt_count,
            from_spool: true,
            spool_payload_path: Some(record.payload_path),
            spool_codec: Some(record.codec),
        }));
    }
}

fn update_runtime_flags(store: &SqliteStateStore, runtime: RuntimeFlagsUpdate) -> AppResult<()> {
    let mut state = store.load_runtime_state()?;
    state.degraded_mode = runtime.degraded_mode;
    state.blocked_delivery = runtime.blocked_delivery;
    state.blocked_reason = runtime.blocked_reason;
    state.spool_enabled = runtime.spool_enabled;
    state.consecutive_send_failures = runtime.consecutive_send_failures;
    state.last_successful_send_at_unix_ms = runtime
        .last_successful_send_at_unix_ms
        .or(state.last_successful_send_at_unix_ms);
    state.updated_at_unix_ms = chrono::Utc::now().timestamp_millis();
    store.save_runtime_state(&state)
}

fn merge_read_offset_updates(
    store: &SqliteStateStore,
    markers: &[SourceOffsetMarker],
) -> AppResult<Vec<FileOffsetUpdate>> {
    let mut updates = Vec::with_capacity(markers.len());
    for marker in markers {
        let existing = store.load_file_offset(Path::new(&marker.path))?;
        let acked_offset = existing
            .as_ref()
            .map(|record| record.acked_offset)
            .unwrap_or_default();
        let durable_read_offset = existing
            .as_ref()
            .map(|record| record.durable_read_offset.max(marker.offset))
            .unwrap_or(marker.offset);
        updates.push(FileOffsetUpdate {
            path: marker.path.clone(),
            file_key: marker.file_key.clone(),
            durable_read_offset,
            acked_offset,
        });
    }
    Ok(updates)
}

fn merge_ack_offset_updates(
    store: &SqliteStateStore,
    markers: &[SourceOffsetMarker],
) -> AppResult<Vec<FileOffsetUpdate>> {
    let mut updates = Vec::with_capacity(markers.len());
    for marker in markers {
        let existing = store.load_file_offset(Path::new(&marker.path))?;
        let durable_read_offset = existing
            .as_ref()
            .map(|record| record.durable_read_offset.max(marker.offset))
            .unwrap_or(marker.offset);
        let acked_offset = existing
            .as_ref()
            .map(|record| record.acked_offset.max(marker.offset))
            .unwrap_or(marker.offset);
        updates.push(FileOffsetUpdate {
            path: marker.path.clone(),
            file_key: marker.file_key.clone(),
            durable_read_offset,
            acked_offset,
        });
    }
    Ok(updates)
}

fn recover_broken_spool_entries(store: &SqliteStateStore) -> AppResult<()> {
    let records = store.list_spool_batches()?;
    let mut recovered = 0usize;
    for record in records {
        if record.payload_path.exists() {
            continue;
        }
        warn!(
            batch_id = record.batch_id,
            payload_path = %record.payload_path.display(),
            "cleaning broken spool metadata left without payload"
        );
        store.delete_spool_batch(&record.batch_id)?;
        recovered = recovered.saturating_add(1);
    }
    if recovered > 0 {
        info!(recovered, "recovered broken spool metadata entries");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::{
        proto::ingest,
        state::{
            encode_spool_payload, write_spool_payload, SourceOffsetMarker, SpoolBatchRecord,
            SqliteStateStore,
        },
    };

    use super::{spawn_state_writer, RuntimeFlagsUpdate};

    fn make_batch(batch_id: &str, path: &str, offset: u64) -> crate::batching::PendingBatch {
        crate::batching::PendingBatch {
            batch_id: batch_id.to_string(),
            batch: ingest::LogBatch {
                agent_id: "agent-1".to_string(),
                host: "demo-host".to_string(),
                sent_at_unix_ms: 10,
                events: vec![ingest::LogEvent {
                    timestamp_unix_ms: 10,
                    message: "hello".to_string(),
                    source: "demo".to_string(),
                    source_type: "file".to_string(),
                    service: "svc".to_string(),
                    severity: "info".to_string(),
                    labels: Default::default(),
                    raw: "hello".to_string(),
                }],
            },
            approx_bytes: 128,
            source_offsets: vec![SourceOffsetMarker {
                source_id: format!("file:{path}"),
                path: path.to_string(),
                file_key: Some("1:2".to_string()),
                offset,
            }],
            created_at_unix_ms: 10,
            attempt_count: 0,
            from_spool: false,
            spool_payload_path: None,
            spool_codec: None,
        }
    }

    #[tokio::test]
    async fn removes_broken_spool_metadata_on_startup() {
        let dir = TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        store
            .insert_spool_batch(&SpoolBatchRecord {
                batch_id: "broken-1".to_string(),
                payload_path: dir.path().join("spool").join("broken-1.bin"),
                codec: "identity".to_string(),
                created_at_unix_ms: 10,
                attempt_count: 0,
                next_retry_at_unix_ms: 0,
                approx_bytes: 64,
                source_offsets: vec![SourceOffsetMarker {
                    source_id: "file:/tmp/demo.log".to_string(),
                    path: "/tmp/demo.log".to_string(),
                    file_key: Some("1:2".to_string()),
                    offset: 64,
                }],
            })
            .unwrap();

        let (handle, worker) =
            spawn_state_writer(store.clone(), dir.path().join("spool"), 64 * 1024 * 1024);
        assert!(handle
            .load_due_spooled_batch(i64::MAX)
            .await
            .unwrap()
            .is_none());
        drop(handle);
        worker.await.unwrap().unwrap();

        assert_eq!(store.spool_stats().unwrap().batch_count, 0);
    }

    #[tokio::test]
    async fn loads_next_valid_spool_batch_after_missing_payload_recovery() {
        let dir = TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let valid = make_batch("valid-1", "/tmp/demo.log", 64);
        let (codec, payload) = encode_spool_payload(&valid.batch, usize::MAX).unwrap();
        let payload_path =
            write_spool_payload(&dir.path().join("spool"), &valid.batch_id, &codec, &payload)
                .unwrap();

        store
            .insert_spool_batch(&SpoolBatchRecord {
                batch_id: "broken-1".to_string(),
                payload_path: dir.path().join("spool").join("broken-1.bin"),
                codec: "identity".to_string(),
                created_at_unix_ms: 10,
                attempt_count: 0,
                next_retry_at_unix_ms: 0,
                approx_bytes: 64,
                source_offsets: valid.source_offsets.clone(),
            })
            .unwrap();
        store
            .insert_spool_batch(&SpoolBatchRecord {
                batch_id: valid.batch_id.clone(),
                payload_path,
                codec,
                created_at_unix_ms: 20,
                attempt_count: 0,
                next_retry_at_unix_ms: 0,
                approx_bytes: valid.approx_bytes,
                source_offsets: valid.source_offsets.clone(),
            })
            .unwrap();

        let (handle, worker) =
            spawn_state_writer(store.clone(), dir.path().join("spool"), 64 * 1024 * 1024);
        let loaded = handle
            .load_due_spooled_batch(i64::MAX)
            .await
            .unwrap()
            .unwrap();
        drop(handle);
        worker.await.unwrap().unwrap();

        assert_eq!(loaded.batch_id, "valid-1");
        assert_eq!(store.spool_stats().unwrap().batch_count, 1);
    }

    #[test]
    fn runtime_flags_carry_blocked_delivery_state() {
        let flags = RuntimeFlagsUpdate {
            degraded_mode: true,
            blocked_delivery: true,
            blocked_reason: Some("permanent transport failure".to_string()),
            spool_enabled: true,
            consecutive_send_failures: 4,
            last_successful_send_at_unix_ms: None,
        };

        assert!(flags.degraded_mode);
        assert!(flags.blocked_delivery);
        assert_eq!(
            flags.blocked_reason.as_deref(),
            Some("permanent transport failure")
        );
    }
}
