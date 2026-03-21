use std::sync::Arc;

use anyhow::Context;
use common::bootstrap;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::{
    config::DeploymentConfig,
    executor::{AnsibleRunnerExecutor, DynDeploymentExecutor, MockExecutor, MockExecutorOptions},
    health::{self, HealthState},
    repository::DeploymentRepository,
    service::DeploymentService,
    transport,
};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

pub async fn run(config: DeploymentConfig) -> anyhow::Result<()> {
    let pool = bootstrap::connect_postgres(&config.shared.postgres_dsn, 5).await?;
    bootstrap::run_migrations(&MIGRATOR, &pool).await?;
    let nats = bootstrap::connect_nats(&config.shared.nats_url).await?;
    let executor = build_executor(&config)?;

    let repo = DeploymentRepository::new(pool.clone());
    repo.ping().await.context("ping postgres")?;
    executor
        .readiness_check()
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("initialize executor")?;

    let service = Arc::new(DeploymentService::new(
        repo,
        nats.clone(),
        executor.clone(),
        config.edge_public_url.clone(),
        config.edge_grpc_addr.clone(),
        config.agent_state_dir_default.clone(),
    ));
    service
        .reconcile_stale_attempts()
        .await
        .context("reconcile stale attempts")?;

    let shutdown = CancellationToken::new();
    let subscriber_tasks =
        transport::spawn_handlers(nats.clone(), service, shutdown.clone()).await?;

    let listener = TcpListener::bind(&config.deployment_http_addr)
        .await
        .with_context(|| format!("bind http listener on {}", config.deployment_http_addr))?;
    let addr = listener.local_addr().context("resolve local http addr")?;

    info!(
        http_addr = %addr,
        nats_url = %config.shared.nats_url,
        executor_kind = config.deployment_executor_kind.as_str(),
        "starting deployment-plane"
    );

    let server = axum::serve(
        listener,
        health::router(HealthState::new(pool, nats, executor)),
    )
    .with_graceful_shutdown(bootstrap::shutdown_signal());

    let server_result = server.await.context("run http server");
    shutdown.cancel();

    for task in subscriber_tasks {
        task.abort();
        let _ = task.await;
    }

    server_result
}

fn build_executor(config: &DeploymentConfig) -> anyhow::Result<DynDeploymentExecutor> {
    match config.deployment_executor_kind {
        crate::models::ExecutorKind::Mock => Ok(Arc::new(MockExecutor::new(MockExecutorOptions {
            step_delay_ms: config.mock_executor_step_delay_ms,
            fail_mode: config.mock_executor_fail_mode,
            fail_hostnames: config.mock_executor_fail_hosts.iter().cloned().collect(),
        }))),
        crate::models::ExecutorKind::Ansible => {
            let runner_bin = config
                .ansible_runner_bin
                .clone()
                .context("ANSIBLE_RUNNER_BIN is required for ansible executor")?;
            let playbook_path = config
                .ansible_playbook_path
                .clone()
                .context("ANSIBLE_PLAYBOOK_PATH is required for ansible executor")?;
            let temp_dir = config
                .deployment_temp_dir
                .clone()
                .unwrap_or_else(|| std::env::temp_dir().join("doro-deployment-plane"));
            Ok(Arc::new(AnsibleRunnerExecutor::new(
                runner_bin,
                playbook_path,
                temp_dir,
            )))
        }
    }
}
