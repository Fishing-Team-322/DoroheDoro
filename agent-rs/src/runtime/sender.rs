use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    batching::PendingBatch,
    error::{AppResult, TransportErrorKind},
    runtime::{
        state_writer::{RuntimeFlagsUpdate, StateWriterHandle},
        RuntimeStatusHandle,
    },
    transport::AgentTransport,
};

pub fn spawn_sender(
    mut rx: mpsc::Receiver<PendingBatch>,
    transport: Arc<dyn AgentTransport>,
    state_writer: StateWriterHandle,
    status: RuntimeStatusHandle,
    shutdown: CancellationToken,
    spool_enabled: bool,
    compress_threshold_bytes: usize,
    shutdown_spool_grace_sec: u64,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let mut retry_batch: Option<(PendingBatch, tokio::time::Instant)> = None;
        let mut shutdown_requested = false;
        let mut shutdown_deadline = None;

        loop {
            if shutdown_requested {
                if shutdown_deadline.is_none() {
                    shutdown_deadline = Some(
                        tokio::time::Instant::now() + Duration::from_secs(shutdown_spool_grace_sec),
                    );
                }

                if let Some((batch, _)) = retry_batch.take() {
                    let _ = spool_in_memory_batch(
                        batch,
                        &state_writer,
                        &status,
                        spool_enabled,
                        compress_threshold_bytes,
                    )
                    .await;
                }

                while let Ok(batch) = rx.try_recv() {
                    status.record_send_queue_pop(batch.approx_bytes);
                    let _ = spool_in_memory_batch(
                        batch,
                        &state_writer,
                        &status,
                        spool_enabled,
                        compress_threshold_bytes,
                    )
                    .await;
                }

                if rx.is_closed() {
                    info!("sender stopped after draining in-memory batches to spool");
                    return Ok(());
                }

                if let Some(deadline) = shutdown_deadline {
                    if tokio::time::Instant::now() >= deadline {
                        info!("sender stopped after shutdown spool grace period elapsed");
                        return Ok(());
                    }
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            if let Some(batch) = state_writer
                .load_due_spooled_batch(Utc::now().timestamp_millis())
                .await?
            {
                process_batch(
                    batch,
                    &transport,
                    &state_writer,
                    &status,
                    spool_enabled,
                    compress_threshold_bytes,
                    &mut retry_batch,
                    shutdown_requested,
                )
                .await?;
                continue;
            }

            if let Some((batch, deadline)) = retry_batch.take() {
                if tokio::time::Instant::now() >= deadline {
                    process_batch(
                        batch,
                        &transport,
                        &state_writer,
                        &status,
                        spool_enabled,
                        compress_threshold_bytes,
                        &mut retry_batch,
                        shutdown_requested,
                    )
                    .await?;
                    continue;
                }
                retry_batch = Some((batch, deadline));
            }

            tokio::select! {
                _ = shutdown.cancelled(), if !shutdown_requested => {
                    shutdown_requested = true;
                }
                maybe_batch = rx.recv() => {
                    match maybe_batch {
                        Some(batch) => {
                            status.record_send_queue_pop(batch.approx_bytes);
                            process_batch(
                                batch,
                                &transport,
                                &state_writer,
                                &status,
                                spool_enabled,
                                compress_threshold_bytes,
                                &mut retry_batch,
                                shutdown_requested,
                            ).await?;
                        }
                        None => {
                            if retry_batch.is_none() {
                                info!("sender stopped");
                                return Ok(());
                            }
                            shutdown_requested = true;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(250)) => {}
            }
        }
    })
}

async fn process_batch(
    batch: PendingBatch,
    transport: &Arc<dyn AgentTransport>,
    state_writer: &StateWriterHandle,
    status: &RuntimeStatusHandle,
    spool_enabled: bool,
    compress_threshold_bytes: usize,
    retry_batch: &mut Option<(PendingBatch, tokio::time::Instant)>,
    shutdown_requested: bool,
) -> AppResult<()> {
    match transport.send_batch(batch.batch.clone()).await {
        Ok(()) => {
            let now = Utc::now().timestamp_millis();
            let runtime = RuntimeFlagsUpdate {
                degraded_mode: status.is_degraded_mode(),
                spool_enabled,
                consecutive_send_failures: 0,
                last_successful_send_at_unix_ms: Some(now),
            };
            let spool_stats = state_writer.ack_batch(batch.clone(), runtime).await?;
            status.update_spool_stats(spool_stats);
            status.record_send_success(now);
            for offset in &batch.source_offsets {
                status.record_source_commit(&offset.path, offset.file_key.clone(), offset.offset);
            }
            info!(
                batch_id = batch.batch_id,
                event_count = batch.event_count(),
                from_spool = batch.from_spool,
                "sent log batch"
            );
        }
        Err(error) => {
            let error_kind = error.transport_error_kind();
            status.record_send_failure(format!("batch send failed: {error}"), error_kind);
            state_writer
                .update_runtime_flags(RuntimeFlagsUpdate {
                    degraded_mode: status.is_degraded_mode(),
                    spool_enabled,
                    consecutive_send_failures: status.current_consecutive_failures(),
                    last_successful_send_at_unix_ms: None,
                })
                .await?;

            if batch.from_spool {
                let next_retry_at_unix_ms = Utc::now().timestamp_millis()
                    + batch
                        .next_retry_delay(error_kind, status.is_degraded_mode())
                        .as_millis() as i64;
                let next_attempt = batch.attempt_count.saturating_add(1);
                state_writer
                    .mark_spool_retry(batch.batch_id.clone(), next_attempt, next_retry_at_unix_ms)
                    .await?;
                warn!(
                    batch_id = batch.batch_id,
                    error = %error,
                    error_kind = %error_kind,
                    "spooled batch send failed, retry scheduled"
                );
                return Ok(());
            }

            let should_spool = shutdown_requested
                || should_spool_after_failure(&batch, status, error_kind, spool_enabled);
            if should_spool {
                if let Err(spool_error) = spool_in_memory_batch(
                    batch.clone(),
                    state_writer,
                    status,
                    spool_enabled,
                    compress_threshold_bytes,
                )
                .await
                {
                    error!(
                        batch_id = batch.batch_id,
                        error = %spool_error,
                        "failed to spool batch after send error"
                    );
                    schedule_memory_retry(batch, status, error_kind, retry_batch);
                }
            } else {
                schedule_memory_retry(batch, status, error_kind, retry_batch);
            }
        }
    }

    Ok(())
}

fn schedule_memory_retry(
    mut batch: PendingBatch,
    status: &RuntimeStatusHandle,
    error_kind: TransportErrorKind,
    retry_batch: &mut Option<(PendingBatch, tokio::time::Instant)>,
) {
    batch.attempt_count = batch.attempt_count.saturating_add(1);
    let retry_at =
        tokio::time::Instant::now() + batch.next_retry_delay(error_kind, status.is_degraded_mode());
    *retry_batch = Some((batch, retry_at));
}

fn should_spool_after_failure(
    batch: &PendingBatch,
    status: &RuntimeStatusHandle,
    error_kind: TransportErrorKind,
    spool_enabled: bool,
) -> bool {
    if !spool_enabled {
        return false;
    }
    if status.is_degraded_mode() {
        return true;
    }
    if !matches!(error_kind, TransportErrorKind::TransientNetwork) {
        return true;
    }
    batch.attempt_count >= 2
}

async fn spool_in_memory_batch(
    batch: PendingBatch,
    state_writer: &StateWriterHandle,
    status: &RuntimeStatusHandle,
    spool_enabled: bool,
    compress_threshold_bytes: usize,
) -> AppResult<()> {
    if !spool_enabled {
        return Ok(());
    }
    let spool_stats = state_writer
        .spool_batch(batch.clone(), compress_threshold_bytes)
        .await?;
    status.update_spool_stats(spool_stats);
    for offset in &batch.source_offsets {
        status.record_source_durable_read(&offset.path, offset.file_key.clone(), offset.offset);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc, time::Duration};

    use async_trait::async_trait;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use crate::{
        config::{SourceConfig, StartAt},
        error::{AppError, AppResult},
        proto::{agent, ingest},
        runtime::{state_writer::spawn_state_writer, RuntimeStatusHandle},
        state::{SourceOffsetMarker, SqliteStateStore},
        transport::{
            AgentTransport, EnrollRequest, EnrollResponse, FetchPolicyRequest, PolicySnapshot,
        },
    };

    use super::spawn_sender;

    #[derive(Default)]
    struct AlwaysRejectTransport;

    #[async_trait]
    impl AgentTransport for AlwaysRejectTransport {
        async fn enroll(&self, _request: EnrollRequest) -> AppResult<EnrollResponse> {
            unreachable!()
        }

        async fn fetch_policy(&self, _request: FetchPolicyRequest) -> AppResult<PolicySnapshot> {
            unreachable!()
        }

        async fn send_heartbeat(&self, _payload: agent::HeartbeatPayload) -> AppResult<()> {
            unreachable!()
        }

        async fn send_batch(&self, _batch: ingest::LogBatch) -> AppResult<()> {
            Err(AppError::protocol("rejected"))
        }

        async fn send_diagnostics(&self, _payload: agent::DiagnosticsPayload) -> AppResult<()> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn spools_batch_after_non_transient_send_failure() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let (state_writer, state_writer_handle) =
            spawn_state_writer(store.clone(), dir.path().join("spool"), 64 * 1024 * 1024);
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
                source: "demo".to_string(),
                service: "svc".to_string(),
                severity_hint: "info".to_string(),
            }],
            &[],
            None,
        );
        let (tx, rx) = mpsc::channel(4);
        let shutdown = CancellationToken::new();
        let sender = spawn_sender(
            rx,
            Arc::new(AlwaysRejectTransport),
            state_writer,
            status.clone(),
            shutdown.clone(),
            true,
            32,
            1,
        );

        let batch = crate::batching::PendingBatch {
            batch_id: "batch-1".to_string(),
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
            approx_bytes: 256,
            source_offsets: vec![SourceOffsetMarker {
                source_id: "file:/tmp/demo.log".to_string(),
                path: "/tmp/demo.log".to_string(),
                file_key: Some("1:2".to_string()),
                offset: 50,
            }],
            created_at_unix_ms: 10,
            attempt_count: 0,
            from_spool: false,
            spool_payload_path: None,
            spool_codec: None,
        };

        status.record_send_queue_push(batch.approx_bytes);
        tx.send(batch).await.unwrap();
        drop(tx);

        tokio::time::sleep(Duration::from_millis(300)).await;
        let stats = store.spool_stats().unwrap();
        let offset = store
            .load_file_offset(std::path::Path::new("/tmp/demo.log"))
            .unwrap()
            .unwrap();

        shutdown.cancel();
        sender.await.unwrap().unwrap();
        state_writer_handle.await.unwrap().unwrap();

        assert_eq!(stats.batch_count, 1);
        assert_eq!(offset.read_offset, 50);
        assert_eq!(offset.acked_offset, 0);
    }
}
