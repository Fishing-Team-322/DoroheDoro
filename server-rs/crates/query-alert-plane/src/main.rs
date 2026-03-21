use anyhow::Result;
use common::telemetry::init_tracing;
use query_alert_plane::config::QueryAlertPlaneConfig;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = QueryAlertPlaneConfig::from_env()?;
    init_tracing(&config.shared.rust_log);

    query_alert_plane::run(config).await
}
