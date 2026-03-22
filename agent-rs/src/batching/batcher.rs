use std::{collections::BTreeMap, time::Duration};

use chrono::Utc;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    batching::PendingBatch,
    config::{BatchConfig, QueueConfig, SpoolConfig},
    proto::ingest,
    runtime::{state_writer::StateWriterHandle, RuntimeStatusHandle},
    sources::SourceEvent,
    state::SourceOffsetMarker,
};

pub fn spawn_batcher(
    mut rx: mpsc::Receiver<SourceEvent>,
    send_tx: mpsc::Sender<PendingBatch>,
    state_writer: StateWriterHandle,
    status: RuntimeStatusHandle,
    shutdown: CancellationToken,
    batch_config: BatchConfig,
    queue_config: QueueConfig,
    spool_config: SpoolConfig,
    host: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut buffer = Vec::<SourceEvent>::new();
        let mut buffered_bytes = 0_usize;
        let mut batch_started_at = None;
        let mut ticker = tokio::time::interval(Duration::from_millis(100));
        let mut shutdown_requested = false;

        loop {
            tokio::select! {
                _ = shutdown.cancelled(), if !shutdown_requested => {
                    shutdown_requested = true;
                }
                maybe_event = rx.recv() => {
                    match maybe_event {
                        Some(event) => {
                            status.record_event_queue_pop(event.approx_bytes);
                            if batch_started_at.is_none() {
                                batch_started_at = Some(tokio::time::Instant::now());
                            }
                            buffered_bytes = buffered_bytes.saturating_add(event.approx_bytes);
                            buffer.push(event);
                        }
                        None => {
                            shutdown_requested = true;
                        }
                    }
                }
                _ = ticker.tick() => {}
            }

            if buffer.is_empty() {
                if shutdown_requested && rx.is_closed() {
                    info!("batcher stopped");
                    return;
                }
                continue;
            }

            let flush_due = buffer.len() >= batch_config.max_events
                || buffered_bytes >= batch_config.max_bytes
                || batch_started_at
                    .map(|started| {
                        started.elapsed() >= Duration::from_millis(batch_config.flush_interval_ms)
                    })
                    .unwrap_or(false);
            let closing = shutdown_requested && rx.is_closed();

            if !flush_due && !closing {
                continue;
            }

            let pending_batch =
                build_pending_batch(&buffer, buffered_bytes, &status.current_agent_id(), &host);
            let mut batch_to_dispatch = Some(pending_batch);

            while let Some(batch) = batch_to_dispatch.take() {
                match send_tx.try_send(batch) {
                    Ok(()) => {
                        status.record_send_queue_push(buffered_bytes);
                        buffer.clear();
                        buffered_bytes = 0;
                        batch_started_at = None;
                    }
                    Err(mpsc::error::TrySendError::Full(batch)) => {
                        status.record_send_queue_full();
                        if should_spool_batch(&status, &queue_config, &spool_config, closing) {
                            match state_writer
                                .spool_batch(batch.clone(), batch_config.compress_threshold_bytes)
                                .await
                            {
                                Ok(spool_stats) => {
                                    status.update_spool_stats(spool_stats);
                                    for offset in checkpoint_updates(&buffer) {
                                        status.record_source_durable_read(
                                            &offset.path,
                                            offset.file_key.clone(),
                                            offset.offset,
                                        );
                                    }
                                    buffer.clear();
                                    buffered_bytes = 0;
                                    batch_started_at = None;
                                }
                                Err(error) => {
                                    status.record_error(format!("spool write failed: {error}"));
                                    warn!(error = %error, "failed to write fallback spool from batcher");
                                    tokio::time::sleep(Duration::from_millis(200)).await;
                                    batch_to_dispatch = Some(batch);
                                }
                            }
                        } else {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            batch_to_dispatch = Some(batch);
                        }
                    }
                    Err(mpsc::error::TrySendError::Closed(batch)) => {
                        if spool_config.enabled {
                            match state_writer
                                .spool_batch(batch.clone(), batch_config.compress_threshold_bytes)
                                .await
                            {
                                Ok(spool_stats) => {
                                    status.update_spool_stats(spool_stats);
                                    for offset in checkpoint_updates(&buffer) {
                                        status.record_source_durable_read(
                                            &offset.path,
                                            offset.file_key.clone(),
                                            offset.offset,
                                        );
                                    }
                                }
                                Err(error) => {
                                    status.record_error(format!("spool write failed: {error}"));
                                    warn!(error = %error, "failed to spool batch after sender shutdown");
                                }
                            }
                        }
                        info!("batcher stopped after sender queue closed");
                        return;
                    }
                }
            }
        }
    })
}

