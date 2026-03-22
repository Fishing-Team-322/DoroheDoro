use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use chrono::Utc;
use rusqlite::OpenFlags;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    batching::spawn_batcher,
    config::{AgentConfig, SourceConfig, TransportMode},
    error::{AppError, AppResult},
    logging,
    metadata::{IdentityStatusSnapshot, RuntimeMetadataContext},
    policy::parse_file_sources,
    runtime::{
        degraded::spawn_degraded_controller, diagnostics::spawn_diagnostics_worker,
        heartbeat::spawn_heartbeat_worker, sender::spawn_sender, state_writer::spawn_state_writer,
        ConnectivityStaticContext, RuntimePhase, RuntimeStaticContext, RuntimeStatusHandle,
    },
    security::{spawn_security_scan_worker, SecurityPostureStatusSnapshot},
    sources::{spawn_file_source, SourceEvent},
    state::{RuntimeStatePatch, RuntimeStateRecord, SqliteStateStore},
    transport::{
        client::{build_base_url, derive_server_name, endpoint_uses_tls},
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
    active_sources: Vec<SourceConfig>,
}

struct SourceWorker {
    config: SourceConfig,
    shutdown: CancellationToken,
    handle: JoinHandle<()>,
}

struct SourceSupervisor {
    queue_config: crate::config::QueueConfig,
    enrichment: crate::metadata::EventEnrichmentContext,
    store: SqliteStateStore,
    status: RuntimeStatusHandle,
    event_tx: mpsc::Sender<SourceEvent>,
    shutdown: CancellationToken,
    workers: BTreeMap<String, SourceWorker>,
}

struct FetchedPolicy {
    resolved_agent_id: String,
    identity_status: IdentityStatusSnapshot,
    policy_id: String,
    policy_revision: String,
    policy_body_json: String,
    policy_status: String,
    sources: Vec<SourceConfig>,
    fetched_at_unix_ms: i64,
}

impl SourceSupervisor {
    fn new(
        queue_config: crate::config::QueueConfig,
        enrichment: crate::metadata::EventEnrichmentContext,
        store: SqliteStateStore,
        status: RuntimeStatusHandle,
        event_tx: mpsc::Sender<SourceEvent>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            queue_config,
            enrichment,
            store,
            status,
            event_tx,
            shutdown,
            workers: BTreeMap::new(),
        }
    }

    async fn reconcile(&mut self, desired_sources: Vec<SourceConfig>) -> AppResult<()> {
        log_source_shape(&desired_sources);

        let persisted_offsets = self.store.list_file_offsets()?;
        self.status
            .set_configured_sources(&desired_sources, &persisted_offsets);

        let desired_by_path = desired_sources
            .into_iter()
            .map(|source| (source.path.to_string_lossy().into_owned(), source))
            .collect::<BTreeMap<_, _>>();

        let paths_to_stop = self
            .workers
            .iter()
            .filter_map(|(path, worker)| match desired_by_path.get(path) {
                Some(source) if worker.config == *source => None,
                _ => Some(path.clone()),
            })
            .collect::<Vec<_>>();

        for path in paths_to_stop {
            if let Some(worker) = self.workers.remove(&path) {
                worker.shutdown.cancel();
                let _ = worker.handle.await;
            }
        }

        for (path, source) in desired_by_path {
            if self.workers.contains_key(&path) {
                continue;
            }

            let source_shutdown = self.shutdown.child_token();
            let handle = spawn_file_source(
                source.clone(),
                self.queue_config.clone(),
                self.enrichment.clone(),
                self.store.clone(),
                self.status.clone(),
                self.event_tx.clone(),
                source_shutdown.clone(),
            );
            self.workers.insert(
                path,
                SourceWorker {
                    config: source,
                    shutdown: source_shutdown,
                    handle,
                },
            );
        }

        Ok(())
    }

    async fn shutdown(&mut self) {
        let workers = std::mem::take(&mut self.workers);
        for (_, worker) in workers {
            worker.shutdown.cancel();
            let _ = worker.handle.await;
        }
    }

    async fn poll_workers(&mut self) {
        let finished_paths = self
            .workers
            .iter()
            .filter_map(|(path, worker)| worker.handle.is_finished().then_some(path.clone()))
            .collect::<Vec<_>>();

        for path in finished_paths {
            let Some(worker) = self.workers.remove(&path) else {
                continue;
            };

            match worker.handle.await {
                Ok(()) => warn!(
                    path = %path,
                    source_id = worker.config.source_id(),
                    "source worker exited unexpectedly, restarting"
                ),
                Err(error) => warn!(
                    path = %path,
                    source_id = worker.config.source_id(),
                    error = %error,
                    "source worker panicked, restarting"
                ),
            }

            if self.shutdown.is_cancelled() {
                continue;
            }

            self.status.record_source_error(
                &path,
                "source worker exited unexpectedly; restarting".to_string(),
            );
            let source_shutdown = self.shutdown.child_token();
            let handle = spawn_file_source(
                worker.config.clone(),
                self.queue_config.clone(),
                self.enrichment.clone(),
                self.store.clone(),
                self.status.clone(),
                self.event_tx.clone(),
                source_shutdown.clone(),
            );
            self.workers.insert(
                path,
                SourceWorker {
                    config: worker.config,
                    shutdown: source_shutdown,
                    handle,
                },
            );
        }
    }
}

