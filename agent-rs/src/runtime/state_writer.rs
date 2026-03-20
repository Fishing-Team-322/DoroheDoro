use std::path::{Path, PathBuf};

use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    batching::PendingBatch,
    error::{AppError, AppResult},
    state::{
        decode_spool_payload, encode_spool_payload, remove_spool_payload, write_spool_payload,
        FileOffsetUpdate, SourceOffsetMarker, SpoolBatchRecord, SpoolStats, SqliteStateStore,
    },
};

#[derive(Debug, Clone)]
pub struct RuntimeFlagsUpdate {
    pub degraded_mode: bool,
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
}

pub fn spawn_state_writer(
    store: SqliteStateStore,
    spool_dir: PathBuf,
    max_disk_bytes: u64,
) -> (StateWriterHandle, JoinHandle<AppResult<()>>) {
    let (tx, mut rx) = mpsc::channel(64);
    let handle = tokio::task::spawn_blocking(move || {
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
    let Some(record) = store.load_due_spool_batch(now_unix_ms)? else {
        return Ok(None);
    };
    let payload = std::fs::read(&record.payload_path)?;
    let batch = decode_spool_payload(&record.codec, &payload)?;
    Ok(Some(PendingBatch {
        batch_id: record.batch_id,
        batch,
        approx_bytes: record.approx_bytes,
        source_offsets: record.source_offsets,
        created_at_unix_ms: record.created_at_unix_ms,
        attempt_count: record.attempt_count,
        from_spool: true,
        spool_payload_path: Some(record.payload_path),
        spool_codec: Some(record.codec),
    }))
}

fn update_runtime_flags(store: &SqliteStateStore, runtime: RuntimeFlagsUpdate) -> AppResult<()> {
    let mut state = store.load_runtime_state()?;
    state.degraded_mode = runtime.degraded_mode;
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
        let read_offset = existing
            .as_ref()
            .map(|record| record.read_offset.max(marker.offset))
            .unwrap_or(marker.offset);
        updates.push(FileOffsetUpdate {
            path: marker.path.clone(),
            file_key: marker.file_key.clone(),
            read_offset,
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
        let read_offset = existing
            .as_ref()
            .map(|record| record.read_offset.max(marker.offset))
            .unwrap_or(marker.offset);
        let acked_offset = existing
            .as_ref()
            .map(|record| record.acked_offset.max(marker.offset))
            .unwrap_or(marker.offset);
        updates.push(FileOffsetUpdate {
            path: marker.path.clone(),
            file_key: marker.file_key.clone(),
            read_offset,
            acked_offset,
        });
    }
    Ok(updates)
}
