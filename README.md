# rtile 🪟

Un **tiling window manager** para Wayland, escrito **100% en Rust**, con la
filosofía de configuración de **qtile** y la capacidad de personalización de
**Hyprland**. La configuración es **código Rust**: no hay DSL, ni JSON, ni YAML.

> Estado: *scaffold funcional*. El compositor arranca, gestiona ventanas XDG,
> aplica tiling, responde a keybinds, alterna workspaces y renderiza por GL.

## Características

- **Config 100% en Rust** — editas `crates/rtile/src/config.rs` y recompilas.
  Puedes usar bucles, closures, funciones y todo el poder del lenguaje.
- **Layouts de tiling**: `Tile` (master+stack), `Monocle`, `Columns`, `Grid`, `Floating`.
- **Keybinds** con modificadores al estilo Hyprland (`SUPER` por defecto) y
  acciones integradas (spawn, focus, swap, workspaces, fullscreen, floating…).
- **Hooks** tipo qtile (`Startup`, `FocusChanged`, `ClientAdded`, …).
- **Reglas por ventana** (`Rule`): haz flotante/fullscreen una app por `app_id`
  o título, o mándala a un workspace concreto.
- **Workspaces** múltiples con layout propio.
- **Gaps y colores** configurables.

## Requisitos

- Linux con Wayland (o WSLg). El backend por defecto es `winit` (una ventana),
  así que también sirve para desarrollar.
- **Nix** (con flakes) — recomendado — o instalar las dependencias a mano.

## Inicio rápido (con Nix)

```bash
git clone https://github.com/myticalruben/rtile   # o tu propio repo
cd rtile
nix develop          # entra al devShell (Rust + libs de Wayland/GL)
cargo run            # compila y arranca el compositor en una ventana
```

Los programas que lances desde la config (p. ej. `kitty`) se conectarán al
compositor a través de `$WAYLAND_DISPLAY`, que se configura automáticamente.

### Sin Nix

Instala las dependencias del sistema (Ubuntu):

```bash
sudo apt install -y build-essential pkg-config libwayland-dev libxkbcommon-dev \
  libgl1-mesa-dev libx11-dev libxcb1-dev libinput-dev libudev-dev libgbm-dev \
  libseat-dev libpixman-1-dev libdrm-dev xkb-data
```

y luego:

```bash
cargo run
```

## Tu configuración

Toda la config vive en [`crates/rtile/src/config.rs`](crates/rtile/src/config.rs):

```rust
let mut b = ConfigBuilder::new();
b = b
    .gap(6)
    .outer_gap(8)
    .background(Color::from_hex("#1e1e2e").unwrap())
    .layout(Tile::default())
    .layout(Monocle)
    .workspaces((1..=9).map(|i| WorkspaceSpec::new(i.to_string(), 0)).collect())
    .spawn(Mod::SUPER, "Return", "kitty")
    .keybind(Mod::SUPER, "q", Action::Quit)
    .keybind(Mod::SUPER, "h", Action::Focus(Direction::Left))
    .rule(Rule::new(WindowMatcher::app_id("firefox")).floating())
    .hook(|e| tracing::info!("evento: {e:?}"));
```

Consulta [`ARCHITECTURE.md`](ARCHITECTURE.md) para el diseño y el roadmap.

## Estructura

```
rtile/
├── crates/
│   ├── rtile-core/   # motor: lógica pura + backend smithay
│   │   └── src/
│   │       ├── compositor/   # smithay (Wayland): state, handlers, winit
│   │       ├── config.rs     # Config + ConfigBuilder
│   │       ├── layout.rs     # Layout (trait) + Tile/Monocle/Columns/Grid/Floating
│   │       ├── keybind.rs    # Mod/Key/Action/Keybind
│   │       ├── hook.rs       # HookEvent + Hooks
│   │       ├── rule.rs       # WindowMatcher + Rule
│   │       └── ...
│   └── rtile/        # binario: tu config vive aquí (src/config.rs)
├── flake.nix         # devShell Nix (Rust + Wayland/GL)
└── Cargo.toml        # workspace
```

## Licencia

MIT.
