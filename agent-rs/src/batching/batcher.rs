use std::{collections::BTreeMap, sync::Arc, time::Duration};

use chrono::Utc;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    config::BatchConfig,
    proto::ingest,
    runtime::RuntimeStatusHandle,
    sources::SourceEvent,
    state::{FileOffsetUpdate, RuntimeStateRecord, SqliteStateStore},
    transport::AgentTransport,
};

pub fn spawn_batcher(
    mut rx: mpsc::Receiver<SourceEvent>,
    transport: Arc<dyn AgentTransport>,
    store: SqliteStateStore,
    status: RuntimeStatusHandle,
    shutdown: CancellationToken,
    batch_config: BatchConfig,
    agent_id: String,
    host: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut buffer = Vec::<SourceEvent>::new();
        let mut batch_started_at = None;
        let mut next_retry_at = None;
        let mut retry_delay = Duration::from_secs(1);
        let mut ticker = tokio::time::interval(Duration::from_millis(200));
        let mut shutdown_requested = false;

        loop {
            tokio::select! {
                _ = shutdown.cancelled(), if !shutdown_requested => {
                    shutdown_requested = true;
                }
                maybe_event = rx.recv() => {
                    match maybe_event {
                        Some(event) => {
                            if batch_started_at.is_none() {
                                batch_started_at = Some(tokio::time::Instant::now());
                            }
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
                || batch_started_at
                    .map(|started| {
                        started.elapsed() >= Duration::from_secs(batch_config.flush_interval_sec)
                    })
                    .unwrap_or(false);
            let retry_due = next_retry_at
                .map(|deadline| tokio::time::Instant::now() >= deadline)
                .unwrap_or(false);
            let closing = shutdown_requested && rx.is_closed();

            if !flush_due && !retry_due && !closing {
                continue;
            }

            let batch = ingest::LogBatch {
                agent_id: agent_id.clone(),
                host: host.clone(),
                events: buffer.iter().map(|item| item.event.clone()).collect(),
                sent_at_unix_ms: Utc::now().timestamp_millis(),
            };

            match transport.send_batch(batch).await {
                Ok(()) => {
                    let updates = checkpoint_updates(&buffer);
                    if let Err(error) = store.commit_file_offsets(&updates) {
                        status.record_error(format!("offset commit failed: {error}"));
                        error!(error = %error, "failed to persist committed offsets");
                    } else {
                        for update in &updates {
                            status.record_source_commit(
                                &update.path,
                                update.file_key.clone(),
                                update.offset,
                            );
                        }
                    }

                    let now = Utc::now().timestamp_millis();
                    let runtime_state = match store.load_runtime_state() {
                        Ok(mut state) => {
                            state.last_successful_send_at_unix_ms = Some(now);
                            state.updated_at_unix_ms = now;
                            state
                        }
                        Err(error) => {
                            warn!(error = %error, "failed to load runtime state before batch save");
                            RuntimeStateRecord {
                                applied_policy_revision: None,
                                policy_body_json: None,
                                last_successful_send_at_unix_ms: Some(now),
                                last_known_edge_url: None,
                                updated_at_unix_ms: now,
                            }
                        }
                    };

                    if let Err(error) = store.save_runtime_state(&runtime_state) {
                        status.record_error(format!("runtime state save failed: {error}"));
                        error!(error = %error, "failed to save runtime state");
                    }

                    status.record_last_send(now);
                    info!(accepted = buffer.len(), "sent log batch");
                    buffer.clear();
                    batch_started_at = None;
                    next_retry_at = None;
                    retry_delay = Duration::from_secs(1);
                }
                Err(error) => {
                    status.record_error(format!("batch send failed: {error}"));
                    warn!(error = %error, size = buffer.len(), "failed to send log batch");
                    next_retry_at = Some(tokio::time::Instant::now() + retry_delay);
                    retry_delay = retry_delay.mul_f32(2.0).min(Duration::from_secs(15));
                    if closing {
                        return;
                    }
                }
            }
        }
    })
}

fn checkpoint_updates(buffer: &[SourceEvent]) -> Vec<FileOffsetUpdate> {
    let mut updates = BTreeMap::<String, FileOffsetUpdate>::new();
    for item in buffer {
        updates.insert(
            item.checkpoint.path.clone(),
            FileOffsetUpdate {
                path: item.checkpoint.path.clone(),
                file_key: item.checkpoint.file_key.clone(),
                offset: item.checkpoint.offset,
            },
        );
    }
    updates.into_values().collect()
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use async_trait::async_trait;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use crate::{
        config::{BatchConfig, SourceConfig},
        proto::{agent, ingest},
        runtime::RuntimeStatusHandle,
        sources::{SourceCheckpoint, SourceEvent},
        state::SqliteStateStore,
        transport::{
            AgentTransport, EnrollRequest, EnrollResponse, FetchPolicyRequest, PolicySnapshot,
        },
    };

    use super::spawn_batcher;

    #[derive(Default)]
    struct CountingTransport {
        sent_batches: Mutex<Vec<usize>>,
    }

    #[async_trait]
    impl AgentTransport for CountingTransport {
        async fn enroll(&self, _request: EnrollRequest) -> crate::error::AppResult<EnrollResponse> {
            unreachable!()
        }

        async fn fetch_policy(
            &self,
            _request: FetchPolicyRequest,
        ) -> crate::error::AppResult<PolicySnapshot> {
            unreachable!()
        }

        async fn send_heartbeat(
            &self,
            _payload: agent::HeartbeatPayload,
        ) -> crate::error::AppResult<()> {
            unreachable!()
        }

        async fn send_batch(&self, batch: ingest::LogBatch) -> crate::error::AppResult<()> {
            self.sent_batches.lock().unwrap().push(batch.events.len());
            Ok(())
        }

        async fn send_diagnostics(
            &self,
            _payload: agent::DiagnosticsPayload,
        ) -> crate::error::AppResult<()> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn flushes_by_event_count() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let status = RuntimeStatusHandle::new(
            "demo-host".to_string(),
            "0.1.0".to_string(),
            &[SourceConfig {
                kind: "file".to_string(),
                path: PathBuf::from("/tmp/demo.log"),
                source: "demo".to_string(),
                service: "svc".to_string(),
                severity_hint: "info".to_string(),
            }],
            &[],
        );
        let transport = Arc::new(CountingTransport::default());
        let (tx, rx) = mpsc::channel(8);
        let shutdown = CancellationToken::new();

        let handle = spawn_batcher(
            rx,
            transport.clone(),
            store,
            status,
            shutdown.clone(),
            BatchConfig {
                max_events: 2,
                flush_interval_sec: 60,
            },
            "agent-1".to_string(),
            "demo-host".to_string(),
        );

        for offset in [1_u64, 2_u64] {
            tx.send(SourceEvent {
                checkpoint: SourceCheckpoint {
                    path: "/tmp/demo.log".to_string(),
                    file_key: Some("1:2".to_string()),
                    offset,
                },
                event: ingest::LogEvent {
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

        tokio::time::sleep(Duration::from_millis(500)).await;
        shutdown.cancel();
        drop(tx);
        handle.await.unwrap();

        assert_eq!(transport.sent_batches.lock().unwrap().as_slice(), &[2]);
    }
}
