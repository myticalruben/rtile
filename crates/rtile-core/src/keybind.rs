//! Modelo de entrada: modificadores, teclas, keybinds y las acciones que disparan.
//!
//! Las teclas se referencian por su nombre de *keysym* de XKB (p. ej. `"Return"`,
//! `"a"`, `"XF86AudioRaiseVolume"`). El compositor resuelve esos nombres con
//! `xkbcommon` en tiempo de ejecución; este módulo permanece puro y sin dependencias.

use bitflags::bitflags;

bitflags! {
    /// Modificadores de teclado. `MOD4` (la tecla Super/Windows) es el modificador
    /// principal por defecto, igual que `SUPER` en Hyprland.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Mod: u16 {
        const SHIFT = 1 << 0;
        const CAPS  = 1 << 1;
        const CTRL  = 1 << 2;
        const ALT   = 1 << 3; // Alt izquierdo (Mod1)
        const MOD2  = 1 << 4;
        const MOD3  = 1 << 5;
        const MOD4  = 1 << 6; // "Super" / Windows / Command
        const MOD5  = 1 << 7; // AltGr / ISO_Level3_Shift
    }
}

impl Mod {
    /// El modificador clásico de window managers: Super (Win/Cmd).
    pub const SUPER: Mod = Mod::MOD4;
}

/// Una tecla, referenciada por su nombre de keysym XKB.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key(String);

impl Key {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Dirección para acciones de foco / movimiento / intercambio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Acciones integradas del window manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Lanza un programa (se ejecuta por shell, así `"kitty -e nvim"` funciona).
    Spawn(String),
    /// Lanza un programa en un workspace concreto.
    SpawnOnWorkspace { command: String, workspace: usize },

    /// Cierra la ventana enfocada.
    Kill,
    /// Alterna pantalla completa en la ventana enfocada.
    ToggleFullscreen,
    /// Alterna modo flotante en la ventana enfocada.
    ToggleFloating,

    /// Enfoca la ventana siguiente / anterior del workspace actual.
    FocusNext,
    FocusPrevious,
    /// Enfoca la ventana en la dirección indicada.
    Focus(Direction),

    /// Intercambia (reordena) la ventana enfocada en la dirección indicada.
    Swap(Direction),

    /// Cambia al layout siguiente / anterior del workspace actual.
    NextLayout,
    PreviousLayout,
    /// Activa un layout concreto por índice dentro de `Config::layouts`.
    SetLayout(usize),

    /// Cambia de workspace.
    GotoWorkspace(usize),
    NextWorkspace,
    PreviousWorkspace,
    /// Envía la ventana enfocada a un workspace (y opcionalmente la sigue).
    SendToWorkspace(usize),

    /// Aumenta / reduce el tamaño de la región "main" de los layouts con ratio.
    IncreaseMain,
    DecreaseMain,
    /// Aumenta / reduce el número de ventanas de la región "main".
    IncreaseMainCount,
    DecreaseMainCount,

    /// Recarga la configuración y re-ejecuta los comandos de arranque.
    Reload,
    /// Sale del compositor.
    Quit,
}

/// Un keybind: máscara de modificadores + tecla que dispara una acción.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keybind {
    pub modifiers: Mod,
    pub key: Key,
    pub action: Action,
    pub description: Option<String>,
}

impl Keybind {
    pub fn new(modifiers: Mod, key: impl Into<Key>, action: Action) -> Self {
        Self {
            modifiers,
            key: key.into(),
            action,
            description: None,
        }
    }

    /// Añade una descripción legible (útil para un futuro bar / panel).
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
