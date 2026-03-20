use std::sync::OnceLock;

use tracing_subscriber::EnvFilter;

static TRACING_INIT: OnceLock<()> = OnceLock::new();

pub fn init_tracing(default_filter: &str) {
    let default_filter = default_filter.to_string();
    TRACING_INIT.get_or_init(|| {
        let filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(default_filter))
            .expect("valid tracing filter");

        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .try_init();
    });
}
