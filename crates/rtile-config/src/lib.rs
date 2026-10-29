//! Configuración de rtile como librería dinámica (`.so`) para hot-reload.
//!
//! El binario `rtile` carga esta librería y llama `rtile_config_load()` para
//! obtener la [`Config`]. Para recargar: recompila solo este crate
//! (`cargo build -p rtile-config`) y pulsa el keybind de recarga.

mod config;

use rtile_core::Config;

/// Devuelve la config (Box filtrado por el binario) vía FFI.
#[no_mangle]
pub extern "C" fn rtile_config_load() -> *mut Config {
    Box::into_raw(Box::new(config::config()))
}
