//! Hooks al estilo qtile: callbacks que el compositor dispara ante eventos.
//!
//! En vez de un trait rígido, se usa un `Vec` de closures, que es lo más
//! ergonómico para una configuración escrita en Rust.

use crate::ClientId;

/// Eventos que el compositor publica a los hooks.
#[derive(Debug, Clone)]
pub enum HookEvent {
    Startup,
    Shutdown,
    ClientAdded { client: ClientId },
    ClientRemoved { client: ClientId },
    FocusChanged { client: Option<ClientId> },
    LayoutChanged { layout: String },
    WorkspaceChanged { workspace: usize },
    ScreenChanged,
}

/// Callback de hook.
pub type HookFn = Box<dyn Fn(&HookEvent) + Send + Sync>;

/// Conjunto de hooks. `Default` produce un conjunto vacío.
#[derive(Default)]
pub struct Hooks {
    callbacks: Vec<HookFn>,
}

impl Hooks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra un callback.
    pub fn push<F>(&mut self, f: F)
    where
        F: Fn(&HookEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(f));
    }

    /// Registra varios callbacks.
    pub fn extend<F>(&mut self, iter: impl IntoIterator<Item = F>)
    where
        F: Fn(&HookEvent) + Send + Sync + 'static,
    {
        self.callbacks.extend(iter.into_iter().map(|f| Box::new(f) as HookFn));
    }

    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }

    /// Dispara todos los callbacks con el evento dado.
    pub fn dispatch(&self, event: &HookEvent) {
        for cb in &self.callbacks {
            cb(event);
        }
    }
}
