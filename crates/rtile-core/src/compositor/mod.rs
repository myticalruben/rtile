//! Backend del compositor sobre `smithay` (Wayland, backend winit).
//!
//! Estructura (inspirada en `smallvil` de smithay 0.7):
//! - `mod.rs`: entrada `run()`, datos del bucle de calloop y estado por cliente.
//! - `state.rs`: `State` (estado global) + gestión de ventanas / tiling / input.
//! - `handlers.rs`: implementaciones de los handlers de Wayland + macros `delegate_*!`.
//! - `winit.rs`: arranque del backend winit y render.

mod handlers;
mod render;
mod state;
mod winit;

use smithay::{
    reexports::wayland_server::{
        backend::{ClientData, ClientId as WlClientId, DisconnectReason},
        Display,
    },
    wayland::compositor::CompositorClientState,
};

use crate::{Config, HookEvent};

pub use state::State;

/// Estado por cliente Wayland.
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: WlClientId) {}
    fn disconnected(&self, _client_id: WlClientId, _reason: DisconnectReason) {}
}

/// Arranca el compositor con la configuración dada.
pub fn run(
    config: Config,
    config_library: Option<libloading::Library>,
    config_path: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use smithay::reexports::calloop::EventLoop;

    let mut event_loop: EventLoop<'static, State> = EventLoop::try_new()?;
    let display: Display<State> = Display::new()?;

    let mut state = State::new(&mut event_loop, display, config, config_library, config_path);

    winit::init_winit(&mut event_loop, &mut state)?;
    state.start_xwayland();

    for cmd in &state.config.startup {
        state.spawn(cmd);
    }
    state.config.hooks.dispatch(&HookEvent::Startup);

    event_loop.run(None, &mut state, |_| {})?;

    Ok(())
}
