//! Layouts de tiling. Un `Layout` decide la geometría de cada ventana visible.
//!
//! La convención es: `windows` llega en orden de apilamiento (la enfocada al
//! final) y el layout devuelve un `(ClientId, Rect)` por ventana visible. Las
//! ventanas que no deben recolocarse (p. ej. flotantes) se omiten del resultado.

use crate::geometry::Rect;
use crate::ClientId;

/// Un algoritmo de tiling.
pub trait Layout: std::fmt::Debug + Send + Sync {
    /// Identificador estable, p. ej. `"tile"`, `"monocle"`.
    fn id(&self) -> &'static str;
    /// Nombre legible para barras / hooks.
    fn name(&self) -> &str;

    /// Reparte `windows` en `area` y devuelve la geometría de cada una.
    fn arrange(&self, windows: &[ClientId], area: Rect) -> Vec<(ClientId, Rect)>;

    /// Ajusta el ratio main/stack (solo en layouts con ratio).
    fn set_ratio(&mut self, _ratio: f32) {}
    /// Ratio actual (0.0..=1.0).
    fn ratio(&self) -> f32 {
        0.5
    }
    /// Agranda/encoge la región "main" en `delta` (fracción).
    fn grow_main(&mut self, _delta: f32) {}
    /// Aumenta/reduce el número de ventanas en la región "main".
    fn inc_main(&mut self, _delta: i32) {}
}

fn zip(windows: &[ClientId], rects: Vec<Rect>) -> Vec<(ClientId, Rect)> {
    windows.iter().copied().zip(rects).collect()
}

/// Todas las ventanas ocupan el área completa (la enfocada queda encima).
#[derive(Debug, Clone, Copy, Default)]
pub struct Monocle;

impl Layout for Monocle {
    fn id(&self) -> &'static str {
        "monocle"
    }
    fn name(&self) -> &str {
        "Monocle"
    }
    fn arrange(&self, windows: &[ClientId], area: Rect) -> Vec<(ClientId, Rect)> {
        windows.iter().copied().map(|w| (w, area)).collect()
    }
}

/// No recoloca nada: las ventanas conservan su geometría (modo flotante).
#[derive(Debug, Clone, Copy, Default)]
pub struct Floating;

impl Layout for Floating {
    fn id(&self) -> &'static str {
        "floating"
    }
    fn name(&self) -> &str {
        "Floating"
    }
    fn arrange(&self, _windows: &[ClientId], _area: Rect) -> Vec<(ClientId, Rect)> {
        Vec::new()
    }
}

/// "Master + stack": una región principal y el resto apilado.
#[derive(Debug, Clone)]
pub struct Tile {
    ratio: f32,
    main_count: usize,
    vertical: bool,
}

impl Tile {
    pub fn new() -> Self {
        Self { ratio: 0.5, main_count: 1, vertical: false }
    }

    pub fn vertical() -> Self {
        Self { ratio: 0.5, main_count: 1, vertical: true }
    }

    pub fn with_ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.clamp(0.0, 1.0);
        self
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for Tile {
    fn id(&self) -> &'static str {
        "tile"
    }
    fn name(&self) -> &str {
        if self.vertical { "Tile (vertical)" } else { "Tile" }
    }

    fn arrange(&self, windows: &[ClientId], area: Rect) -> Vec<(ClientId, Rect)> {
        if windows.is_empty() {
            return Vec::new();
        }
        if windows.len() == 1 {
            return vec![(windows[0], area)];
        }

        let main = self.main_count.min(windows.len());
        let (mains, stack) = windows.split_at(main);

        let (main_area, stack_area) = if self.vertical {
            area.split_top(self.ratio)
        } else {
            area.split_left(self.ratio)
        };

        let mut out = Vec::with_capacity(windows.len());
        let main_rects = if self.vertical {
            main_area.split_columns(main)
        } else {
            main_area.split_rows(main)
        };
        out.extend(zip(mains, main_rects));

        let stack_rects = if self.vertical {
            stack_area.split_columns(stack.len())
        } else {
            stack_area.split_rows(stack.len())
        };
        out.extend(zip(stack, stack_rects));

        out
    }

    fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(0.0, 1.0);
    }
    fn ratio(&self) -> f32 {
        self.ratio
    }
    fn grow_main(&mut self, delta: f32) {
        self.set_ratio(self.ratio + delta);
    }
    fn inc_main(&mut self, delta: i32) {
        self.main_count = (self.main_count as i32 + delta).max(1) as usize;
    }
}

/// N columnas de igual ancho; cada columna es una pila vertical.
#[derive(Debug, Clone)]
pub struct Columns {
    columns: usize,
}

impl Columns {
    pub fn new(columns: usize) -> Self {
        Self { columns: columns.max(1) }
    }
}

impl Default for Columns {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Layout for Columns {
    fn id(&self) -> &'static str {
        "columns"
    }
    fn name(&self) -> &str {
        "Columns"
    }

    fn arrange(&self, windows: &[ClientId], area: Rect) -> Vec<(ClientId, Rect)> {
        if windows.is_empty() {
            return Vec::new();
        }
        let cols = self.columns.min(windows.len());
        let mut buckets: Vec<Vec<ClientId>> = vec![Vec::new(); cols];
        for (i, w) in windows.iter().enumerate() {
            buckets[i % cols].push(*w);
        }

        let mut out = Vec::with_capacity(windows.len());
        for (bucket, col_area) in buckets.iter().zip(area.split_columns(cols)) {
            out.extend(zip(bucket, col_area.split_rows(bucket.len())));
        }
        out
    }
}

/// Rejilla de filas × columnas.
#[derive(Debug, Clone, Copy, Default)]
pub struct Grid;

impl Layout for Grid {
    fn id(&self) -> &'static str {
        "grid"
    }
    fn name(&self) -> &str {
        "Grid"
    }

    fn arrange(&self, windows: &[ClientId], area: Rect) -> Vec<(ClientId, Rect)> {
        let n = windows.len();
        if n == 0 {
            return Vec::new();
        }
        let cols = (n as f32).sqrt().ceil() as usize;
        let rows = n.div_ceil(cols);
        zip(windows, area.grid(rows, cols))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<ClientId> {
        (0..n as u64).collect()
    }

    #[test]
    fn monocle_fills_every_window() {
        let area = Rect::new(0, 0, 800, 600);
        let out = Monocle.arrange(&ids(3), area);
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|(_, r)| *r == area));
    }

    #[test]
    fn tile_splits_master_and_stack() {
        let area = Rect::new(0, 0, 800, 600);
        let out = Tile::default().arrange(&ids(3), area);
        assert_eq!(out.len(), 3);
        // el maestro ocupa la mitad izquierda a altura completa
        let master = out.iter().find(|(id, _)| *id == 0).unwrap().1;
        assert_eq!(master.x, 0);
        assert_eq!(master.width, 400);
        assert_eq!(master.height, 600);
        // la pila ocupa la mitad derecha, apilada verticalmente
        let stack_heights: u32 = out
            .iter()
            .filter(|(id, _)| *id != 0)
            .map(|(_, r)| r.height)
            .sum();
        assert_eq!(stack_heights, 600);
    }

    #[test]
    fn grid_covers_every_window() {
        let area = Rect::new(0, 0, 800, 600);
        let out = Grid.arrange(&ids(5), area);
        assert_eq!(out.len(), 5);
    }
}
