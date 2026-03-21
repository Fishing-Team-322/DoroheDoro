use anyhow::Result;
use common::{telemetry::init_tracing, EnrollmentPlaneConfig};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = EnrollmentPlaneConfig::from_env()?;
    init_tracing(&config.shared.rust_log);

    enrollment_plane::run(config).await
}
