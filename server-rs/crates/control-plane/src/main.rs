use anyhow::Result;
use common::telemetry::init_tracing;
use control_plane::config::ControlPlaneRuntimeConfig;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = ControlPlaneRuntimeConfig::from_env()?;
    init_tracing(&config.shared.rust_log);

    control_plane::run(config).await
}
