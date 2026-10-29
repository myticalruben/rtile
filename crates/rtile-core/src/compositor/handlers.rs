//! Implementaciones de los handlers de Wayland + macros `delegate_*!`.
//!
//! Reproduce el conjunto mínimo de handlers de `smallvil` (smithay 0.7), más
//! los callbacks de ciclo de vida (`toplevel_destroyed`, `app_id_changed`,
//! `title_changed`) para mantener las reglas y limpiar ventanas cerradas.

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    desktop::{PopupKind, Window},
    input::{Seat, SeatHandler, SeatState},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            protocol::{wl_buffer, wl_seat, wl_surface::WlSurface},
            Client, Resource,
        },
    },
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, with_states, CompositorClientState, CompositorHandler,
            CompositorState,
        },
        output::OutputHandler,
        selection::data_device::{
            set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
            ServerDndGrabHandler,
        },
        selection::SelectionHandler,
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
};

use crate::compositor::state::{State, WindowData, WindowId};
use crate::compositor::ClientState;

// ── Seat ────────────────────────────────────────────────────────────────
impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<State> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {}

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let dh = &self.display_handle;
        let client = focused.and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client);
    }
}
delegate_seat!(State);

// ── Data device / selection ─────────────────────────────────────────────
impl SelectionHandler for State {
    type SelectionUserData = ();
}
impl DataDeviceHandler for State {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
impl ClientDndGrabHandler for State {}
impl ServerDndGrabHandler for State {}
delegate_data_device!(State);

// ── Output ──────────────────────────────────────────────────────────────
impl OutputHandler for State {}
delegate_output!(State);

// ── Compositor / Buffer / Shm ───────────────────────────────────────────
impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .space
                .elements()
                .find(|w| w.toplevel().map(|t| t.wl_surface() == &root).unwrap_or(false))
            {
                window.on_commit();
            }
        }
        self.popups.commit(surface);
        self.center_pending_floating();
    }
}

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_compositor!(State);
delegate_shm!(State);

// ── XDG shell ───────────────────────────────────────────────────────────

fn read_toplevel_props(window: &Window) -> (String, String) {
    window
        .toplevel()
        .map(|t| {
            with_states(t.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .and_then(|d| {
                        let d = d.lock().unwrap();
                        Some((
                            d.app_id.clone().unwrap_or_default(),
                            d.title.clone().unwrap_or_default(),
                        ))
                    })
            })
            .unwrap_or_default()
        })
        .unwrap_or_default()
}

impl State {
    /// Re-aplica las reglas cuando cambian app_id/título.
    fn rules_changed(&mut self, surface: &ToplevelSurface) {
        let id = self
            .windows
            .iter()
            .find(|(_, w)| w.toplevel().map(|t| t.wl_surface() == surface.wl_surface()).unwrap_or(false))
            .map(|(id, _)| *id);
        let Some(id) = id else { return };
        let Some(window) = self.windows.get(&id).cloned() else { return };

        let (app_id, title) = read_toplevel_props(&window);
        let props = crate::WindowProps {
            app_id,
            title,
            class: String::new(),
        };

        let mut data = self.window_data.get(&id).copied().unwrap_or_default();
        let was_floating = data.floating;
        for rule in &self.config.rules {
            if rule.matcher.matches(&props) {
                data.floating = rule.float;
                data.fullscreen = rule.fullscreen;
                if let Some(ws) = rule.workspace {
                    data.workspace = ws.min(self.config.workspaces.len().saturating_sub(1));
                }
            }
        }
        // Si acaba de pasar a flotante, necesita centrado.
        if data.floating && !was_floating {
            data.centered = false;
        }
        self.window_data.insert(id, data);
        self.retile();
        self.center_pending_floating();
    }
}

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let id = self.next_client_id;
        self.next_client_id += 1;
        let window = Window::new_wayland_window(surface);

        window.user_data().insert_if_missing_threadsafe(|| WindowId(id));

        let mut data = WindowData {
            workspace: self.current_workspace,
            floating: false,
            fullscreen: false,
            centered: false,
        };

        let (app_id, title) = read_toplevel_props(&window);
        let props = crate::WindowProps {
            app_id,
            title,
            class: String::new(),
        };
        for rule in &self.config.rules {
            if rule.matcher.matches(&props) {
                data.floating = rule.float;
                data.fullscreen = rule.fullscreen;
                if let Some(ws) = rule.workspace {
                    data.workspace = ws.min(self.config.workspaces.len().saturating_sub(1));
                }
            }
        }

        // Tiled → ya posicionado por el retile; flotante → se centrará luego.
        data.centered = !data.floating;

        self.window_data.insert(id, data);
        self.windows.insert(id, window.clone());

        if data.workspace == self.current_workspace {
            self.space.map_element(window, (0, 0), false);
        }

        self.retile();
        self.focus_window(Some(id));
        self.config
            .hooks
            .dispatch(&crate::HookEvent::ClientAdded { client: id });
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        let _ = self.popups.track_popup(PopupKind::Xdg(surface));
    }

    fn reposition_request(&mut self, surface: PopupSurface, positioner: PositionerState, token: u32) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn move_request(&mut self, _surface: ToplevelSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        // En un WM de tiling, el movimiento interactivo se gestiona por keybinds.
    }

    fn resize_request(
        &mut self,
        _surface: ToplevelSurface,
        _seat: wl_seat::WlSeat,
        _serial: Serial,
        _edges: xdg_toplevel::ResizeEdge,
    ) {
        // Ídem.
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let id = self
            .windows
            .iter()
            .find(|(_, w)| w.toplevel().map(|t| t.wl_surface() == surface.wl_surface()).unwrap_or(false))
            .map(|(id, _)| *id);
        if let Some(id) = id {
            self.remove_window(id);
        }
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        self.rules_changed(&surface);
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        self.rules_changed(&surface);
    }
}
delegate_xdg_shell!(State);
