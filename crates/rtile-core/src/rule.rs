//! Reglas por ventana: emparejan propiedades y aplican comportamiento.

/// Propiedades identificadoras de una ventana.
#[derive(Debug, Clone, Default)]
pub struct WindowProps {
    /// Wayland `app_id`.
    pub app_id: String,
    /// Título de la ventana.
    pub title: String,
    /// Clase X11 (`WM_CLASS`), útil para XWayland.
    pub class: String,
}

/// Criterio de emparejamiento (subcadena, insensible a mayúsculas por ahora simple).
#[derive(Debug, Clone, Default)]
pub struct WindowMatcher {
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub class: Option<String>,
}

impl WindowMatcher {
    pub fn app_id(app_id: impl Into<String>) -> Self {
        Self { app_id: Some(app_id.into()), ..Default::default() }
    }

    pub fn title(title: impl Into<String>) -> Self {
        Self { title: Some(title.into()), ..Default::default() }
    }

    pub fn class(class: impl Into<String>) -> Self {
        Self { class: Some(class.into()), ..Default::default() }
    }

    pub fn matches(&self, props: &WindowProps) -> bool {
        fn sub(pat: &Option<String>, hay: &str) -> bool {
            match pat {
                None => true,
                Some(p) => p.is_empty() || hay.contains(p.as_str()),
            }
        }
        sub(&self.app_id, &props.app_id)
            && sub(&self.title, &props.title)
            && sub(&self.class, &props.class)
    }
}

/// Una regla aplicada cuando una ventana coincide.
#[derive(Debug, Clone)]
pub struct Rule {
    pub matcher: WindowMatcher,
    pub float: bool,
    pub fullscreen: bool,
    pub workspace: Option<usize>,
    pub size: Option<(u32, u32)>,
    pub position: Option<(i32, i32)>,
}

impl Rule {
    pub fn new(matcher: WindowMatcher) -> Self {
        Self {
            matcher,
            float: false,
            fullscreen: false,
            workspace: None,
            size: None,
            position: None,
        }
    }

    pub fn floating(mut self) -> Self {
        self.float = true;
        self
    }

    pub fn fullscreen(mut self) -> Self {
        self.fullscreen = true;
        self
    }

    pub fn on_workspace(mut self, ws: usize) -> Self {
        self.workspace = Some(ws);
        self
    }

    pub fn with_size(mut self, w: u32, h: u32) -> Self {
        self.size = Some((w, h));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matcher_substring() {
        let m = WindowMatcher::app_id("firefox");
        assert!(m.matches(&WindowProps { app_id: "org.mozilla.firefox".into(), ..Default::default() }));
        assert!(!m.matches(&WindowProps { app_id: "kitty".into(), ..Default::default() }));
    }
}
