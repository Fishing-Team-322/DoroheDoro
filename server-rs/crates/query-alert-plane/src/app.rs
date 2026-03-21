use std::sync::Arc;

use anyhow::Context;
use common::bootstrap;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::{
    config::QueryAlertPlaneConfig,
    http::{self, HttpState},
    repository::QueryAlertRepository,
    service::QueryAlertService,
    storage::{ClickHouseClient, OpenSearchClient},
    transport,
};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

pub async fn run(config: QueryAlertPlaneConfig) -> anyhow::Result<()> {
    let pool = bootstrap::connect_postgres(&config.shared.postgres_dsn, 5).await?;
    bootstrap::run_migrations(&MIGRATOR, &pool).await?;
    let nats = bootstrap::connect_nats(&config.shared.nats_url).await?;

    let repo = QueryAlertRepository::new(pool.clone());
    let service = Arc::new(QueryAlertService::new(
        repo,
        nats.clone(),
        OpenSearchClient::new(config.opensearch.clone()),
        ClickHouseClient::new(config.clickhouse.clone()),
        config.detection.clone(),
    ));
    service
        .initialize()
        .await
        .context("initialize query-alert-plane")?;

    let shutdown = CancellationToken::new();
    let subscriber_tasks =
        transport::spawn_handlers(nats.clone(), service.clone(), shutdown.clone()).await?;

    let listener = TcpListener::bind(&config.http_addr)
        .await
        .with_context(|| format!("bind http listener on {}", config.http_addr))?;
    let addr = listener.local_addr().context("resolve local http addr")?;

    info!(
        http_addr = %addr,
        nats_url = %config.shared.nats_url,
        "starting query-alert-plane"
    );

    let server = axum::serve(listener, http::router(HttpState::new(service)))
        .with_graceful_shutdown(bootstrap::shutdown_signal());

    let server_result = server.await.context("run http server");
    shutdown.cancel();

    for task in subscriber_tasks {
        task.abort();
        let _ = task.await;
    }

    server_result
}
