//! Estado global del compositor y toda la lógica de tiling / foco / workspaces.

use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Arc;
use std::time::Instant;

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    desktop::{layer_map_for_output, PopupManager, Space, Window, WindowSurfaceType},
    input::{
        keyboard::{xkb, FilterResult, Keysym, ModifiersState, XkbConfig},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
        Seat, SeatState,
    },
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopSignal, Mode, PostAction},
        wayland_server::{
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle,
        },
    },
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::{
        compositor::CompositorState,
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use crate::compositor::ClientState;
use crate::geometry::Rect;
use crate::keybind::{Action, Direction, Mod};
use crate::{ClientId, Config, HookEvent, Keybind};

/// Identificador estable de una ventana, guardado en `Window::user_data()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub ClientId);

/// Estado mutable de una ventana (workspace / flags).
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowData {
    pub workspace: usize,
    pub floating: bool,
    pub fullscreen: bool,
    /// `true` si ya está posicionada (tiled, o flotante ya centrada).
    pub centered: bool,
}

/// Keybind ya resuelto (nombre de tecla → keysym numérico).
pub struct ResolvedKeybind {
    pub modifiers: Mod,
    pub keysym: Keysym,
    pub action: Action,
}

/// Estado global del compositor.
pub struct State {
    pub start_time: Instant,
    pub socket_name: OsString,
    pub display_handle: DisplayHandle,

    pub space: Space<Window>,
    pub loop_signal: LoopSignal,

    // Estado de smithay
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub wlr_layer_state: WlrLayerShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<State>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,
    pub seat: Seat<Self>,

    // rtile
    pub config: Config,
    pub keybinds: Vec<ResolvedKeybind>,
    pub windows: HashMap<ClientId, Window>,
    pub window_data: HashMap<ClientId, WindowData>,
    pub next_client_id: ClientId,
    pub current_workspace: usize,
    pub focused_client: Option<ClientId>,
}

// ── funciones libres (para evitar conflictos de borrow) ─────────────────

pub(crate) fn window_id(window: &Window) -> ClientId {
    window.user_data().get::<WindowId>().map(|w| w.0).unwrap_or(0)
}

fn modifiers_match(want: Mod, have: &ModifiersState) -> bool {
    (want.contains(Mod::SHIFT) == have.shift)
        && (want.contains(Mod::CTRL) == have.ctrl)
        && (want.contains(Mod::ALT) == have.alt)
        && (want.contains(Mod::MOD4) == have.logo)
}

fn find_keybind(binds: &[ResolvedKeybind], mods: &ModifiersState, keysym: Keysym) -> Option<Action> {
    binds
        .iter()
        .find(|b| b.keysym == keysym && modifiers_match(b.modifiers, mods))
        .map(|b| b.action.clone())
}

fn resolve_keybinds(binds: &[Keybind]) -> Vec<ResolvedKeybind> {
    binds
        .iter()
        .map(|b| ResolvedKeybind {
            modifiers: b.modifiers,
            keysym: xkb::keysym_from_name(b.key.name(), xkb::KEYSYM_NO_FLAGS),
            action: b.action.clone(),
        })
        .collect()
}

/// Ventana más próxima en `dir` según los centros (x, y) de cada ventana.
fn directional_target(
    centers: &[(ClientId, i32, i32)],
    focused: ClientId,
    dir: Direction,
) -> Option<ClientId> {
    let Some(&(_, fx, fy)) = centers.iter().find(|(id, _, _)| *id == focused) else {
        return None;
    };
    let mut best: Option<(ClientId, i32)> = None;
    for &(id, cx, cy) in centers {
        if id == focused {
            continue;
        }
        let dx = cx - fx;
        let dy = cy - fy;
        let in_dir = match dir {
            Direction::Left => dx < 0,
            Direction::Right => dx > 0,
            Direction::Up => dy < 0,
            Direction::Down => dy > 0,
        };
        if !in_dir {
            continue;
        }
        let (main, orth) = match dir {
            Direction::Left | Direction::Right => (dx.abs(), dy.abs()),
            Direction::Up | Direction::Down => (dy.abs(), dx.abs()),
        };
        let score = orth * 100 + main;
        if best.map(|(_, s)| score < s).unwrap_or(true) {
            best = Some((id, score));
        }
    }
    best.map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybind::Direction;

    #[test]
    fn directional_prefers_aligned_window() {
        let centers = vec![(1, 100, 100), (2, 0, 100), (3, 0, 500)];
        // (0,100) alineado a la izquierda gana sobre (0,500) desalineado.
        assert_eq!(directional_target(&centers, 1, Direction::Left), Some(2));
        // (0,100) no tiene nada a su izquierda → None
        assert_eq!(directional_target(&centers, 2, Direction::Left), None);
    }
}

impl State {
    pub fn new(event_loop: &mut EventLoop<State>, display: Display<State>, config: Config) -> Self {
        let start_time = Instant::now();
        let dh = display.handle();

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let wlr_layer_state = WlrLayerShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let popups = PopupManager::default();

        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "rtile");
        seat.add_keyboard(
            XkbConfig::default(),
            config.key_repeat_delay_ms as i32,
            config.key_repeat_rate as i32,
        )
        .expect("no se pudo inicializar el teclado");
        seat.add_pointer();

        let space = Space::default();

        let socket_name = Self::init_wayland_listener(display, event_loop);
        let loop_signal = event_loop.get_signal();

        let keybinds = resolve_keybinds(&config.keybinds);
        let default_ws = config.default_workspace.min(config.workspaces.len().saturating_sub(1));

        Self {
            start_time,
            display_handle: dh,
            space,
            loop_signal,
            socket_name,
            compositor_state,
            xdg_shell_state,
            wlr_layer_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            popups,
            seat,
            config,
            keybinds,
            windows: HashMap::new(),
            window_data: HashMap::new(),
            next_client_id: 1,
            current_workspace: default_ws,
            focused_client: None,
        }
    }

    fn init_wayland_listener(
        display: Display<State>,
        event_loop: &mut EventLoop<State>,
    ) -> OsString {
        let listening_socket = ListeningSocketSource::new_auto().unwrap();
        let socket_name = listening_socket.socket_name().to_os_string();

        let loop_handle = event_loop.handle();
        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("no se pudo iniciar el socket de Wayland");

        loop_handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: no soltamos el display
                    unsafe {
                        display.get_mut().dispatch_clients(state).unwrap();
                    }
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        socket_name
    }

    // ── helpers de geometría ────────────────────────────────────────────

    fn workspace_layout_index(&self, ws: usize) -> usize {
        self.config
            .workspaces
            .get(ws)
            .map(|w| w.layout)
            .unwrap_or(0)
            .min(self.config.layouts.len().saturating_sub(1))
    }

    fn output_rect(&self) -> Rect {
        let output = self.space.outputs().next().unwrap();
        let geo = self.space.output_geometry(output).unwrap();
        // Área utilizable: excluye la zona reservada por barras layer-shell.
        let zone = layer_map_for_output(output).non_exclusive_zone();
        Rect::new(
            geo.loc.x + zone.loc.x,
            geo.loc.y + zone.loc.y,
            zone.size.w.max(0) as u32,
            zone.size.h.max(0) as u32,
        )
    }

    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space.element_under(pos).and_then(|(window, location)| {
            window
                .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                .map(|(s, p)| (s, (p + location).to_f64()))
        })
    }

    fn window_under(&self, pos: Point<f64, Logical>) -> Option<Window> {
        self.space.element_under(pos).map(|(w, _)| w.clone())
    }

    // ── tiling ──────────────────────────────────────────────────────────

    pub fn retile(&mut self) {
        let ws = self.current_workspace;
        let layout_index = self.workspace_layout_index(ws);
        let full = self.output_rect();
        let outer = self.config.appearance.outer_gap as i32;
        let area = Rect::new(
            full.x + outer,
            full.y + outer,
            (full.width as i32 - 2 * outer).max(0) as u32,
            (full.height as i32 - 2 * outer).max(0) as u32,
        );
        let gap = self.config.appearance.gap;

        // 1. ventanas de tiling (no flotantes ni fullscreen)
        let mut tiling: Vec<ClientId> = Vec::new();
        for w in self.space.elements() {
            let id = window_id(w);
            let d = self.window_data.get(&id).copied().unwrap_or_default();
            if d.workspace == ws && !d.floating && !d.fullscreen {
                tiling.push(id);
            }
        }

        let arrangements = self.config.layouts[layout_index].arrange(&tiling, area);
        for (id, rect) in arrangements {
            let rect = rect.inset(gap);
            if let Some(window) = self.windows.get(&id).cloned() {
                if let Some(t) = window.toplevel() {
                    t.with_pending_state(|s| {
                        s.size = Some((rect.width as i32, rect.height as i32).into());
                    });
                    t.send_configure();
                }
                self.space.map_element(window, (rect.x, rect.y), false);
            }
        }

        // 2. ventanas a pantalla completa
        let mut fullscreen: Vec<ClientId> = Vec::new();
        for w in self.space.elements() {
            let id = window_id(w);
            let d = self.window_data.get(&id).copied().unwrap_or_default();
            if d.workspace == ws && d.fullscreen {
                fullscreen.push(id);
            }
        }
        for id in fullscreen {
            if let Some(window) = self.windows.get(&id).cloned() {
                if let Some(t) = window.toplevel() {
                    t.with_pending_state(|s| {
                        s.size = Some((full.width as i32, full.height as i32).into());
                    });
                    t.send_configure();
                }
                self.space.map_element(window, (full.x, full.y), false);
            }
        }
    }

    // ── foco ────────────────────────────────────────────────────────────

    /// Centra las ventanas flotantes que aún no tienen posición (una sola vez).
    pub fn center_pending_floating(&mut self) {
        let Some(output) = self.space.outputs().next() else {
            return;
        };
        let Some(og) = self.space.output_geometry(output) else {
            return;
        };
        let mut jobs: Vec<(ClientId, Window, Point<i32, Logical>)> = Vec::new();
        for w in self.space.elements() {
            let id = window_id(w);
            let d = self.window_data.get(&id).copied().unwrap_or_default();
            if d.floating && !d.centered {
                let bbox = w.bbox();
                if !bbox.is_empty() {
                    let cx = og.loc.x + (og.size.w - bbox.size.w) / 2;
                    let cy = og.loc.y + (og.size.h - bbox.size.h) / 2;
                    jobs.push((id, w.clone(), Point::from((cx, cy))));
                }
            }
        }
        for (id, window, pos) in jobs {
            self.space.map_element(window, pos, false);
            if let Some(d) = self.window_data.get_mut(&id) {
                d.centered = true;
            }
        }
    }

    pub fn focus_window(&mut self, id: Option<ClientId>) {
        let ids: Vec<ClientId> = self.space.elements().map(window_id).collect();

        for wid in ids {
            if let Some(window) = self.windows.get(&wid) {
                window.set_activated(Some(wid) == id);
                if let Some(t) = window.toplevel() {
                    t.send_pending_configure();
                }
            }
        }

        let serial = SERIAL_COUNTER.next_serial();
        let surface = id
            .and_then(|wid| self.windows.get(&wid))
            .and_then(|w| w.toplevel())
            .map(|t| t.wl_surface().clone());
        let keyboard = self.seat.get_keyboard().unwrap();
        keyboard.set_focus(self, surface, serial);

        self.focused_client = id;
        self.config.hooks.dispatch(&HookEvent::FocusChanged { client: id });
    }

    fn focus_cycle(&mut self, delta: i32) {
        let ws = self.current_workspace;
        let mut ids: Vec<ClientId> = Vec::new();
        for w in self.space.elements() {
            let id = window_id(w);
            let d = self.window_data.get(&id).copied().unwrap_or_default();
            if d.workspace == ws {
                ids.push(id);
            }
        }
        if ids.is_empty() {
            return;
        }
        let cur = self
            .focused_client
            .and_then(|f| ids.iter().position(|&i| i == f))
            .unwrap_or(ids.len() - 1);
        let next = (cur as i32 + delta).rem_euclid(ids.len() as i32) as usize;
        self.focus_window(Some(ids[next]));
    }

    fn focus_direction(&mut self, dir: Direction) {
        let Some(fid) = self.focused_client else {
            return;
        };
        let ws = self.current_workspace;
        let mut centers: Vec<(ClientId, i32, i32)> = Vec::new();
        for w in self.space.elements() {
            let id = window_id(w);
            let d = self.window_data.get(&id).copied().unwrap_or_default();
            if d.workspace != ws {
                continue;
            }
            let loc = self.space.element_location(w).unwrap_or_default();
            let bbox = w.bbox();
            centers.push((
                id,
                loc.x + bbox.loc.x + bbox.size.w / 2,
                loc.y + bbox.loc.y + bbox.size.h / 2,
            ));
        }
        if let Some(id) = directional_target(&centers, fid, dir) {
            self.focus_window(Some(id));
        }
    }

    // ── workspaces ──────────────────────────────────────────────────────

    pub fn switch_workspace(&mut self, ws: usize) {
        let ws = ws.min(self.config.workspaces.len().saturating_sub(1));
        if ws == self.current_workspace {
            return;
        }
        self.current_workspace = ws;

        let mapping: Vec<(ClientId, bool)> = self
            .window_data
            .iter()
            .map(|(id, d)| (*id, d.workspace == ws))
            .collect();

        for (id, visible) in mapping {
            if let Some(window) = self.windows.get(&id).cloned() {
                if visible {
                    self.space.map_element(window, (0, 0), false);
                } else {
                    self.space.unmap_elem(&window);
                }
            }
        }

        let first = self
            .window_data
            .iter()
            .find(|(_, d)| d.workspace == ws)
            .map(|(id, _)| *id);

        self.focus_window(first);
        self.retile();
        self.config.hooks.dispatch(&HookEvent::WorkspaceChanged { workspace: ws });
    }

    fn send_focused_to_workspace(&mut self, ws: usize) {
        let ws = ws.min(self.config.workspaces.len().saturating_sub(1));
        if let Some(id) = self.focused_client {
            if let Some(d) = self.window_data.get_mut(&id) {
                d.workspace = ws;
            }
            if ws != self.current_workspace {
                if let Some(window) = self.windows.get(&id).cloned() {
                    self.space.unmap_elem(&window);
                }
            }
            self.retile();
        }
    }

    // ── layouts ─────────────────────────────────────────────────────────

    fn cycle_layout(&mut self, delta: i32) {
        let n = self.config.layouts.len();
        if n == 0 {
            return;
        }
        let ws = self.current_workspace;
        let cur = self.workspace_layout_index(ws);
        let next = ((cur as i32 + delta).rem_euclid(n as i32)) as usize;
        let name = self.config.layouts[next].name().to_string();
        if let Some(w) = self.config.workspaces.get_mut(ws) {
            w.layout = next;
        }
        self.retile();
        self.config.hooks.dispatch(&HookEvent::LayoutChanged { layout: name });
    }

    fn set_layout(&mut self, index: usize) {
        let index = index.min(self.config.layouts.len().saturating_sub(1));
        let ws = self.current_workspace;
        if let Some(w) = self.config.workspaces.get_mut(ws) {
            w.layout = index;
        }
        self.retile();
    }

    fn grow_layout(&mut self, delta: f32) {
        let ws = self.current_workspace;
        let li = self.workspace_layout_index(ws);
        if let Some(layout) = self.config.layouts.get_mut(li) {
            layout.grow_main(delta);
        }
        self.retile();
    }

    fn inc_main(&mut self, delta: i32) {
        let ws = self.current_workspace;
        let li = self.workspace_layout_index(ws);
        if let Some(layout) = self.config.layouts.get_mut(li) {
            layout.inc_main(delta);
        }
        self.retile();
    }

    // ── acciones ────────────────────────────────────────────────────────

    pub fn spawn(&self, command: &str) {
        match std::process::Command::new("sh").arg("-c").arg(command).spawn() {
            Ok(_) => {}
            Err(e) => tracing::warn!("fallo al lanzar '{command}': {e}"),
        }
    }

    fn kill_focused(&mut self) {
        if let Some(id) = self.focused_client {
            if let Some(window) = self.windows.get(&id) {
                if let Some(t) = window.toplevel() {
                    t.send_close();
                }
            }
        }
    }

    fn toggle_focused(&mut self, f: impl FnOnce(&mut WindowData)) {
        if let Some(id) = self.focused_client {
            if let Some(d) = self.window_data.get_mut(&id) {
                f(d);
            }
            self.retile();
        }
    }

    pub fn remove_window(&mut self, id: ClientId) {
        if let Some(window) = self.windows.remove(&id) {
            self.space.unmap_elem(&window);
            self.window_data.remove(&id);
            if self.focused_client == Some(id) {
                self.focused_client = None;
            }
            self.config.hooks.dispatch(&HookEvent::ClientRemoved { client: id });
        }
        self.retile();
    }

    pub fn dispatch_action(&mut self, action: Action) {
        match action {
            Action::Spawn(cmd) => self.spawn(&cmd),
            Action::SpawnOnWorkspace { command, .. } => self.spawn(&command),
            Action::Kill => self.kill_focused(),
            Action::ToggleFullscreen => self.toggle_focused(|d| d.fullscreen = !d.fullscreen),
            Action::ToggleFloating => self.toggle_focused(|d| d.floating = !d.floating),
            Action::FocusNext => self.focus_cycle(1),
            Action::FocusPrevious => self.focus_cycle(-1),
            Action::Focus(dir) => self.focus_direction(dir),
            Action::Swap(_) => { /* v1: no-op (follow-up) */ }
            Action::NextLayout => self.cycle_layout(1),
            Action::PreviousLayout => self.cycle_layout(-1),
            Action::SetLayout(i) => self.set_layout(i),
            Action::GotoWorkspace(n) => self.switch_workspace(n),
            Action::NextWorkspace => {
                let n = self.current_workspace + 1;
                self.switch_workspace(n);
            }
            Action::PreviousWorkspace => {
                let n = self.current_workspace.saturating_sub(1);
                self.switch_workspace(n);
            }
            Action::SendToWorkspace(n) => self.send_focused_to_workspace(n),
            Action::IncreaseMain => self.grow_layout(0.05),
            Action::DecreaseMain => self.grow_layout(-0.05),
            Action::IncreaseMainCount => self.inc_main(1),
            Action::DecreaseMainCount => self.inc_main(-1),
            Action::Reload => { /* v1: no-op */ }
            Action::Quit => self.loop_signal.stop(),
        }
    }

    // ── input ───────────────────────────────────────────────────────────

    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);
                let keycode = event.key_code();
                let state = event.state();
                let keyboard = self.seat.get_keyboard().unwrap();

                let action = keyboard.input::<Action, _>(
                    self,
                    keycode,
                    state,
                    serial,
                    time,
                    |data, modifiers, handle| {
                        if state == KeyState::Pressed {
                            if let Some(action) =
                                find_keybind(&data.keybinds, modifiers, handle.modified_sym())
                            {
                                return FilterResult::Intercept(action);
                            }
                        }
                        FilterResult::Forward
                    },
                );

                if let Some(action) = action {
                    self.dispatch_action(action);
                }
            }
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.space.outputs().next().unwrap();
                let output_geo = self.space.output_geometry(output).unwrap();
                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.seat.get_pointer().unwrap();
                let under = self.surface_under(pos);
                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);

                if self.config.focus_follows_mouse && !pointer.is_grabbed() {
                    if let Some(window) = self.window_under(pos) {
                        let id = window_id(&window);
                        if self.focused_client != Some(id) {
                            self.focus_window(Some(id));
                        }
                    }
                }
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.seat.get_pointer().unwrap();
                let serial = SERIAL_COUNTER.next_serial();
                let button = event.button_code();
                let state = event.state();

                if ButtonState::Pressed == state && !pointer.is_grabbed() {
                    if let Some(window) = self.window_under(pointer.current_location()) {
                        self.space.raise_element(&window, true);
                        let id = window_id(&window);
                        self.focus_window(Some(id));
                    } else {
                        self.focus_window(None);
                    }
                }

                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event, .. } => {
                let source = event.source();

                let horizontal_amount = event.amount(Axis::Horizontal).unwrap_or_else(|| {
                    event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.
                });
                let vertical_amount = event.amount(Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.
                });
                let horizontal_discrete = event.amount_v120(Axis::Horizontal);
                let vertical_discrete = event.amount_v120(Axis::Vertical);

                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                if horizontal_amount != 0.0 {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);
                    if let Some(d) = horizontal_discrete {
                        frame = frame.v120(Axis::Horizontal, d as i32);
                    }
                }
                if vertical_amount != 0.0 {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    if let Some(d) = vertical_discrete {
                        frame = frame.v120(Axis::Vertical, d as i32);
                    }
                }
                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        frame = frame.stop(Axis::Vertical);
                    }
                }

                let pointer = self.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }
}
