use anyhow::Result;
use common::telemetry::init_tracing;

use deployment_plane::config::DeploymentConfig;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = DeploymentConfig::from_env()?;
    init_tracing(&config.shared.rust_log);

    deployment_plane::run(config).await
}
