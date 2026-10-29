# Arquitectura de rtile

Este documento explica el diseño y el roadmap del window manager.

## Modelo de configuración: "config = código Rust"

A diferencia de Hyprland (archivo de texto) o qtile (Python), en rtile la
configuración es **un programa Rust** que se compila junto al compositor:

```
crates/rtile/src/config.rs  →  fn config() -> Config
                                   │
                                   ▼
rtile_core::run(config)  →  compositor (smithay)
```

La API de configuración es un *builder* fluido (`ConfigBuilder`) que expone
todo: keybinds, layouts, hooks, reglas, workspaces y apariencia. Como es Rust,
el usuario puede usar bucles, closures y funciones auxiliares sin límites.

### ¿Por qué compilar la config y no hot-reload?

- Es el modelo "100% Rust" más simple y robusto (sin `dylib`, sin ABI frágil).
- El camino a *hot-reload* es directo: mover `config.rs` a una crate `cdylib`
  cargada con `libloading` y volver a cargarla ante cambios (roadmap).

## Capas

### 1. Lógica pura (`rtile-core`, sin `smithay`)

Estos módulos no tocan Wayland y son unit-testables:

| Módulo | Contenido |
|---|---|
| `geometry` | `Rect`/`Size`/`Point` + helpers de partición (tiling) |
| `keybind` | `Mod`, `Key`, `Action`, `Keybind` |
| `layout` | trait `Layout` + `Tile`, `Monocle`, `Columns`, `Grid`, `Floating` |
| `appearance` | `Color`, `Appearance` (gaps, bordes, fondo) |
| `hook` | `HookEvent`, `Hooks` (callbacks) |
| `rule` | `WindowMatcher`, `Rule` |
| `workspace` | `WorkspaceSpec` |
| `config` | `Config`, `ConfigBuilder` |

`Layout::arrange(&[ClientId], Rect) -> Vec<(ClientId, Rect)>` es la interfaz
central: decide geometrías sin conocer el backend.

### 2. Backend (`compositor/`, sobre smithay 0.7)

Basado en el ejemplo `smallvil` de smithay (reducido y adaptado):

| Archivo | Responsabilidad |
|---|---|
| `mod.rs` | `run()`, `CalloopData`, `ClientState` |
| `state.rs` | `State` (estado global) + tiling/foco/workspaces/input/acciones |
| `handlers.rs` | handlers de Wayland (`Seat`, `Compositor`, `Xdg`, `Shm`, …) + `delegate_*!` |
| `winit.rs` | backend `winit` + render (`space::render_output`) |

- **Backend**: `winit` (una ventana) → ideal para desarrollo y para WSLg.
  El backend de producción (`udev`/DRM, para arrancar desde TTY) se añade
  activando las features `backend_udev`, `backend_drm`, `backend_gbm`,
  `backend_libinput`, `backend_session_libseat`.
- **Render**: `GlesRenderer` (OpenGL). El fondo es el `background` de la config
  y los *gaps* se logran al colocar las ventanas con hueco.

## Decisiones clave

- **Smithay 0.7.0** fijado (tag `v0.7.0`), no `main` (que ha derivado hacia una
  API distinta: `winit 0.31-beta`, `glam`, `delegate_dispatch2!`, …).
- **Feature set mínimo** (`backend_winit`, `renderer_gl`, `wayland_frontend`,
  `desktop`) para compilar rápido; se amplía según se necesite.
- **Foco de teclado** se lleva en `State.focused_client` y se sincroniza con
  `keyboard.set_focus` + `Window::set_activated`.

## Puntos de extensión

1. **Bordes por ventana (focused/unfocused)** — dibujar un
   `SolidColorRenderElement` detrás de cada superficie, siguiendo el patrón de
   `anvil` (`render_elements!` + `AsRenderElements` en un wrapper `WindowElement`).
2. **Backend `udev`/DRM** — activar features y añadir `init_udev`.
3. **Hot-reload de config** — crate `cdylib` + `libloading`.
4. **Foco direccional real** — usar geometría (centros) en vez de orden de
   apilamiento (hoy `Focus(Direction)` aproxima a `FocusNext/Previous`).
5. **Mover/redimensionar con el ratón** — implementar `PointerGrab`
   (`MoveSurfaceGrab`/`ResizeSurfaceGrab`, ver `smallvil/grabs`).

## Roadmap

- [x] Scaffold + núcleo funcional (winit, XDG, tiling, keybinds, workspaces, hooks, reglas)
- [ ] Bordes por ventana (color según foco)
- [ ] Barra/panel propio o integración (p. ej. waybar vía protocolo)
- [ ] Backend udev/DRM (producción, arrancar desde TTY)
- [ ] Hot-reload de configuración
- [ ] Animaciones (alpha/opacidad, desplazamientos)
- [ ] XWayland (para apps X11) — feature `xwayland`
