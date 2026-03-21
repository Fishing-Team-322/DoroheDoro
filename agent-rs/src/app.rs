use std::{collections::BTreeMap, sync::Arc};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    batching::spawn_batcher,
    cli::Cli,
    config::{AgentConfig, TransportMode},
    error::AppResult,
    logging,
    runtime::{
        degraded::spawn_degraded_controller, diagnostics::spawn_diagnostics_worker,
        heartbeat::spawn_heartbeat_worker, sender::spawn_sender, state_writer::spawn_state_writer,
        RuntimeStatusHandle,
    },
    sources::spawn_file_source,
    state::{RuntimeStateRecord, SqliteStateStore},
    transport::{
        AgentTransport, DynTransport, EdgeGrpcTransport, EnrollRequest, FetchPolicyRequest,
        MockTransport,
    },
};

const SOURCE_COUNT_WARN_THRESHOLD: usize = 64;

pub struct App {
    config: AgentConfig,
    store: SqliteStateStore,
    transport: DynTransport,
    status: RuntimeStatusHandle,
    hostname: String,
    version: String,
    agent_id: String,
}

impl App {
    pub async fn load(cli: Cli) -> AppResult<Self> {
        let config = AgentConfig::load(&cli.config)?;
        logging::init(&config.log_level)?;

        let store = SqliteStateStore::new(&config.state_dir)?;
        let runtime_state = store.load_runtime_state()?;
        let persisted_offsets = store.list_file_offsets()?;
        let hostname = resolve_hostname();
        let version = env!("CARGO_PKG_VERSION").to_string();
        let status = RuntimeStatusHandle::new(
            String::new(),
            hostname.clone(),
            version.clone(),
            transport_mode_label(&config.transport.mode),
            config.spool.enabled,
            &config.sources,
            &persisted_offsets,
            runtime_state.last_successful_send_at_unix_ms,
        );
        if runtime_state.degraded_mode {
            status.set_degraded_mode(true, Some("persisted runtime state".to_string()));
        }
        if runtime_state.blocked_delivery {
            let reason = runtime_state
                .blocked_reason
                .clone()
                .or_else(|| Some("persisted blocked delivery state".to_string()));
            status.set_blocked_delivery(true, reason.clone());
            status.set_degraded_mode(true, reason);
        }
        if let Some(revision) = runtime_state.applied_policy_revision.clone() {
            status.set_policy_revision(Some(revision));
        }
        status.update_spool_stats(store.spool_stats()?);

        let transport = build_transport(&config)?;
        let mut app = Self {
            config,
            store,
            transport,
            status,
            hostname,
            version,
            agent_id: String::new(),
        };
        app.agent_id = app.ensure_identity_and_policy().await?;
        app.status.set_agent_id(app.agent_id.clone());
        Ok(app)
    }

    pub async fn run(self) -> AppResult<()> {
        info!(
            agent_id = self.agent_id,
            state_db = %self.store.db_path().display(),
            spool_dir = %self.config.spool.dir.display(),
            transport_mode = ?self.config.transport.mode,
            "starting doro-agent"
        );

        let shutdown = CancellationToken::new();
        let (event_tx, event_rx) = mpsc::channel(self.config.queues.event_capacity);
        let (send_tx, send_rx) = mpsc::channel(self.config.queues.send_capacity);

        let (state_writer, state_writer_handle) = spawn_state_writer(
            self.store.clone(),
            self.config.spool.dir.clone(),
            self.config.spool.max_disk_bytes,
        );

        let batcher = spawn_batcher(
            event_rx,
            send_tx.clone(),
            state_writer.clone(),
            self.status.clone(),
            shutdown.clone(),
            self.config.batch.clone(),
            self.config.queues.clone(),
            self.config.spool.clone(),
            self.agent_id.clone(),
            self.hostname.clone(),
        );

        let sender = spawn_sender(
            send_rx,
            self.transport.clone(),
            state_writer.clone(),
            self.status.clone(),
            shutdown.clone(),
            self.config.spool.enabled,
            self.config.batch.compress_threshold_bytes,
            self.config.degraded.shutdown_spool_grace_sec,
        );

        let heartbeat = spawn_heartbeat_worker(
            self.transport.clone(),
            self.status.clone(),
            shutdown.clone(),
            self.config.edge_url.clone(),
            self.config.heartbeat.interval_sec,
        );

        let diagnostics = spawn_diagnostics_worker(
            self.transport.clone(),
            self.status.clone(),
            shutdown.clone(),
            self.config.diagnostics.interval_sec,
        );

        let degraded = spawn_degraded_controller(
            self.status.clone(),
            state_writer.clone(),
            shutdown.clone(),
            self.config.degraded.clone(),
            self.config.queues.clone(),
            self.config.spool.enabled,
            self.config.spool.max_disk_bytes,
        );

        let mut source_handles = Vec::new();
        if self.config.sources.is_empty() {
            warn!("agent started without configured sources");
        } else if self.config.sources.len() > SOURCE_COUNT_WARN_THRESHOLD {
            warn!(
                source_count = self.config.sources.len(),
                threshold = SOURCE_COUNT_WARN_THRESHOLD,
                "high source count configured; current runtime uses one reader task per source"
            );
        }
        for source in self.config.sources.clone() {
            source_handles.push(spawn_file_source(
                source,
                self.config.queues.clone(),
                self.store.clone(),
                self.status.clone(),
                event_tx.clone(),
                shutdown.clone(),
            ));
        }
        drop(event_tx);

        tokio::signal::ctrl_c().await?;
        info!("shutdown signal received");
        shutdown.cancel();

        for handle in source_handles {
            let _ = handle.await;
        }
        drop(send_tx);

        let _ = batcher.await;
        let _ = sender.await??;
        let _ = degraded.await??;
        let _ = heartbeat.await??;
        let _ = diagnostics.await??;
        let _ = state_writer_handle.await??;

        info!("doro-agent stopped");
        Ok(())
    }

