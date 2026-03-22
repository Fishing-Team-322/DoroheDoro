use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    batching::PendingBatch,
    error::{AppError, AppResult, TransportErrorKind},
    runtime::{
        state_writer::{RuntimeFlagsUpdate, StateWriterHandle},
        RuntimeStatusHandle,
    },
    state::RuntimeStatePatch,
    transport::AgentTransport,
};

const SPOOL_BURST: usize = 3;
const BLOCKED_DELIVERY_RECHECK: Duration = Duration::from_secs(60);

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
        let mut spool_burst_count = 0usize;
        let mut blocked_retry_at = status
            .is_blocked_delivery()
            .then(|| tokio::time::Instant::now() + BLOCKED_DELIVERY_RECHECK);

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

            if status.is_blocked_delivery() {
                if blocked_retry_at.is_none() {
                    blocked_retry_at = Some(tokio::time::Instant::now() + BLOCKED_DELIVERY_RECHECK);
                }

                if let Some(deadline) = blocked_retry_at {
                    if tokio::time::Instant::now() < deadline {
                        if spool_enabled {
                            if let Some((batch, _)) = retry_batch.take() {
                                if let Err(spool_error) = spool_in_memory_batch(
                                    batch.clone(),
                                    &state_writer,
                                    &status,
                                    spool_enabled,
                                    compress_threshold_bytes,
                                )
                                .await
                                {
                                    error!(
                                        batch_id = batch.batch_id,
                                        error = %spool_error,
                                        "failed to spool blocked in-memory batch"
                                    );
                                    retry_batch = Some((batch, deadline));
                                }
                            }

                            tokio::select! {
                                _ = shutdown.cancelled(), if !shutdown_requested => {
                                    shutdown_requested = true;
                                }
                                maybe_batch = rx.recv() => {
                                    match maybe_batch {
                                        Some(batch) => {
                                            status.record_send_queue_pop(batch.approx_bytes);
                                            if let Err(spool_error) = spool_in_memory_batch(
                                                batch.clone(),
                                                &state_writer,
                                                &status,
                                                spool_enabled,
                                                compress_threshold_bytes,
                                            )
                                            .await
                                            {
                                                error!(
                                                    batch_id = batch.batch_id,
                                                    error = %spool_error,
                                                    "failed to spool live batch while delivery is blocked"
                                                );
                                                schedule_memory_retry(
                                                    batch,
                                                    &status,
                                                    TransportErrorKind::Unknown,
                                                    &mut retry_batch,
                                                );
                                            }
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
                                _ = tokio::time::sleep_until(deadline) => {}
                            }
                        } else {
                            tokio::select! {
                                _ = shutdown.cancelled(), if !shutdown_requested => {
                                    shutdown_requested = true;
                                }
                                _ = tokio::time::sleep_until(deadline) => {}
                            }
                        }
                        continue;
                    }
                }
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
                        &mut blocked_retry_at,
                        shutdown_requested,
                    )
                    .await?;
                    spool_burst_count = 0;
                    continue;
                }
                retry_batch = Some((batch, deadline));
            }

            if spool_burst_count >= SPOOL_BURST {
                if let Some(batch) = try_recv_live_batch(&mut rx, &status) {
                    process_batch(
                        batch,
                        &transport,
                        &state_writer,
                        &status,
                        spool_enabled,
                        compress_threshold_bytes,
                        &mut retry_batch,
                        &mut blocked_retry_at,
                        shutdown_requested,
                    )
                    .await?;
                    spool_burst_count = 0;
                    continue;
                }
                spool_burst_count = 0;
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
                    &mut blocked_retry_at,
                    shutdown_requested,
                )
                .await?;
                spool_burst_count = spool_burst_count.saturating_add(1);
                continue;
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
                                &mut blocked_retry_at,
                                shutdown_requested,
                            ).await?;
                            spool_burst_count = 0;
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
    blocked_retry_at: &mut Option<tokio::time::Instant>,
    shutdown_requested: bool,
) -> AppResult<()> {
    match transport.send_batch(batch.batch.clone()).await {
        Ok(()) => {
            let now = Utc::now().timestamp_millis();
            status.record_send_success(now);
            let spool_stats = state_writer
                .ack_batch(
                    batch.clone(),
                    success_runtime_flags(status.is_degraded_mode(), spool_enabled, now),
                )
                .await?;
            state_writer
                .update_runtime_state(RuntimeStatePatch {
                    last_handshake_success_at_unix_ms: Some(Some(now)),
                    last_connect_error: Some(None),
                    last_tls_error: Some(None),
                    ..RuntimeStatePatch::default()
                })
                .await?;
            status.update_spool_stats(spool_stats);
            *blocked_retry_at = None;
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
            status.record_connectivity_error(&error);
            state_writer
                .update_runtime_state(connectivity_error_patch(&error))
                .await?;

            if is_permanent_transport_failure(error_kind) {
                let blocked_reason = format!("permanent transport failure: {error_kind} ({error})");
                status.set_blocked_delivery(true, Some(blocked_reason.clone()));
                status.set_degraded_mode(true, Some(blocked_reason.clone()));
                state_writer
                    .update_runtime_flags(current_runtime_flags(status, spool_enabled))
                    .await?;
                *blocked_retry_at = Some(tokio::time::Instant::now() + BLOCKED_DELIVERY_RECHECK);

                if batch.from_spool {
                    warn!(
                        batch_id = batch.batch_id,
                        error = %error,
                        error_kind = %error_kind,
                        "spooled batch hit permanent transport failure, delivery blocked"
                    );
                    return Ok(());
                }

                if spool_enabled {
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
                            "failed to spool live batch after permanent transport failure"
                        );
                        schedule_memory_retry(batch, status, error_kind, retry_batch);
                    }
                } else {
                    schedule_memory_retry(batch, status, error_kind, retry_batch);
                }

                warn!(
                    error = %error,
                    error_kind = %error_kind,
                    "sender entered blocked-delivery mode after permanent transport failure"
                );
                return Ok(());
            }

            if status.is_blocked_delivery() {
                *blocked_retry_at = Some(tokio::time::Instant::now() + BLOCKED_DELIVERY_RECHECK);
            }

            state_writer
                .update_runtime_flags(current_runtime_flags(status, spool_enabled))
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

fn try_recv_live_batch(
    rx: &mut mpsc::Receiver<PendingBatch>,
    status: &RuntimeStatusHandle,
) -> Option<PendingBatch> {
    match rx.try_recv() {
        Ok(batch) => {
            status.record_send_queue_pop(batch.approx_bytes);
            Some(batch)
        }
        Err(_) => None,
    }
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

fn is_permanent_transport_failure(error_kind: TransportErrorKind) -> bool {
    matches!(
        error_kind,
        TransportErrorKind::Unauthorized
            | TransportErrorKind::ServerRejected
            | TransportErrorKind::SerializationError
    )
}

fn current_runtime_flags(status: &RuntimeStatusHandle, spool_enabled: bool) -> RuntimeFlagsUpdate {
    RuntimeFlagsUpdate {
        degraded_mode: status.is_degraded_mode(),
        blocked_delivery: status.is_blocked_delivery(),
        blocked_reason: status.blocked_reason(),
        spool_enabled,
        consecutive_send_failures: status.current_consecutive_failures(),
        last_successful_send_at_unix_ms: None,
    }
}

fn success_runtime_flags(degraded_mode: bool, spool_enabled: bool, now: i64) -> RuntimeFlagsUpdate {
    RuntimeFlagsUpdate {
        degraded_mode,
        blocked_delivery: false,
        blocked_reason: None,
        spool_enabled,
        consecutive_send_failures: 0,
        last_successful_send_at_unix_ms: Some(now),
    }
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

fn connectivity_error_patch(error: &AppError) -> RuntimeStatePatch {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("tls")
        || lower.contains("certificate")
        || lower.contains("rustls")
        || lower.contains("ssl")
    {
        RuntimeStatePatch {
            last_tls_error: Some(Some(message)),
            ..RuntimeStatePatch::default()
        }
    } else {
        RuntimeStatePatch {
            last_connect_error: Some(Some(message)),
            ..RuntimeStatePatch::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        time::Duration,
    };

    use async_trait::async_trait;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use crate::{
        config::{SourceConfig, StartAt},
        error::{AppError, AppResult},
        proto::{agent, ingest},
        runtime::{state_writer::spawn_state_writer, test_static_context, RuntimeStatusHandle},
        state::{
            encode_spool_payload, write_spool_payload, SourceOffsetMarker, SpoolBatchRecord,
            SqliteStateStore,
        },
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

    #[derive(Clone)]
    struct OrderedTransport {
        results: Arc<Mutex<Vec<AppResult<()>>>>,
        seen_messages: Arc<Mutex<Vec<String>>>,
    }

    impl OrderedTransport {
        fn new(results: Vec<AppResult<()>>) -> Self {
            Self {
                results: Arc::new(Mutex::new(results.into_iter().rev().collect())),
                seen_messages: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn seen_messages(&self) -> Vec<String> {
            self.seen_messages
                .lock()
                .expect("transport log lock")
                .clone()
        }
    }

    #[async_trait]
    impl AgentTransport for OrderedTransport {
        async fn enroll(&self, _request: EnrollRequest) -> AppResult<EnrollResponse> {
            unreachable!()
        }

        async fn fetch_policy(&self, _request: FetchPolicyRequest) -> AppResult<PolicySnapshot> {
            unreachable!()
        }

        async fn send_heartbeat(&self, _payload: agent::HeartbeatPayload) -> AppResult<()> {
            unreachable!()
        }

        async fn send_batch(&self, batch: ingest::LogBatch) -> AppResult<()> {
            let message = batch
                .events
                .first()
                .map(|event| event.message.clone())
                .unwrap_or_default();
            self.seen_messages
                .lock()
                .expect("transport log lock")
                .push(message);
            self.results
                .lock()
                .expect("transport result lock")
                .pop()
                .unwrap_or(Ok(()))
        }

        async fn send_diagnostics(&self, _payload: agent::DiagnosticsPayload) -> AppResult<()> {
            unreachable!()
        }
    }

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

    fn make_batch(
        batch_id: &str,
        path: &str,
        offset: u64,
        message: &str,
    ) -> crate::batching::PendingBatch {
        crate::batching::PendingBatch {
            batch_id: batch_id.to_string(),
            batch: ingest::LogBatch {
                agent_id: "agent-1".to_string(),
                host: "demo-host".to_string(),
                sent_at_unix_ms: 10,
                events: vec![ingest::LogEvent {
                    timestamp_unix_ms: 10,
                    message: message.to_string(),
                    source: "demo".to_string(),
                    source_type: "file".to_string(),
                    service: "svc".to_string(),
                    severity: "info".to_string(),
                    labels: Default::default(),
                    raw: message.to_string(),
                }],
            },
            approx_bytes: 256,
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
    async fn spools_batch_after_non_transient_send_failure() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let (state_writer, state_writer_handle) =
            spawn_state_writer(store.clone(), dir.path().join("spool"), 64 * 1024 * 1024);
        let status = test_status();
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

        let batch = make_batch("batch-1", "/tmp/demo.log", 50, "hello");

        status.record_send_queue_push(batch.approx_bytes);
        tx.send(batch).await.unwrap();
        drop(tx);

        tokio::time::sleep(Duration::from_millis(300)).await;
        let stats = store.spool_stats().unwrap();
        let offset = store
            .load_file_offset(Path::new("/tmp/demo.log"))
            .unwrap()
            .unwrap();

        shutdown.cancel();
        sender.await.unwrap().unwrap();
        state_writer_handle.await.unwrap().unwrap();

        assert_eq!(stats.batch_count, 1);
        assert_eq!(offset.durable_read_offset, 50);
        assert_eq!(offset.acked_offset, 0);
    }

    #[tokio::test]
    async fn blocks_spool_retry_on_permanent_failure() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let (state_writer, state_writer_handle) =
            spawn_state_writer(store.clone(), dir.path().join("spool"), 64 * 1024 * 1024);
        let status = test_status();
        let batch = make_batch("spool-batch-1", "/tmp/demo.log", 50, "spooled");
        let (codec, payload) = encode_spool_payload(&batch.batch, usize::MAX).unwrap();
        let payload =
            write_spool_payload(&dir.path().join("spool"), &batch.batch_id, &codec, &payload)
                .unwrap();
        store
            .insert_spool_batch(&SpoolBatchRecord {
                batch_id: batch.batch_id.clone(),
                payload_path: payload,
                codec,
                created_at_unix_ms: batch.created_at_unix_ms,
                attempt_count: 0,
                next_retry_at_unix_ms: 0,
                approx_bytes: batch.approx_bytes,
                source_offsets: batch.source_offsets.clone(),
            })
            .unwrap();

        let shutdown = CancellationToken::new();
        let (_tx, rx) = mpsc::channel(4);
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

        tokio::time::sleep(Duration::from_millis(300)).await;
        let runtime = store.load_runtime_state().unwrap();
        let spooled = store.load_due_spool_batch(i64::MAX).unwrap().unwrap();

        shutdown.cancel();
        sender.await.unwrap().unwrap();
        state_writer_handle.await.unwrap().unwrap();

        assert!(runtime.blocked_delivery);
        assert!(status.snapshot().blocked_delivery);
        assert_eq!(spooled.attempt_count, 0);
        assert_eq!(spooled.next_retry_at_unix_ms, 0);
    }

    #[tokio::test]
    async fn interleaves_live_batches_with_spool_backlog() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let (state_writer, state_writer_handle) =
            spawn_state_writer(store.clone(), dir.path().join("spool"), 64 * 1024 * 1024);
        let status = test_status();

        for index in 0..4 {
            let batch = make_batch(
                &format!("spool-{index}"),
                &format!("/tmp/spool-{index}.log"),
                10 + index as u64,
                &format!("spool-{index}"),
            );
            let (codec, payload) = encode_spool_payload(&batch.batch, usize::MAX).unwrap();
            let payload =
                write_spool_payload(&dir.path().join("spool"), &batch.batch_id, &codec, &payload)
                    .unwrap();
            store
                .insert_spool_batch(&SpoolBatchRecord {
                    batch_id: batch.batch_id.clone(),
                    payload_path: payload,
                    codec,
                    created_at_unix_ms: index as i64,
                    attempt_count: 0,
                    next_retry_at_unix_ms: 0,
                    approx_bytes: batch.approx_bytes,
                    source_offsets: batch.source_offsets.clone(),
                })
                .unwrap();
        }

        let transport = OrderedTransport::new(vec![Ok(()), Ok(()), Ok(()), Ok(()), Ok(())]);
        let transport_handle = transport.clone();
        let (tx, rx) = mpsc::channel(4);
        let shutdown = CancellationToken::new();
        let live_batch = make_batch("live-1", "/tmp/live.log", 99, "live-1");
        status.record_send_queue_push(live_batch.approx_bytes);
        tx.send(live_batch).await.unwrap();
        drop(tx);
        let sender = spawn_sender(
            rx,
            Arc::new(transport),
            state_writer,
            status.clone(),
            shutdown.clone(),
            true,
            32,
            1,
        );

        tokio::time::sleep(Duration::from_millis(600)).await;
        shutdown.cancel();
        sender.await.unwrap().unwrap();
        state_writer_handle.await.unwrap().unwrap();

        assert_eq!(
            transport_handle.seen_messages(),
            vec![
                "spool-0".to_string(),
                "spool-1".to_string(),
                "spool-2".to_string(),
                "live-1".to_string(),
                "spool-3".to_string(),
            ]
        );
    }
}
