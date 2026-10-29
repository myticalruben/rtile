{
  description = "rtile — tiling Wayland window manager en Rust (config 100% en Rust)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    # rust-overlay da toolchains oficiales de rust-lang (stable y nightly).
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];

      forAllSystems = f: nixpkgs.lib.genAttrs systems (system:
        f (import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        }));
    in
    {
      devShells = forAllSystems (pkgs:
        let
          inherit (pkgs) lib;

          # Toolchain stable con lo necesario para un compositor Wayland.
          stable = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          };

          # Herramientas que quiero en TODOS los shells.
          comun = with pkgs; [
            pkg-config      # para que los crates -sys encuentren libs del sistema
            cargo-nextest
            taplo           # LSP + formateador de TOML
            bacon           # `cargo check` en watch
          ];

          # Bibliotecas que smithay/winit cargan en RUNTIME con dlopen().
          # No basta con enlazarlas: hay que ponerlas en LD_LIBRARY_PATH.
          runtimeLibs = with pkgs; [
            libGL
            mesa            # drivers GL (lavapipe incluido)
            wayland
            libxkbcommon
            libdrm
            libgbm          # GBM para el backend udev/DRM
            libx11 libxcursor libxi libxrandr libxcb
            fontconfig
            libinput
            udev
            seatd           # libseat para el backend udev/DRM
            pixman          # para el renderer software
            xkeyboard_config
          ];

          avisoRustup = ''
            cargo_actual="$(command -v cargo 2>/dev/null || true)"
            case "$cargo_actual" in
              ""|/nix/store/*) : ;;
              *) echo "  ⚠  cargo NO viene de Nix sino de $cargo_actual"
                 echo "     (rustup, mise, asdf o sdkman se cuelan por delante)." ;;
            esac
          '';
        in
        {
          default = pkgs.mkShell {
            name = "rtile";
            packages = [ stable pkgs.mold pkgs.cargo-watch ] ++ comun ++ runtimeLibs;

            RUST_SRC_PATH = "${stable}/lib/rustlib/src/rust/library";

            LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibs;

            shellHook = ''
              echo ""
              echo "  🪟 rtile · Rust $(rustc --version | cut -d' ' -f2)  ·  shell: wayland/wm"
              echo "     └─ smithay (Wayland) + backend winit + renderer GL"
              ${avisoRustup}
              echo ""
            '';
          };
        });

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
