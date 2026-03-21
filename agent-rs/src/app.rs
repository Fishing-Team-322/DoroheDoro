use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use rusqlite::OpenFlags;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    batching::spawn_batcher,
    config::{AgentConfig, TransportMode},
    error::AppResult,
    logging,
    metadata::{IdentityStatusSnapshot, RuntimeMetadataContext},
    runtime::{
        degraded::spawn_degraded_controller, diagnostics::spawn_diagnostics_worker,
        heartbeat::spawn_heartbeat_worker, sender::spawn_sender, state_writer::spawn_state_writer,
        RuntimeStaticContext, RuntimeStatusHandle,
    },
    sources::spawn_file_source,
    state::SqliteStateStore,
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
    metadata: RuntimeMetadataContext,
    hostname: String,
    version: String,
    agent_id: String,
}

impl App {
    pub async fn load(config_path: PathBuf) -> AppResult<Self> {
        let config = AgentConfig::load(&config_path)?;
        logging::init(&config.log_level)?;

        let hostname = resolve_hostname();
        let metadata = RuntimeMetadataContext::detect(&config, &config_path, &hostname)?;
        log_metadata_warnings(&metadata);

        let store = SqliteStateStore::new(&config.state_dir)?;
        let runtime_state = store.load_runtime_state()?;
        let persisted_offsets = store.list_file_offsets()?;
        let persisted_identity = store.load_identity()?;
        let version = metadata.build.agent_version.clone();
        let last_known_edge_url = runtime_state
            .last_known_edge_url
            .clone()
            .or_else(|| Some(config.edge_url.clone()));
        let identity_status = runtime_state
            .identity_status
            .clone()
            .map(|status| IdentityStatusSnapshot {
                status,
                reason: runtime_state.identity_status_reason.clone(),
            })
            .unwrap_or_default();

        let status = RuntimeStatusHandle::new(
            String::new(),
            hostname.clone(),
            version.clone(),
            config.transport.mode.as_str().to_string(),
            RuntimeStaticContext {
                metadata: metadata.clone(),
                state_db_exists: store.db_path().exists(),
                state_db_accessible: sqlite_accessible(store.db_path()),
                persisted_identity_present: persisted_identity.is_some(),
                last_known_edge_url,
                identity_status,
            },
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
        status.set_last_known_edge_url(Some(config.edge_url.clone()));
        status.update_spool_stats(store.spool_stats()?);

        let transport = build_transport(&config)?;
        let mut app = Self {
            config,
            store,
            transport,
            status,
            metadata,
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
            transport_mode = self.config.transport.mode.as_str(),
            install_mode = self.metadata.install.resolved_mode,
            target = self.metadata.build.target_triple,
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
                self.metadata.event_enrichment.clone(),
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
        let mut runtime_state = self.store.load_runtime_state()?;
        let persisted_identity = self.store.load_identity()?;
        let (mut agent_id, mut identity_status) = if let Some(identity) = persisted_identity {
            (
                identity.agent_id,
                IdentityStatusSnapshot {
                    status: "reused".to_string(),
                    reason: Some("persisted identity accepted".to_string()),
                },
            )
        } else {
            (
                self.enroll(None).await?,
                IdentityStatusSnapshot {
                    status: "newly_enrolled".to_string(),
                    reason: Some("no persisted identity was found".to_string()),
                },
            )
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
                identity_status = IdentityStatusSnapshot {
                    status: "re_enrolled".to_string(),
                    reason: Some("stored identity was rejected by the server".to_string()),
                };
                agent_id = self.enroll(Some(agent_id.clone())).await?;
                self.transport
                    .fetch_policy(FetchPolicyRequest {
                        agent_id: agent_id.clone(),
                        current_revision: None,
                    })
                    .await?
            }
            Err(error) => return Err(error),
        };

        runtime_state.applied_policy_revision = Some(policy.policy_revision.clone());
        // TODO: keep policy_body_json persisted for future cluster-aware server-issued metadata,
        // but do not parse it on the agent until the shared contract is explicitly fixed.
        runtime_state.policy_body_json = Some(policy.policy_body_json.clone());
        runtime_state.last_known_edge_url = Some(self.config.edge_url.clone());
        runtime_state.identity_status = Some(identity_status.status.clone());
        runtime_state.identity_status_reason = identity_status.reason.clone();
        runtime_state.degraded_mode = runtime_state.degraded_mode;
        runtime_state.blocked_delivery = runtime_state.blocked_delivery;
        runtime_state.blocked_reason = runtime_state.blocked_reason;
        runtime_state.spool_enabled = self.config.spool.enabled;
        runtime_state.updated_at_unix_ms = chrono::Utc::now().timestamp_millis();
        self.store.save_runtime_state(&runtime_state)?;

        info!(
            agent_id = agent_id,
            policy_agent_id = policy.agent_id,
            policy_id = policy.policy_id,
            policy_revision = policy.policy_revision,
            policy_status = policy.status,
            identity_status = identity_status.status,
            "agent policy synchronized"
        );
        self.status
            .set_policy_revision(Some(policy.policy_revision.clone()));
        self.status.set_identity_status(identity_status);
        self.status
            .set_last_known_edge_url(Some(self.config.edge_url.clone()));

        Ok(agent_id)
    }

    async fn enroll(&self, existing_agent_id: Option<String>) -> AppResult<String> {
        let response = self
            .transport
            .enroll(EnrollRequest {
                bootstrap_token: self.config.bootstrap_token.clone(),
                hostname: self.hostname.clone(),
                version: self.version.clone(),
                metadata: enrollment_metadata(&self.metadata, &self.config),
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

pub fn resolve_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "unknown-host".to_string())
}

fn enrollment_metadata(
    metadata: &RuntimeMetadataContext,
    config: &AgentConfig,
) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    values.insert("os_family".to_string(), metadata.platform.os_family.clone());
    values.insert("arch".to_string(), metadata.platform.architecture.clone());
    values.insert("edge_url".to_string(), config.edge_url.clone());
    values.insert(
        "transport_mode".to_string(),
        config.transport.mode.as_str().to_string(),
    );
    values.insert(
        "resolved_install_mode".to_string(),
        metadata.install.resolved_mode.clone(),
    );
    values.insert("build_id".to_string(), metadata.build.build_id.clone());
    values.insert(
        "systemd_detected".to_string(),
        metadata.platform.systemd_detected.to_string(),
    );
    if let Some(value) = metadata.platform.distro_name.clone() {
        values.insert("distro_name".to_string(), value);
    }
    if let Some(value) = metadata.platform.distro_version.clone() {
        values.insert("distro_version".to_string(), value);
    }
    if let Some(value) = metadata.platform.kernel_version.clone() {
        values.insert("kernel_version".to_string(), value);
    }
    if let Some(value) = metadata.platform.machine_id_hash.clone() {
        values.insert("machine_id_hash".to_string(), value);
    }
    if let Some(value) = metadata.cluster.configured_cluster_id.clone() {
        values.insert("cluster_id".to_string(), value);
    }
    if let Some(value) = metadata.cluster.cluster_name.clone() {
        values.insert("cluster_name".to_string(), value);
    }
    if let Some(value) = metadata.cluster.service_name.clone() {
        values.insert("service_name".to_string(), value);
    }
    if let Some(value) = metadata.cluster.environment.clone() {
        values.insert("environment".to_string(), value);
    }
    values
}

fn sqlite_accessible(path: &std::path::Path) -> bool {
    rusqlite::Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).is_ok()
}

fn log_metadata_warnings(metadata: &RuntimeMetadataContext) {
    for warning in &metadata.install.warnings {
        warn!(warning = %warning, "install layout warning");
    }
    for warning in &metadata.compatibility.warnings {
        warn!(warning = %warning, "compatibility warning");
    }
    for issue in &metadata.compatibility.source_path_issues {
        warn!(issue = %issue, "source path note");
    }
}
