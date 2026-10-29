# rtile 🪟

Un **tiling window manager** para Wayland, escrito **100% en Rust**, con la
filosofía de configuración de **qtile** y la capacidad de personalización de
**Hyprland**. La configuración es **código Rust**: no hay DSL, ni JSON, ni YAML.

> Estado: *scaffold funcional*. El compositor arranca, gestiona ventanas XDG,
> aplica tiling, responde a keybinds, alterna workspaces y renderiza por GL.

## Características

- **Config 100% en Rust** — editas `crates/rtile-config/src/config.rs` y
  recargas en caliente (`cargo build -p rtile-config` + el keybind de recarga).
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

### Linux real (backend udev/DRM)

Para correrlo como **WM de verdad** (TTY + DRM, no en una ventana):

1. **Compila con las librerías del sistema** (en Arch):
   ```bash
   sudo pacman -S --needed rust pkg-config
   cargo build --release --workspace
   ```

2. **Ejecuta desde una TTY** (Ctrl+Alt+F2, inicia sesión):
   ```bash
   RTILE_BACKEND=udev ./target/release/rtile
   ```

3. **Instálalo como sesión** (para elegirlo en el login de SDDM):
   ```bash
   sudo install -Dm755 target/release/rtile              /usr/local/bin/rtile
   sudo install -Dm755 target/release/librtile_config.so /usr/local/bin/librtile_config.so
   sudo tee /usr/share/wayland-sessions/rtile.desktop >/dev/null <<'EOF'
   [Desktop Entry]
   Name=rtile
   Comment=Tiling Wayland window manager (configuracion 100% en Rust)
   Exec=env RTILE_BACKEND=udev /usr/local/bin/rtile
   Type=Application
   EOF
   ```

   Luego cierra sesión y elige **"rtile"** en SDDM.

> El backend `udev` necesita una GPU con DRM (virtio-gpu en VMs) y una sesión
> con acceso a la TTY (`seatd` o `logind`). El backend por defecto (`winit`)
> sigue sirviendo para desarrollar en una ventana.

## Tu configuración

Toda la config vive en [`crates/rtile-config/src/config.rs`](crates/rtile-config/src/config.rs):

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
│   ├── rtile-config/ # tu config como cdylib (hot-reload)
│   └── rtile/        # binario
├── flake.nix         # devShell Nix (Rust + Wayland/GL)
└── Cargo.toml        # workspace
```

## Licencia

MIT.
