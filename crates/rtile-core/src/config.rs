//! Configuración del window manager, construida con un builder ergonómico.
//!
//! Esta es la "superficie de personalización": el usuario expresa TODO su
//! setup (keybinds, layouts, hooks, reglas, apariencia) en Rust puro.

use crate::appearance::Appearance;
use crate::hook::{HookEvent, Hooks};
use crate::keybind::{Action, Key, Keybind, Mod};
use crate::layout::{Columns, Floating, Grid, Layout, Monocle, Tile};
use crate::rule::Rule;
use crate::workspace::{numbered_workspaces, WorkspaceSpec};

/// La configuración completa del compositor.
pub struct Config {
    pub keybinds: Vec<Keybind>,
    pub layouts: Vec<Box<dyn Layout>>,
    pub hooks: Hooks,
    pub rules: Vec<Rule>,
    pub workspaces: Vec<WorkspaceSpec>,
    /// Comandos a ejecutar al arrancar (p. ej. `["kitty", "waybar"]`).
    pub startup: Vec<String>,
    pub appearance: Appearance,
    pub default_workspace: usize,
    pub focus_follows_mouse: bool,
    pub warp_cursor_on_focus: bool,
    pub key_repeat_delay_ms: u32,
    pub key_repeat_rate: u32,
}

impl Default for Config {
    fn default() -> Self {
        let layouts: Vec<Box<dyn Layout>> = vec![
            Box::new(Tile::default()),
            Box::new(Monocle),
            Box::new(Columns::default()),
            Box::new(Grid),
            Box::new(Floating),
        ];
        Self {
            keybinds: Vec::new(),
            layouts,
            hooks: Hooks::default(),
            rules: Vec::new(),
            workspaces: numbered_workspaces(9, 0),
            startup: Vec::new(),
            appearance: Appearance::default(),
            default_workspace: 0,
            focus_follows_mouse: true,
            warp_cursor_on_focus: false,
            key_repeat_delay_ms: 200,
            key_repeat_rate: 25,
        }
    }
}

/// Builder de `Config` con API fluida.
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self { config: Config::default() }
    }

    pub fn from(config: Config) -> Self {
        Self { config }
    }

    // ── keybinds ────────────────────────────────────────────────────────
    pub fn keybind(mut self, modifiers: Mod, key: impl Into<Key>, action: Action) -> Self {
        self.config.keybinds.push(Keybind::new(modifiers, key, action));
        self
    }

    /// Atajo para lanzar un programa.
    pub fn spawn(self, modifiers: Mod, key: impl Into<Key>, command: impl Into<String>) -> Self {
        self.keybind(modifiers, key, Action::Spawn(command.into()))
    }

    // ── layouts ─────────────────────────────────────────────────────────
    pub fn layout(mut self, layout: impl Layout + 'static) -> Self {
        self.config.layouts.push(Box::new(layout));
        self
    }

    pub fn layouts(mut self, iter: impl IntoIterator<Item = Box<dyn Layout>>) -> Self {
        self.config.layouts.extend(iter);
        self
    }

    // ── hooks ───────────────────────────────────────────────────────────
    pub fn hook<F>(mut self, f: F) -> Self
    where
        F: Fn(&HookEvent) + Send + Sync + 'static,
    {
        self.config.hooks.push(f);
        self
    }

    // ── rules ───────────────────────────────────────────────────────────
    pub fn rule(mut self, rule: Rule) -> Self {
        self.config.rules.push(rule);
        self
    }

    // ── workspaces ──────────────────────────────────────────────────────
    pub fn workspace(mut self, spec: WorkspaceSpec) -> Self {
        self.config.workspaces.push(spec);
        self
    }

    pub fn workspaces(mut self, specs: Vec<WorkspaceSpec>) -> Self {
        self.config.workspaces = specs;
        self
    }

    // ── startup ─────────────────────────────────────────────────────────
    pub fn spawn_on_startup(mut self, command: impl Into<String>) -> Self {
        self.config.startup.push(command.into());
        self
    }

    // ── apariencia ──────────────────────────────────────────────────────
    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.config.appearance = appearance;
        self
    }

    pub fn gap(mut self, gap: u32) -> Self {
        self.config.appearance.gap = gap;
        self
    }

    pub fn outer_gap(mut self, gap: u32) -> Self {
        self.config.appearance.outer_gap = gap;
        self
    }

    pub fn border_width(mut self, width: u32) -> Self {
        self.config.appearance.border_width = width;
        self
    }

    pub fn background(mut self, color: crate::Color) -> Self {
        self.config.appearance.background = color;
        self
    }

    // ── comportamiento ──────────────────────────────────────────────────
    pub fn default_workspace(mut self, ws: usize) -> Self {
        self.config.default_workspace = ws.min(self.config.workspaces.len().saturating_sub(1));
        self
    }

    pub fn focus_follows_mouse(mut self, enabled: bool) -> Self {
        self.config.focus_follows_mouse = enabled;
        self
    }

    pub fn warp_cursor_on_focus(mut self, enabled: bool) -> Self {
        self.config.warp_cursor_on_focus = enabled;
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
