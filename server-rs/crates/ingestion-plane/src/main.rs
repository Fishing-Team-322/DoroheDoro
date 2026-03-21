use anyhow::Result;
use common::telemetry::init_tracing;
use ingestion_plane::config::IngestionPlaneConfig;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = IngestionPlaneConfig::from_env()?;
    init_tracing(&config.rust_log);

    ingestion_plane::run(config).await
}