impl App {
    pub async fn load(config_path: PathBuf) -> AppResult<Self> {
        let config = AgentConfig::load(&config_path)?;
        logging::init(&config.log_level)?;
        info!(
            phase = "config_load",
            config_path = %config_path.display(),
            log_level = %config.log_level,
            "startup phase completed"
        );

        let hostname = resolve_hostname();
        let metadata = RuntimeMetadataContext::detect(&config, &config_path, &hostname)?;
        log_metadata_warnings(&metadata);
        info!(
            phase = "runtime_metadata_detect",
            resolved_install_mode = %metadata.install.resolved_mode,
            systemd_expected = metadata.install.systemd_expected,
            transport_mode = %config.transport.mode.as_str(),
            "startup phase completed"
        );

        let store = SqliteStateStore::new(&config.state_dir)?;
        info!(
            phase = "state_db_open",
            state_dir = %config.state_dir.display(),
            state_db = %store.db_path().display(),
            "startup phase completed"
        );
        let runtime_state = store.load_runtime_state()?;
        let security_state = store.load_security_scan_state()?;
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
        let initial_sources = if config.transport.mode.is_edge() {
            Vec::new()
        } else {
            config.sources.clone()
        };

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
                connectivity: build_connectivity_static_context(&config)?,
            },
            config.spool.enabled,
            config.heartbeat.interval_sec,
            config.diagnostics.interval_sec,
            &initial_sources,
            &persisted_offsets,
            runtime_state.last_successful_send_at_unix_ms,
        );
        status.restore_runtime_state(&runtime_state);
        status.restore_security_posture(SecurityPostureStatusSnapshot::from_record(
            &config.security_scan,
            &security_state,
        ));

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
        status.set_runtime_phase(RuntimePhase::Starting, None);

        let transport = build_transport(&config)?;
        info!(
            phase = "transport_init",
            edge_url = %config.edge_url,
            edge_grpc_addr = %config.edge_grpc_addr,
            transport_mode = %config.transport.mode.as_str(),
            "startup phase completed"
        );
        let mut app = Self {
            config,
            store,
            transport,
            status,
            metadata,
            hostname,
            version,
            agent_id: String::new(),
            active_sources: initial_sources,
        };

        info!(
            phase = "enrollment_connect",
            "starting bootstrap runtime flow"
        );
        let (agent_id, sources) = app.bootstrap_runtime().await?;
        app.agent_id = agent_id.clone();
        app.active_sources = sources.clone();
        app.status.set_agent_id(agent_id);
        app.status
            .set_configured_sources(&sources, &app.store.list_file_offsets()?);
        info!(
            phase = "source_validation",
            active_sources = sources.len(),
            "startup phase completed"
        );
        Ok(app)
    }

    pub async fn run(mut self) -> AppResult<()> {
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
            state_writer.clone(),
            shutdown.clone(),
            self.config.edge_url.clone(),
            self.config.heartbeat.interval_sec,
        );

        let diagnostics = spawn_diagnostics_worker(
            self.transport.clone(),
            self.status.clone(),
            state_writer.clone(),
            shutdown.clone(),
            self.config.state_dir.clone(),
            self.config.diagnostics.interval_sec,
        );

        let security_scan = spawn_security_scan_worker(
            self.transport.clone(),
            self.status.clone(),
            state_writer.clone(),
            shutdown.clone(),
            self.config.state_dir.clone(),
            self.hostname.clone(),
            self.config.security_scan.clone(),
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

        let mut source_supervisor = SourceSupervisor::new(
            self.config.queues.clone(),
            self.metadata.event_enrichment.clone(),
            self.store.clone(),
            self.status.clone(),
            event_tx,
            shutdown.clone(),
        );
        source_supervisor
            .reconcile(self.active_sources.clone())
            .await?;
        self.status.set_runtime_phase(
            if self.status.is_degraded_mode() {
                RuntimePhase::Degraded
            } else {
                RuntimePhase::Online
            },
            None,
        );
        info!(
            phase = "background_loops_start",
            heartbeat_interval_sec = self.config.heartbeat.interval_sec,
            diagnostics_interval_sec = self.config.diagnostics.interval_sec,
            policy_refresh_interval_sec = self.config.policy.refresh_interval_sec,
            "startup phase completed"
        );

        let mut policy_refresh = tokio::time::interval(std::time::Duration::from_secs(
            self.config.policy.refresh_interval_sec,
        ));
        policy_refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut source_supervision = tokio::time::interval(std::time::Duration::from_secs(5));
        source_supervision.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut worker_supervision = tokio::time::interval(std::time::Duration::from_secs(1));
        worker_supervision.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let shutdown_signal = shutdown_signal();
        tokio::pin!(shutdown_signal);
        let mut batcher = Some(batcher);
        let mut sender = Some(sender);
        let mut degraded = Some(degraded);
        let mut heartbeat = Some(heartbeat);
        let mut diagnostics = Some(diagnostics);
        let mut security_scan = Some(security_scan);
        let mut state_writer_handle = Some(state_writer_handle);
        let mut runtime_error = None;

        loop {
            tokio::select! {
                _ = &mut shutdown_signal => {
                    info!("shutdown signal received");
                    break;
                }
                _ = policy_refresh.tick(), if self.config.transport.mode.is_edge() => {
                    if let Err(error) = self.refresh_policy(&mut source_supervisor).await {
                        warn!(error = %error, "policy refresh failed without a recoverable runtime source set");
                    }
                }
                _ = source_supervision.tick() => {
                    source_supervisor.poll_workers().await;
                }
                _ = worker_supervision.tick() => {
                    if let Some(error) = poll_runtime_workers(
                        &mut batcher,
                        &mut sender,
                        &mut degraded,
                        &mut heartbeat,
                        &mut diagnostics,
                        &mut security_scan,
                        &mut state_writer_handle,
                    ).await {
                        runtime_error = Some(error);
                        break;
                    }
                }
            }
        }

        if let Some(error) = runtime_error.as_ref() {
            let detail = format!("runtime worker exited unexpectedly: {error}");
            self.status
                .set_runtime_phase(RuntimePhase::Error, Some(detail.clone()));
            let _ = state_writer
                .update_runtime_state(RuntimeStatePatch {
                    runtime_status: Some(Some(RuntimePhase::Error.as_str().to_string())),
                    runtime_status_reason: Some(Some(detail)),
                    ..RuntimeStatePatch::default()
                })
                .await;
        }

        if runtime_error.is_none() {
            self.status.set_runtime_phase(
                RuntimePhase::Stopping,
                Some("shutdown requested".to_string()),
            );
            let _ = state_writer
                .update_runtime_state(RuntimeStatePatch {
                    runtime_status: Some(Some(RuntimePhase::Stopping.as_str().to_string())),
                    runtime_status_reason: Some(Some("shutdown requested".to_string())),
                    ..RuntimeStatePatch::default()
                })
                .await;
        }

        shutdown.cancel();
        source_supervisor.shutdown().await;
        drop(source_supervisor);
        drop(send_tx);

        if let Some(handle) = batcher {
            let _ = handle.await;
        }
        if let Some(handle) = sender {
            let _ = handle.await??;
        }
        if let Some(handle) = degraded {
            let _ = handle.await??;
        }
        if let Some(handle) = heartbeat {
            let _ = handle.await??;
        }
        if let Some(handle) = diagnostics {
            let _ = handle.await??;
        }
        if let Some(handle) = security_scan {
            let _ = handle.await??;
        }
        if let Some(handle) = state_writer_handle {
            let _ = handle.await??;
        }

        info!("doro-agent stopped");
        if let Some(error) = runtime_error {
            return Err(error);
        }
        Ok(())
    }

    async fn bootstrap_runtime(&mut self) -> AppResult<(String, Vec<SourceConfig>)> {
        let mut runtime_state = self.store.load_runtime_state()?;
        let (agent_id, identity_status) = self.ensure_identity(&mut runtime_state).await?;

        if self.config.transport.mode == TransportMode::Mock {
            let sources = self.config.sources.clone();
            self.status
                .set_configured_sources(&sources, &self.store.list_file_offsets()?);
            self.status.set_runtime_phase(RuntimePhase::Online, None);
            self.persist_runtime_phase(RuntimePhase::Online, None)?;
            return Ok((agent_id, sources));
        }

        match self
            .fetch_policy_candidate(agent_id.clone(), identity_status.clone())
            .await
        {
            Ok(fetched) => {
                self.commit_policy_success(&fetched, true)?;
                Ok((fetched.resolved_agent_id, fetched.sources))
            }
            Err(error) => {
                let Some(fallback_sources) = self.load_persisted_policy_sources(&runtime_state)?
                else {
                    return Err(error);
                };

                let detail = format!("policy sync failed: {error}");
                warn!(error = %error, "starting agent with last persisted policy after startup sync failure");
                self.persist_policy_failure(&mut runtime_state, &error, &detail)?;
                self.status
                    .set_runtime_phase(RuntimePhase::Degraded, Some(detail));
                self.status
                    .set_configured_sources(&fallback_sources, &self.store.list_file_offsets()?);
                Ok((agent_id, fallback_sources))
            }
        }
    }

    async fn refresh_policy(&mut self, source_supervisor: &mut SourceSupervisor) -> AppResult<()> {
        let identity_status = self
            .store
            .load_runtime_state()?
            .identity_status
            .map(|status| IdentityStatusSnapshot {
                status,
                reason: None,
            })
            .unwrap_or_else(|| IdentityStatusSnapshot {
                status: "reused".to_string(),
                reason: Some("using current in-memory identity".to_string()),
            });

        match self
            .fetch_policy_candidate(self.agent_id.clone(), identity_status)
            .await
        {
            Ok(fetched) => {
                let changed = fetched.sources != self.active_sources;
                if changed {
                    if let Err(error) = source_supervisor.reconcile(fetched.sources.clone()).await {
                        let mut runtime_state = self.store.load_runtime_state()?;
                        let detail = format!("policy apply failed: {error}");
                        self.persist_policy_failure(&mut runtime_state, &error, &detail)?;
                        self.status
                            .set_runtime_phase(RuntimePhase::Degraded, Some(detail));
                        return Ok(());
                    }
                }
                self.commit_policy_success(&fetched, false)?;
                self.agent_id = fetched.resolved_agent_id.clone();
                self.active_sources = fetched.sources.clone();
                Ok(())
            }
            Err(error) => {
                let mut runtime_state = self.store.load_runtime_state()?;
                let detail = format!("policy sync failed: {error}");
                self.persist_policy_failure(&mut runtime_state, &error, &detail)?;
                self.status
                    .set_runtime_phase(RuntimePhase::Degraded, Some(detail));
                Ok(())
            }
        }
    }

    async fn ensure_identity(
        &mut self,
        runtime_state: &mut RuntimeStateRecord,
    ) -> AppResult<(String, IdentityStatusSnapshot)> {
        let persisted_identity = self.store.load_identity()?;
        let (agent_id, identity_status) = if let Some(identity) = persisted_identity {
            (
                identity.agent_id,
                IdentityStatusSnapshot {
                    status: "reused".to_string(),
                    reason: Some("persisted identity accepted".to_string()),
                },
            )
        } else {
            self.status.set_runtime_phase(
                RuntimePhase::Enrolling,
                Some("enrolling agent identity".to_string()),
            );
            let agent_id = self.enroll(None).await?;
            (
                agent_id,
                IdentityStatusSnapshot {
                    status: "newly_enrolled".to_string(),
                    reason: Some("no persisted identity was found".to_string()),
                },
            )
        };

        runtime_state.identity_status = Some(identity_status.status.clone());
        runtime_state.identity_status_reason = identity_status.reason.clone();
        runtime_state.last_known_edge_url = Some(self.config.edge_url.clone());
        runtime_state.runtime_status = Some(RuntimePhase::Starting.as_str().to_string());
        runtime_state.runtime_status_reason = None;
        runtime_state.spool_enabled = self.config.spool.enabled;
        runtime_state.updated_at_unix_ms = Utc::now().timestamp_millis();
        self.store.save_runtime_state(runtime_state)?;

        self.status.set_agent_id(agent_id.clone());
        self.status.set_identity_status(identity_status.clone());
        self.status
            .set_last_known_edge_url(Some(self.config.edge_url.clone()));

        Ok((agent_id, identity_status))
    }

    async fn fetch_policy_candidate(
        &mut self,
        agent_id: String,
        mut identity_status: IdentityStatusSnapshot,
    ) -> AppResult<FetchedPolicy> {
        self.status.set_runtime_phase(
            RuntimePhase::PolicySyncing,
            Some("synchronizing policy".to_string()),
        );
        let mut runtime_state = self.store.load_runtime_state()?;
        let fetch_started_at = Utc::now().timestamp_millis();

        let (resolved_agent_id, policy) = match self
            .transport
            .fetch_policy(FetchPolicyRequest {
                agent_id: agent_id.clone(),
                current_revision: runtime_state.applied_policy_revision.clone(),
            })
            .await
        {
            Ok(policy) => {
                self.status.record_connectivity_success(fetch_started_at);
                (agent_id, policy)
            }
            Err(error) if error.is_identity_error() => {
                warn!(error = %error, "stored agent identity was rejected, re-enrolling");
                self.status.record_connectivity_error(&error);
                identity_status = IdentityStatusSnapshot {
                    status: "re_enrolled".to_string(),
                    reason: Some("stored identity was rejected by the server".to_string()),
                };
                let reenrolled_agent_id = self.enroll(Some(agent_id.clone())).await?;
                self.status.set_agent_id(reenrolled_agent_id.clone());
                let policy = self
                    .transport
                    .fetch_policy(FetchPolicyRequest {
                        agent_id: reenrolled_agent_id.clone(),
                        current_revision: None,
                    })
                    .await?;
                self.status
                    .record_connectivity_success(Utc::now().timestamp_millis());
                (reenrolled_agent_id, policy)
            }
            Err(error) => {
                self.status.record_connectivity_error(&error);
                return Err(error);
            }
        };

        self.status.set_policy_fetch_result(fetch_started_at, None);
        let candidate_sources = if self.config.transport.mode.is_edge() {
            parse_file_sources(&policy.policy_body_json)?
        } else {
            self.config.sources.clone()
        };

        runtime_state.identity_status = Some(identity_status.status.clone());
        runtime_state.identity_status_reason = identity_status.reason.clone();
        runtime_state.updated_at_unix_ms = fetch_started_at;
        self.store.save_runtime_state(&runtime_state)?;

        Ok(FetchedPolicy {
            resolved_agent_id,
            identity_status,
            policy_id: policy.policy_id,
            policy_revision: policy.policy_revision,
            policy_body_json: policy.policy_body_json,
            policy_status: policy.status,
            sources: candidate_sources,
            fetched_at_unix_ms: fetch_started_at,
        })
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
            .await
            .inspect_err(|error| self.status.record_connectivity_error(error))?;

        self.status
            .record_connectivity_success(Utc::now().timestamp_millis());
        self.store
            .save_identity(&response.agent_id, &self.hostname, &self.version)?;
        info!(
            agent_id = response.agent_id,
            enrollment_status = response.status,
            "agent enrolled"
        );
        Ok(response.agent_id)
    }

    fn load_persisted_policy_sources(
        &self,
        runtime_state: &RuntimeStateRecord,
    ) -> AppResult<Option<Vec<SourceConfig>>> {
        let Some(policy_body_json) = runtime_state.policy_body_json.as_deref() else {
            return Ok(None);
        };
        Ok(Some(parse_file_sources(policy_body_json)?))
    }

    fn persist_policy_failure(
        &self,
        runtime_state: &mut RuntimeStateRecord,
        error: &AppError,
        detail: &str,
    ) -> AppResult<()> {
        let now = Utc::now().timestamp_millis();
        runtime_state.last_known_edge_url = Some(self.config.edge_url.clone());
        runtime_state.last_policy_fetch_at_unix_ms = Some(now);
        runtime_state.last_policy_error = Some(error.to_string());
        runtime_state.runtime_status = Some(RuntimePhase::Degraded.as_str().to_string());
        runtime_state.runtime_status_reason = Some(detail.to_string());
        runtime_state.spool_enabled = self.config.spool.enabled;
        runtime_state.updated_at_unix_ms = now;
        self.store.save_runtime_state(runtime_state)?;

        self.status
            .set_policy_fetch_result(now, Some(error.to_string()));
        self.status.set_policy_error(error.to_string());
        self.status.record_connectivity_error(error);
        Ok(())
    }

    fn commit_policy_success(&mut self, fetched: &FetchedPolicy, startup: bool) -> AppResult<()> {
        let apply_at = Utc::now().timestamp_millis();
        let runtime_phase = if self.status.is_degraded_mode() {
            RuntimePhase::Degraded
        } else {
            RuntimePhase::Online
        };
        let mut runtime_state = self.store.load_runtime_state()?;
        runtime_state.applied_policy_revision = Some(fetched.policy_revision.clone());
        runtime_state.policy_body_json = Some(fetched.policy_body_json.clone());
        runtime_state.last_known_edge_url = Some(self.config.edge_url.clone());
        runtime_state.identity_status = Some(fetched.identity_status.status.clone());
        runtime_state.identity_status_reason = fetched.identity_status.reason.clone();
        runtime_state.last_policy_fetch_at_unix_ms = Some(fetched.fetched_at_unix_ms);
        runtime_state.last_policy_apply_at_unix_ms = Some(apply_at);
        runtime_state.last_policy_error = None;
        runtime_state.runtime_status = Some(runtime_phase.as_str().to_string());
        runtime_state.runtime_status_reason = None;
        runtime_state.spool_enabled = self.config.spool.enabled;
        runtime_state.updated_at_unix_ms = apply_at;
        self.store.save_runtime_state(&runtime_state)?;

        info!(
            agent_id = fetched.resolved_agent_id,
            policy_id = fetched.policy_id,
            policy_revision = fetched.policy_revision,
            policy_status = fetched.policy_status,
            identity_status = fetched.identity_status.status,
            startup,
            "agent policy synchronized"
        );

        self.status
            .set_policy_revision(Some(fetched.policy_revision.clone()));
        self.status
            .set_identity_status(fetched.identity_status.clone());
        self.status
            .set_last_known_edge_url(Some(self.config.edge_url.clone()));
        self.status.set_agent_id(fetched.resolved_agent_id.clone());
        self.status.set_policy_apply_success(
            Some(fetched.policy_revision.clone()),
            apply_at,
            fetched.sources.len(),
        );
        self.status.set_runtime_phase(runtime_phase, None);
        self.status
            .set_configured_sources(&fetched.sources, &self.store.list_file_offsets()?);
        Ok(())
    }

    fn persist_runtime_phase(&self, phase: RuntimePhase, reason: Option<String>) -> AppResult<()> {
        let mut runtime_state = self.store.load_runtime_state()?;
        runtime_state.runtime_status = Some(phase.as_str().to_string());
        runtime_state.runtime_status_reason = reason;
        runtime_state.updated_at_unix_ms = Utc::now().timestamp_millis();
        self.store.save_runtime_state(&runtime_state)
    }
}

