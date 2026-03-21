use anyhow::Result;
use common::{telemetry::init_tracing, RuntimeConfig};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = RuntimeConfig::from_env()?;
    init_tracing(&config.rust_log);

    control_plane::run(config).await
}
