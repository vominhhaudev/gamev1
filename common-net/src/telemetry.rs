use std::sync::Once;

use tracing::info;
use tracing_subscriber::EnvFilter;

static INIT: Once = Once::new();

pub fn init(service_name: &str) {
    INIT.call_once(|| {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_thread_names(true)
            .compact()
            .init();
    });

    info!(service = service_name, "telemetry initialized");
}
