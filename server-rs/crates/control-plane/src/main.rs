use anyhow::Result;
use common::{telemetry::init_tracing, ControlPlaneConfig};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = ControlPlaneConfig::from_env()?;
    init_tracing(&config.shared.rust_log);

    control_plane::run(config).await
}