    async fn ensure_identity_and_policy(&mut self) -> AppResult<String> {
        let runtime_state = self.store.load_runtime_state()?;
        let mut agent_id = match self.store.load_identity()? {
            Some(identity) => identity.agent_id,
            None => self.enroll(None).await?,
        };

        let policy = match self
            .transport
            .fetch_policy(FetchPolicyRequest {
                agent_id: agent_id.clone(),
                current_revision: runtime_state.applied_policy_revision.clone(),
            })
            .await
        {
            Ok(policy) => policy,
            Err(error) if error.is_identity_error() => {
                warn!(error = %error, "stored agent identity was rejected, re-enrolling");
                agent_id = self.enroll(None).await?;
                self.transport
                    .fetch_policy(FetchPolicyRequest {
                        agent_id: agent_id.clone(),
                        current_revision: None,
                    })
                    .await?
            }
            Err(error) => return Err(error),
        };

        self.store.save_runtime_state(&RuntimeStateRecord {
            applied_policy_revision: Some(policy.policy_revision.clone()),
            policy_body_json: Some(policy.policy_body_json),
            last_successful_send_at_unix_ms: runtime_state.last_successful_send_at_unix_ms,
            last_known_edge_url: Some(self.config.edge_url.clone()),
            degraded_mode: runtime_state.degraded_mode,
            blocked_delivery: runtime_state.blocked_delivery,
            blocked_reason: runtime_state.blocked_reason,
            spool_enabled: self.config.spool.enabled,
            consecutive_send_failures: runtime_state.consecutive_send_failures,
            updated_at_unix_ms: chrono::Utc::now().timestamp_millis(),
        })?;
        info!(
            agent_id = agent_id,
            policy_agent_id = policy.agent_id,
            policy_id = policy.policy_id,
            policy_revision = policy.policy_revision,
            policy_status = policy.status,
            "agent policy synchronized"
        );
        self.status
            .set_policy_revision(Some(policy.policy_revision.clone()));

        Ok(agent_id)
    }

    async fn enroll(&self, existing_agent_id: Option<String>) -> AppResult<String> {
        let response = self
            .transport
            .enroll(EnrollRequest {
                bootstrap_token: self.config.bootstrap_token.clone(),
                hostname: self.hostname.clone(),
                version: self.version.clone(),
                metadata: enrollment_metadata(&self.config.transport.mode, &self.config.edge_url),
                existing_agent_id,
            })
            .await?;

        self.store
            .save_identity(&response.agent_id, &self.hostname, &self.version)?;
        info!(
            agent_id = response.agent_id,
            enrollment_status = response.status,
            "agent enrolled"
        );
        Ok(response.agent_id)
    }
}

fn build_transport(config: &AgentConfig) -> AppResult<Arc<dyn AgentTransport>> {
    match config.transport.mode {
        TransportMode::Mock => Ok(Arc::new(MockTransport::default())),
        TransportMode::Edge => Ok(Arc::new(EdgeGrpcTransport::new(
            &config.edge_url,
            &config.edge_grpc_addr,
        )?)),
    }
}

fn resolve_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "unknown-host".to_string())
}

fn transport_mode_label(mode: &TransportMode) -> String {
    match mode {
        TransportMode::Edge => "edge".to_string(),
        TransportMode::Mock => "mock".to_string(),
    }
}

fn enrollment_metadata(mode: &TransportMode, edge_url: &str) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("os".to_string(), std::env::consts::OS.to_string());
    metadata.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    metadata.insert("edge_url".to_string(), edge_url.to_string());
    metadata.insert("transport_mode".to_string(), transport_mode_label(mode));
    metadata
}