fn build_transport(config: &AgentConfig) -> AppResult<Arc<dyn AgentTransport>> {
    match config.transport.mode {
        TransportMode::Mock => Ok(Arc::new(MockTransport::default())),
        TransportMode::Edge => Ok(Arc::new(EdgeGrpcTransport::new(
            &config.edge_url,
            &config.edge_grpc_addr,
            &config.tls,
        )?)),
    }
}

fn build_connectivity_static_context(config: &AgentConfig) -> AppResult<ConnectivityStaticContext> {
    Ok(ConnectivityStaticContext {
        endpoint: build_base_url(&config.edge_url, &config.edge_grpc_addr)?,
        tls_enabled: endpoint_uses_tls(&config.edge_url, &config.edge_grpc_addr)?,
        mtls_enabled: config.tls.cert_path.is_some() && config.tls.key_path.is_some(),
        server_name: derive_server_name(
            &config.edge_url,
            &config.edge_grpc_addr,
            config.tls.server_name.as_deref(),
        )?,
        ca_path: config
            .tls
            .ca_path
            .as_ref()
            .map(|path| path.display().to_string()),
        cert_path: config
            .tls
            .cert_path
            .as_ref()
            .map(|path| path.display().to_string()),
        key_path: config
            .tls
            .key_path
            .as_ref()
            .map(|path| path.display().to_string()),
    })
}

