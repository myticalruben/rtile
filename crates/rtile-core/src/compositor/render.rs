//! Elementos de render de la salida, incluidos los bordes por ventana
//! (color distinto para la ventana enfocada).

use smithay::{
    backend::renderer::{
        element::{
            solid::{SolidColorBuffer, SolidColorRenderElement},
            surface::WaylandSurfaceRenderElement,
            AsRenderElements, Kind,
        },
        ImportAll, ImportMem, Renderer, Texture,
    },
    desktop::{Space, Window},
    utils::{Physical, Point, Scale},
};

use crate::appearance::Appearance;
use crate::compositor::state::window_id;
use crate::ClientId;

smithay::backend::renderer::element::render_elements!(
    pub OutputRenderElement<R> where R: ImportAll + ImportMem;
    Window = WaylandSurfaceRenderElement<R>,
    Decoration = SolidColorRenderElement,
);

fn phys_i32(v: i32, scale: f64) -> i32 {
    (v as f64 * scale).round() as i32
}

/// Construye la lista de elementos a renderizar: por cada ventana, un borde
/// sólido (color según foco) detrás de su superficie.
pub fn build_output_elements<R>(
    space: &Space<Window>,
    focused: Option<ClientId>,
    appearance: &Appearance,
    renderer: &mut R,
    scale: f64,
) -> Vec<OutputRenderElement<R>>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + Texture + 'static,
{
    let scale_ty = Scale::from(scale);
    let border = appearance.border_width as i32;
    let mut elements: Vec<OutputRenderElement<R>> = Vec::new();

    // `space.elements()` va de abajo hacia arriba; lo invertimos para recorrer
    // de arriba hacia abajo, igual que `render_elements_for_region`.
    for window in space.elements().rev() {
        let Some(loc) = space.element_location(window) else {
            continue;
        };
        // Usamos `bbox()` (área real de la superficie), NO `geometry()` (la
        // "window geometry" declarada por el cliente, que puede diferir del
        // búfer realmente dibujado). Así el borde coincide con la superficie.
        let bbox = window.bbox();
        if bbox.is_empty() {
            continue;
        }

        // Superficie(s) primero: `render_output` dibuja en orden inverso, así
        // que lo pusheado después queda debajo.
        let wpos: Point<i32, Physical> =
            Point::from((phys_i32(loc.x, scale), phys_i32(loc.y, scale)));
        let surfaces: Vec<OutputRenderElement<R>> =
            AsRenderElements::<R>::render_elements(window, renderer, wpos, scale_ty, 1.0);
        elements.extend(surfaces);

        // Borde después: queda debajo de la superficie (solo se ve el marco).
        if border > 0 {
            let color = if Some(window_id(window)) == focused {
                appearance.border_focused
            } else {
                appearance.border_unfocused
            };
            let buffer = SolidColorBuffer::new(
                (bbox.size.w + 2 * border, bbox.size.h + 2 * border),
                color.to_array(),
            );
            let bpos: Point<i32, Physical> = Point::from((
                phys_i32(loc.x + bbox.loc.x - border, scale),
                phys_i32(loc.y + bbox.loc.y - border, scale),
            ));
            let elem =
                SolidColorRenderElement::from_buffer(&buffer, bpos, scale, 1.0, Kind::Unspecified);
            elements.push(OutputRenderElement::Decoration(elem));
        }
    }

    elements
}
