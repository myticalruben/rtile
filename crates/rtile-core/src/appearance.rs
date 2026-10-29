//! Apariencia: colores, huecos, bordes y esquinas.

/// Color RGBA normalizado (componentes 0.0..=1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color([f32; 4]);

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r, g, b, a])
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn transparent() -> Self {
        Self([0.0, 0.0, 0.0, 0.0])
    }

    pub const fn black() -> Self {
        Self::rgb(0.0, 0.0, 0.0)
    }

    pub const fn white() -> Self {
        Self::rgb(1.0, 1.0, 1.0)
    }

    /// Parsea `"#RRGGBB"` o `"#RRGGBBAA"`.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let h = hex.trim().trim_start_matches('#');
        let byte = |s: &str| u8::from_str_radix(s, 16).ok().map(|v| v as f32 / 255.0);
        match h.len() {
            6 => Some(Self::rgb(byte(&h[0..2])?, byte(&h[2..4])?, byte(&h[4..6])?)),
            8 => Some(Self::rgba(
                byte(&h[0..2])?,
                byte(&h[2..4])?,
                byte(&h[4..6])?,
                byte(&h[6..8])?,
            )),
            _ => None,
        }
    }

    pub fn r(&self) -> f32 {
        self.0[0]
    }
    pub fn g(&self) -> f32 {
        self.0[1]
    }
    pub fn b(&self) -> f32 {
        self.0[2]
    }
    pub fn a(&self) -> f32 {
        self.0[3]
    }

    pub fn to_array(&self) -> [f32; 4] {
        self.0
    }
}

/// Opciones visuales del compositor.
#[derive(Debug, Clone)]
pub struct Appearance {
    /// Hueco entre ventanas.
    pub gap: u32,
    /// Hueco en los bordes de la pantalla.
    pub outer_gap: u32,
    /// Grosor del borde de las ventanas.
    pub border_width: u32,
    /// Color del borde de la ventana enfocada.
    pub border_focused: Color,
    /// Color del borde de las ventanas no enfocadas.
    pub border_unfocused: Color,
    /// Color de fondo.
    pub background: Color,
    /// Radio de esquina (0 = esquinas rectas; puede ignorarse en el backend GL).
    pub corner_radius: u32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            gap: 4,
            outer_gap: 4,
            border_width: 2,
            border_focused: Color::rgb(0.36, 0.55, 0.95),
            border_unfocused: Color::rgb(0.17, 0.17, 0.17),
            background: Color::rgb(0.10, 0.10, 0.10),
            corner_radius: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses_rgb() {
        assert_eq!(Color::from_hex("#FF0000").unwrap(), Color::rgb(1.0, 0.0, 0.0));
    }

    #[test]
    fn hex_parses_rgba() {
        assert_eq!(
            Color::from_hex("#00000000").unwrap(),
            Color::rgba(0.0, 0.0, 0.0, 0.0)
        );
    }
}
