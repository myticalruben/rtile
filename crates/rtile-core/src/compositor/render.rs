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

    for window in space.elements() {
        let Some(loc) = space.element_location(window) else {
            continue;
        };
        let geo = window.geometry();
        if geo.is_empty() {
            continue;
        }

        if border > 0 {
            let color = if Some(window_id(window)) == focused {
                appearance.border_focused
            } else {
                appearance.border_unfocused
            };
            let buffer = SolidColorBuffer::new(
                (geo.size.w + 2 * border, geo.size.h + 2 * border),
                color.to_array(),
            );
            let bpos: Point<i32, Physical> = Point::from((
                phys_i32(loc.x - border, scale),
                phys_i32(loc.y - border, scale),
            ));
            let elem =
                SolidColorRenderElement::from_buffer(&buffer, bpos, scale, 1.0, Kind::Unspecified);
            elements.push(OutputRenderElement::Decoration(elem));
        }

        let wpos: Point<i32, Physical> =
            Point::from((phys_i32(loc.x, scale), phys_i32(loc.y, scale)));
        let surfaces: Vec<OutputRenderElement<R>> =
            AsRenderElements::<R>::render_elements(window, renderer, wpos, scale_ty, 1.0);
        elements.extend(surfaces);
    }

    elements
}
