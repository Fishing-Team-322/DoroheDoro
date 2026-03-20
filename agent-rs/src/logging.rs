use tracing_subscriber::{fmt, EnvFilter};

use crate::error::AppResult;

pub fn init(default_level: &str) -> AppResult<()> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    let _ = fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .compact()
        .try_init();

    Ok(())
}
