//! Binario de rtile. Carga la config dinámica (hot-reload) desde `librtile_config.so`.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }

    let path = config_path();
    match rtile_core::load_config(&path) {
        Ok((library, config)) => rtile_core::run(config, Some(library), Some(path)),
        Err(e) => {
            tracing::error!("no se pudo cargar la config ({path:?}): {e}");
            Err(e)
        }
    }
}

/// Ruta de la config dinámica: junto al binario, o `RTILE_CONFIG`.
fn config_path() -> PathBuf {
    std::env::var_os("RTILE_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|d| d.join("librtile_config.so")))
        })
        .unwrap_or_else(|| PathBuf::from("librtile_config.so"))
}
