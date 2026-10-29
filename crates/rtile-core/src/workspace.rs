//! Definición de workspaces (grupos de ventanas).

/// Un workspace: nombre + índice de layout dentro de `Config::layouts`.
#[derive(Debug, Clone)]
pub struct WorkspaceSpec {
    pub name: String,
    pub layout: usize,
}

impl WorkspaceSpec {
    pub fn new(name: impl Into<String>, layout: usize) -> Self {
        Self { name: name.into(), layout }
    }
}

/// Genera `n` workspaces numerados 1..=n con un layout por defecto.
pub fn numbered_workspaces(n: usize, default_layout: usize) -> Vec<WorkspaceSpec> {
    (1..=n)
        .map(|i| WorkspaceSpec::new(i.to_string(), default_layout))
        .collect()
}
