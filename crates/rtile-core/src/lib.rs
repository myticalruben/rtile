//! rtile — un tiling window manager para Wayland con configuración 100% en Rust.
//!
//! Estructura:
//! - módulos *puros* (`geometry`, `keybind`, `layout`, `appearance`, `hook`,
//!   `rule`, `workspace`, `config`): la lógica del WM, sin dependencias de Wayland.
//! - `compositor`: el backend real sobre [`smithay`](https://smithay.github.io/).

pub mod appearance;
pub mod compositor;
pub mod config;
pub mod geometry;
pub mod hook;
pub mod keybind;
pub mod layout;
pub mod rule;
pub mod workspace;

/// Identificador estable de una ventana dentro de una sesión.
pub type ClientId = u64;

pub use appearance::{Appearance, Color};
pub use config::{Config, ConfigBuilder};
pub use geometry::{Point, Rect, Size};
pub use hook::{HookEvent, Hooks};
pub use keybind::{Action, Direction, Key, Keybind, Mod};
pub use layout::{Columns, Floating, Grid, Layout, Monocle, Tile};
pub use rule::{Rule, WindowMatcher, WindowProps};
pub use workspace::WorkspaceSpec;

use std::path::{Path, PathBuf};

use libloading::Library;

/// Carga la configuración desde la librería dinámica en `path`.
pub fn load_config(path: &Path) -> Result<(Library, Config), Box<dyn std::error::Error>> {
    let library = unsafe { Library::new(path)? };
    let config = unsafe {
        let load: libloading::Symbol<unsafe extern "C" fn() -> *mut Config> =
            library.get(b"rtile_config_load")?;
        *Box::from_raw(load())
    };
    Ok((library, config))
}

/// Arranca el compositor. `config_library` mantiene viva la `.so` de la config
/// (para sus closures) y `config_path` permite recargarla en caliente.
pub fn run(
    config: Config,
    config_library: Option<Library>,
    config_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    compositor::run(config, config_library, config_path)
}