fn build_pending_batch(
    buffer: &[SourceEvent],
    approx_bytes: usize,
    agent_id: &str,
    host: &str,
) -> PendingBatch {
    PendingBatch {
        batch_id: Uuid::new_v4().to_string(),
        batch: ingest::LogBatch {
            agent_id: agent_id.to_string(),
            host: host.to_string(),
            events: buffer.iter().map(|item| item.event.clone()).collect(),
            sent_at_unix_ms: Utc::now().timestamp_millis(),
        },
        approx_bytes,
        source_offsets: checkpoint_updates(buffer),
        created_at_unix_ms: Utc::now().timestamp_millis(),
        attempt_count: 0,
        from_spool: false,
        spool_payload_path: None,
        spool_codec: None,
    }
}

fn checkpoint_updates(buffer: &[SourceEvent]) -> Vec<SourceOffsetMarker> {
    let mut updates = BTreeMap::<String, SourceOffsetMarker>::new();
    for item in buffer {
        updates.insert(
            item.checkpoint.path.clone(),
            SourceOffsetMarker {
                source_id: item.checkpoint.source_id.clone(),
                path: item.checkpoint.path.clone(),
                file_key: item.checkpoint.file_key.clone(),
                offset: item.checkpoint.offset,
            },
        );
    }
    updates.into_values().collect()
}

fn should_spool_batch(
    status: &RuntimeStatusHandle,
    queue_config: &QueueConfig,
    spool_config: &SpoolConfig,
    closing: bool,
) -> bool {
    if !spool_config.enabled {
        return false;
    }

    closing
        || status.is_degraded_mode()
        || status.current_send_queue_len() >= queue_config.send_capacity
        || status.current_send_queue_bytes() >= queue_config.send_bytes_soft_limit
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::Duration};

    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use crate::{
        batching::PendingBatch,
        config::{BatchConfig, QueueConfig, SourceConfig, SpoolConfig, StartAt},
        runtime::{
            state_writer::{spawn_state_writer, StateWriterHandle},
            test_static_context, RuntimeStatusHandle,
        },
        sources::{SourceCheckpoint, SourceEvent},
        state::SqliteStateStore,
    };

    use super::spawn_batcher;

    fn test_status() -> RuntimeStatusHandle {
        RuntimeStatusHandle::new(
            "agent-1".to_string(),
            "demo-host".to_string(),
            "0.1.0".to_string(),
            "mock".to_string(),
            test_static_context(),
            true,
            30,
            30,
            &[SourceConfig {
                kind: "file".to_string(),
                source_id: Some("file:/tmp/demo.log".to_string()),
                path: PathBuf::from("/tmp/demo.log"),
                start_at: StartAt::End,
                source: "demo".to_string(),
                service: "svc".to_string(),
                severity_hint: "info".to_string(),
            }],
            &[],
            None,
        )
    }

    fn spawn_test_writer(
        store: SqliteStateStore,
    ) -> (
        StateWriterHandle,
        tokio::task::JoinHandle<crate::error::AppResult<()>>,
    ) {
        spawn_state_writer(
            store,
            std::env::temp_dir().join("doro-agent-test-spool"),
            64 * 1024 * 1024,
        )
    }

    #[tokio::test]
    async fn flushes_by_event_count() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let (state_writer, state_handle) = spawn_test_writer(store);
        let status = test_status();
        let (source_tx, source_rx) = mpsc::channel(8);
        let (send_tx, mut send_rx) = mpsc::channel::<PendingBatch>(8);
        let shutdown = CancellationToken::new();

        let handle = spawn_batcher(
            source_rx,
            send_tx,
            state_writer,
            status,
            shutdown.clone(),
            BatchConfig {
                max_events: 2,
                max_bytes: 1024,
                flush_interval_ms: 60_000,
                compress_threshold_bytes: 16 * 1024,
            },
            QueueConfig::default(),
            SpoolConfig::default(),
            "demo-host".to_string(),
        );

        for offset in [1_u64, 2_u64] {
            source_tx
                .send(SourceEvent {
                    checkpoint: SourceCheckpoint {
                        source_id: "file:/tmp/demo.log".to_string(),
                        path: "/tmp/demo.log".to_string(),
                        file_key: Some("1:2".to_string()),
                        offset,
                    },
                    approx_bytes: 64,
                    event: crate::proto::ingest::LogEvent {
                        timestamp_unix_ms: 1,
                        message: "hello".to_string(),
                        source: "demo".to_string(),
                        source_type: "file".to_string(),
                        service: "svc".to_string(),
                        severity: "info".to_string(),
                        labels: Default::default(),
                        raw: "hello".to_string(),
                    },
                })
                .await
                .unwrap();
        }

        let pending = tokio::time::timeout(Duration::from_secs(2), send_rx.recv())
            .await
            .unwrap()
            .unwrap();

        shutdown.cancel();
        drop(source_tx);
        handle.await.unwrap();
        drop(send_rx);
        drop(state_handle);

        assert_eq!(pending.event_count(), 2);
    }
}
