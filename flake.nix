{
  description = "MAL interpreter.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default =
          with pkgs; mkShell {
            buildInputs = [
              gdb
              lldb
              (rust-bin.stable.latest.default.override {
                extensions = [ "rust-analyzer" "rust-src" "rustfmt" ];
              })
              libGL libxkbcommon wayland
              # libx11 libxcursor libxrandr libxi
              zenity
            ];
            # NIXOS_OZONE_WL = "1";
            shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath (with pkgs; [
              libGL libxkbcommon wayland
            ])}
            '';
          };
      }
    );
}