fn log_source_shape(sources: &[SourceConfig]) {
    if sources.is_empty() {
        warn!("agent runtime has no configured sources");
    } else if sources.len() > SOURCE_COUNT_WARN_THRESHOLD {
        warn!(
            source_count = sources.len(),
            threshold = SOURCE_COUNT_WARN_THRESHOLD,
            "high source count configured; current runtime uses one reader task per source"
        );
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

async fn poll_runtime_workers(
    batcher: &mut Option<JoinHandle<()>>,
    sender: &mut Option<JoinHandle<AppResult<()>>>,
    degraded: &mut Option<JoinHandle<AppResult<()>>>,
    heartbeat: &mut Option<JoinHandle<AppResult<()>>>,
    diagnostics: &mut Option<JoinHandle<AppResult<()>>>,
    security_scan: &mut Option<JoinHandle<AppResult<()>>>,
    state_writer_handle: &mut Option<JoinHandle<AppResult<()>>>,
) -> Option<AppError> {
    if batcher.as_ref().is_some_and(JoinHandle::is_finished) {
        let handle = batcher.take().expect("batcher handle must exist");
        return Some(match handle.await {
            Ok(()) => AppError::protocol("batcher worker exited unexpectedly"),
            Err(error) => error.into(),
        });
    }
    if sender.as_ref().is_some_and(JoinHandle::is_finished) {
        let handle = sender.take().expect("sender handle must exist");
        return Some(unexpected_async_worker_exit("sender", handle).await);
    }
    if degraded.as_ref().is_some_and(JoinHandle::is_finished) {
        let handle = degraded.take().expect("degraded handle must exist");
        return Some(unexpected_async_worker_exit("degraded", handle).await);
    }
    if heartbeat.as_ref().is_some_and(JoinHandle::is_finished) {
        let handle = heartbeat.take().expect("heartbeat handle must exist");
        return Some(unexpected_async_worker_exit("heartbeat", handle).await);
    }
    if diagnostics.as_ref().is_some_and(JoinHandle::is_finished) {
        let handle = diagnostics.take().expect("diagnostics handle must exist");
        return Some(unexpected_async_worker_exit("diagnostics", handle).await);
    }
    if security_scan.as_ref().is_some_and(JoinHandle::is_finished) {
        let handle = security_scan
            .take()
            .expect("security scan handle must exist");
        return Some(unexpected_async_worker_exit("security_scan", handle).await);
    }
    if state_writer_handle
        .as_ref()
        .is_some_and(JoinHandle::is_finished)
    {
        let handle = state_writer_handle
            .take()
            .expect("state writer handle must exist");
        return Some(unexpected_async_worker_exit("state_writer", handle).await);
    }
    None
}

async fn unexpected_async_worker_exit(name: &str, handle: JoinHandle<AppResult<()>>) -> AppError {
    match handle.await {
        Ok(Ok(())) => AppError::protocol(format!("{name} worker exited unexpectedly")),
        Ok(Err(error)) => error,
        Err(error) => error.into(),
    }
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut terminate = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
