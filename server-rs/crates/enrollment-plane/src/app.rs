use std::sync::Arc;

use anyhow::Context;
use async_nats::Client;
use common::RuntimeConfig;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::{
    http::{self, HttpState},
    repository::EnrollmentRepository,
    service::EnrollmentService,
    transport,
};

pub async fn run(config: RuntimeConfig) -> anyhow::Result<()> {
    let pool = connect_postgres(&config.postgres_dsn).await?;
    let nats = connect_nats(&config.nats_url).await?;

    let repo = EnrollmentRepository::new(pool.clone());
    repo.ping().await.context("ping postgres")?;

    let service = Arc::new(EnrollmentService::new(repo));
    service
        .bootstrap_defaults(&config.enrollment_dev_bootstrap_token)
        .await
        .context("bootstrap dev policy and token")?;

    let shutdown = CancellationToken::new();
    let subscriber_tasks =
        transport::spawn_handlers(nats.clone(), service, shutdown.clone()).await?;

    let listener = TcpListener::bind(&config.enrollment_http_addr)
        .await
        .with_context(|| format!("bind http listener on {}", config.enrollment_http_addr))?;
    let addr = listener.local_addr().context("resolve local http addr")?;

    info!(
        http_addr = %addr,
        nats_url = %config.nats_url,
        "starting enrollment-plane"
    );

    let server = axum::serve(listener, http::router(HttpState::new(pool, nats)))
        .with_graceful_shutdown(shutdown_signal());

    let server_result = server.await.context("run http server");
    shutdown.cancel();

    for task in subscriber_tasks {
        task.abort();
        let _ = task.await;
    }

    server_result
}

async fn connect_postgres(postgres_dsn: &str) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(postgres_dsn)
        .await
        .with_context(|| format!("connect postgres: {postgres_dsn}"))
}

async fn connect_nats(nats_url: &str) -> anyhow::Result<Client> {
    async_nats::connect(nats_url)
        .await
        .with_context(|| format!("connect nats: {nats_url}"))
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let ctrl_c = tokio::signal::ctrl_c();
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler")
                .recv()
                .await;
        };

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
