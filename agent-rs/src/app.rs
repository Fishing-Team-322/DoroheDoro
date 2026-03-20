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
    runtime::{r#loop::spawn_heartbeat_loop, RuntimeStatusHandle},
    sources::spawn_file_source,
    state::{RuntimeStateRecord, SqliteStateStore},
    transport::{
        AgentTransport, DynTransport, EdgeGrpcTransport, EnrollRequest, FetchPolicyRequest,
        MockTransport,
    },
};

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
            hostname.clone(),
            version.clone(),
            &config.sources,
            &persisted_offsets,
        );

        if let Some(revision) = runtime_state.applied_policy_revision.clone() {
            status.set_policy_revision(Some(revision));
        }
        if let Some(last_send) = runtime_state.last_successful_send_at_unix_ms {
            status.record_last_send(last_send);
        }

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
        Ok(app)
    }

    pub async fn run(self) -> AppResult<()> {
        info!(
            agent_id = self.agent_id,
            state_db = %self.store.db_path().display(),
            transport_mode = ?self.config.transport.mode,
            "starting doro-agent"
        );

        let shutdown = CancellationToken::new();
        let (tx, rx) = mpsc::channel(self.config.batch.max_events.saturating_mul(8).max(128));

        let batcher = spawn_batcher(
            rx,
            self.transport.clone(),
            self.store.clone(),
            self.status.clone(),
            shutdown.clone(),
            self.config.batch.clone(),
            self.agent_id.clone(),
            self.hostname.clone(),
        );

        let heartbeat = spawn_heartbeat_loop(
            self.transport.clone(),
            self.status.clone(),
            shutdown.clone(),
            self.agent_id.clone(),
            self.hostname.clone(),
            self.version.clone(),
            self.config.edge_url.clone(),
            self.config.transport.mode.clone(),
            self.config.heartbeat.interval_sec,
        );

        let mut source_handles = Vec::new();
        if self.config.sources.is_empty() {
            warn!("agent started without configured sources");
        }
        for source in self.config.sources.clone() {
            source_handles.push(spawn_file_source(
                source,
                self.store.clone(),
                self.status.clone(),
                tx.clone(),
                shutdown.clone(),
            ));
        }

        tokio::signal::ctrl_c().await?;
        info!("shutdown signal received");
        shutdown.cancel();
        drop(tx);

        for handle in source_handles {
            let _ = handle.await;
        }
        let _ = batcher.await;
        let _ = heartbeat.await?;

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

fn enrollment_metadata(mode: &TransportMode, edge_url: &str) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("os".to_string(), std::env::consts::OS.to_string());
    metadata.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    metadata.insert("edge_url".to_string(), edge_url.to_string());
    metadata.insert(
        "transport_mode".to_string(),
        match mode {
            TransportMode::Edge => "edge".to_string(),
            TransportMode::Mock => "mock".to_string(),
        },
    );
    metadata
}
