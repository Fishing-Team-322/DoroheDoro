use anyhow::Context;
use async_nats::Client;
use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn connect_postgres(postgres_dsn: &str, max_connections: u32) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(postgres_dsn)
        .await
        .with_context(|| format!("connect postgres: {postgres_dsn}"))
}

pub async fn connect_nats(nats_url: &str) -> anyhow::Result<Client> {
    async_nats::connect(nats_url)
        .await
        .with_context(|| format!("connect nats: {nats_url}"))
}

pub async fn run_migrations(
    migrator: &sqlx::migrate::Migrator,
    pool: &PgPool,
) -> anyhow::Result<()> {
    migrator.run(pool).await.context("run postgres migrations")
}

pub async fn shutdown_signal() {
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
