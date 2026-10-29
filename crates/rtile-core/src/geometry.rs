//! Geometría 2D mínima usada por los layouts y la configuración.
//!
//! Estos tipos son deliberadamente independientes de `smithay`, de modo que la
//! lógica de tiling se puede probar por unidad sin tener Wayland/GL/DRM.

/// Posición en píxeles enteros (esquina superior izquierda).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Tamaño en píxeles enteros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Rectángulo alineado a los ejes. `x`/`y` son la esquina superior izquierda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    #[inline]
    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    #[inline]
    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// ¿Contiene este rectángulo al punto `p`?
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    /// Encoge el rectángulo `gap` píxeles por cada lado (nunca por debajo de 0).
    pub fn inset(&self, gap: u32) -> Rect {
        let g = gap as i32;
        let x = self.x + g;
        let y = self.y + g;
        let width = (self.width as i32 - 2 * g).max(0) as u32;
        let height = (self.height as i32 - 2 * g).max(0) as u32;
        Rect::new(x, y, width, height)
    }

    // ─────────────────────────────────────────────────────────────────────
    // Helpers de partición usados por los layouts de tiling.
    // ─────────────────────────────────────────────────────────────────────

    /// Parte el rectángulo verticalmente: izquierda `ratio`, derecha el resto.
    pub fn split_left(&self, ratio: f32) -> (Rect, Rect) {
        let r = ratio.clamp(0.0, 1.0);
        let left_w = (self.width as f32 * r).round() as u32;
        let left_w = left_w.min(self.width);
        let right_w = self.width - left_w;
        let left = Rect::new(self.x, self.y, left_w, self.height);
        let right = Rect::new(self.x + left_w as i32, self.y, right_w, self.height);
        (left, right)
    }

    /// Parte el rectángulo horizontalmente: arriba `ratio`, abajo el resto.
    pub fn split_top(&self, ratio: f32) -> (Rect, Rect) {
        let r = ratio.clamp(0.0, 1.0);
        let top_h = (self.height as f32 * r).round() as u32;
        let top_h = top_h.min(self.height);
        let bottom_h = self.height - top_h;
        let top = Rect::new(self.x, self.y, self.width, top_h);
        let bottom = Rect::new(self.x, self.y + top_h as i32, self.width, bottom_h);
        (top, bottom)
    }

    /// Divide el rectángulo en `n` filas de igual alto (apiladas verticalmente).
    pub fn split_rows(&self, n: usize) -> Vec<Rect> {
        if n == 0 {
            return Vec::new();
        }
        let n = n as u32;
        let mut out = Vec::with_capacity(n as usize);
        let base = self.height / n;
        let rem = self.height % n;
        let mut y = self.y;
        for i in 0..n {
            let h = base + if i < rem { 1 } else { 0 };
            out.push(Rect::new(self.x, y, self.width, h));
            y += h as i32;
        }
        out
    }

    /// Divide el rectángulo en `n` columnas de igual ancho.
    pub fn split_columns(&self, n: usize) -> Vec<Rect> {
        if n == 0 {
            return Vec::new();
        }
        let n = n as u32;
        let mut out = Vec::with_capacity(n as usize);
        let base = self.width / n;
        let rem = self.width % n;
        let mut x = self.x;
        for i in 0..n {
            let w = base + if i < rem { 1 } else { 0 };
            out.push(Rect::new(x, self.y, w, self.height));
            x += w as i32;
        }
        out
    }

    /// Rejilla de `rows` filas por `cols` columnas.
    pub fn grid(&self, rows: usize, cols: usize) -> Vec<Rect> {
        if rows == 0 || cols == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(rows * cols);
        for row in self.split_rows(rows) {
            out.extend(row.split_columns(cols));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_left_preserves_area() {
        let r = Rect::new(0, 0, 100, 50);
        let (a, b) = r.split_left(0.6);
        assert_eq!(a.width, 60);
        assert_eq!(b.width, 40);
        assert_eq!(a.width + b.width, 100);
        assert_eq!(a.height, 50);
        assert_eq!(b.x, 60);
    }

    #[test]
    fn split_rows_distributes_remainder() {
        let r = Rect::new(0, 0, 10, 10);
        let rows = r.split_rows(3);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().map(|r| r.height).sum::<u32>(), 10);
    }

    #[test]
    fn grid_fills_area() {
        let r = Rect::new(0, 0, 200, 100);
        let cells = r.grid(2, 3);
        assert_eq!(cells.len(), 6);
    }
}
