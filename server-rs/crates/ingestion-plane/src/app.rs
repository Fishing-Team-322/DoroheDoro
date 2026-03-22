use std::sync::Arc;

use anyhow::Context;
use common::bootstrap;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::{
    config::IngestionPlaneConfig,
    http::{self, HttpState},
    service::IngestionService,
    transport,
};

pub async fn run(config: IngestionPlaneConfig) -> anyhow::Result<()> {
    let nats = bootstrap::connect_nats(&config.nats_url).await?;
    let service = Arc::new(IngestionService::new(&config, nats.clone()));
    service
        .initialize()
        .await
        .context("initialize ingestion storage")?;

    let shutdown = CancellationToken::new();
    let subscriber_tasks =
        transport::spawn_handlers(nats.clone(), service.clone(), shutdown.clone()).await?;

    let listener = TcpListener::bind(&config.http_addr)
        .await
        .with_context(|| format!("bind http listener on {}", config.http_addr))?;
    let addr = listener.local_addr().context("resolve local http addr")?;

    info!(
        http_addr = %addr,
        nats_url = %config.nats_url,
        "starting ingestion-plane"
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
