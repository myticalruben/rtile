//! Backend del compositor sobre `smithay` (Wayland, backend winit).
//!
//! Estructura (inspirada en `smallvil` de smithay 0.7):
//! - `mod.rs`: entrada `run()`, datos del bucle de calloop y estado por cliente.
//! - `state.rs`: `State` (estado global) + gestión de ventanas / tiling / input.
//! - `handlers.rs`: implementaciones de los handlers de Wayland + macros `delegate_*!`.
//! - `winit.rs`: arranque del backend winit y render.

mod handlers;
mod state;
mod winit;

use smithay::{
    reexports::wayland_server::{
        backend::{ClientData, ClientId as WlClientId, DisconnectReason},
        Display, DisplayHandle,
    },
    wayland::compositor::CompositorClientState,
};

use crate::{Config, HookEvent};

pub use state::State;

/// Datos del bucle de eventos de calloop.
pub struct CalloopData {
    pub state: State,
    pub display_handle: DisplayHandle,
}

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
pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    use smithay::reexports::calloop::EventLoop;

    let mut event_loop: EventLoop<CalloopData> = EventLoop::try_new()?;
    let display: Display<State> = Display::new()?;
    let display_handle = display.handle();

    let state = State::new(&mut event_loop, display, config);
    let mut data = CalloopData {
        state,
        display_handle,
    };

    winit::init_winit(&mut event_loop, &mut data)?;

    for cmd in &data.state.config.startup {
        data.state.spawn(cmd);
    }
    data.state.config.hooks.dispatch(&HookEvent::Startup);

    event_loop.run(None, &mut data, |_| {})?;

    Ok(())
}
