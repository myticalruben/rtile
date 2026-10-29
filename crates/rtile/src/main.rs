//! Binario de rtile. La configuración vive en [`config::config`].

mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }

    let config = config::config();
    rtile_core::run(config)
}
