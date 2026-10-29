//! Configuración de ejemplo de rtile — **100% en Rust**.
//!
//! Edita este archivo a tu gusto: añade keybinds, layouts, reglas, hooks,
//! cambia colores y huecos, y recompila con `cargo run`. No hay ningún DSL ni
//! archivo de texto: todo el poder de Rust está disponible (bucles, closures,
//! funciones auxiliares, etc.), como pide una personalización nivel Hyprland.

use rtile_core::{
    Action, Color, Config, ConfigBuilder, Direction, Grid, Mod, Monocle, Rule, Tile, WindowMatcher,
    WorkspaceSpec,
};

pub fn config() -> Config {
    // Modificador principal (la tecla Super/Windows), igual que `SUPER` en Hyprland.
    let m = Mod::ALT;

    let mut b = ConfigBuilder::new();

    // ── apariencia ──────────────────────────────────────────────────────
    b = b
        .gap(6)
        .outer_gap(8)
        .border_width(2)
        .background(Color::from_hex("#1e1e2e").unwrap())
        .focus_follows_mouse(true)
        .warp_cursor_on_focus(false);

    // ── layouts (índices 0..4) ─────────────────────────────────────────
    b = b
        .layout(Tile::default())
        .layout(Monocle)
        .layout(rtile_core::Columns::new(3))
        .layout(Grid)
        .layout(rtile_core::Floating);

    // 9 workspaces numerados, todos arrancando con el layout 0 (Tile).
    b = b.workspaces(
        (1..=9)
            .map(|i| WorkspaceSpec::new(i.to_string(), 0))
            .collect(),
    );

    // ── programas al arrancar ──────────────────────────────────────────
    b = b.spawn_on_startup("kitty").spawn_on_startup("waybar");

    // ── lanzar apps ─────────────────────────────────────────────────────
    b = b
        .spawn(m, "Return", "kitty")
        .spawn(m, "d", "wofi --show drun")
        .spawn(m, "e", "dolphin")
        .spawn(m, "b", "firefox");

    // ── gestión de ventanas ────────────────────────────────────────────
    b = b
        .keybind(m, "q", Action::Quit)
        .keybind(m, "w", Action::Kill)
        .keybind(m, "f", Action::ToggleFullscreen)
        .keybind(m, "t", Action::ToggleFloating);

    // ── foco y movimiento (estilo vim) ─────────────────────────────────
    b = b
        .keybind(m, "h", Action::Focus(Direction::Left))
        .keybind(m, "l", Action::Focus(Direction::Right))
        .keybind(m, "j", Action::Focus(Direction::Down))
        .keybind(m, "k", Action::Focus(Direction::Up))
        .keybind(m | Mod::SHIFT, "h", Action::Swap(Direction::Left))
        .keybind(m | Mod::SHIFT, "l", Action::Swap(Direction::Right));

    // ── layouts ─────────────────────────────────────────────────────────
    b = b
        .keybind(m, "Tab", Action::NextLayout)
        .keybind(m | Mod::SHIFT, "Tab", Action::PreviousLayout)
        .keybind(m, "equal", Action::IncreaseMain)
        .keybind(m, "minus", Action::DecreaseMain);

    // ── workspaces 1..9 ────────────────────────────────────────────────
    for i in 1..=9usize {
        let key = i.to_string();
        b = b
            .keybind(m, key.clone(), Action::GotoWorkspace(i - 1))
            .keybind(m | Mod::SHIFT, key, Action::SendToWorkspace(i - 1));
    }

    // ── recarga en caliente ─────────────────────────────────────────────
    b = b.keybind(m, "r", Action::Reload);

    // ── reglas por ventana ─────────────────────────────────────────────
    b = b
        //        .rule(Rule::new(WindowMatcher::app_id("firefox")).floating())
        .rule(Rule::new(WindowMatcher::title("Picture-in-Picture")).floating())
        .rule(Rule::new(WindowMatcher::app_id("mpv")).floating());

    // ── hooks (callbacks tipo qtile) ───────────────────────────────────
    b = b.hook(|event| {
        tracing::info!("hook: {event:?}");
    });

    b.build()
}
