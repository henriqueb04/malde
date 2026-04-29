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

        rustToolchain = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        runtimeLibs = with pkgs; [
          libGL
          libxkbcommon
          wayland
          # libx11 libxcursor libxrandr libxi
        ];
        buildInputs = runtimeLibs ++ [ pkgs.zenity ];
      in
      {
        packages.default = rustPlatform.buildRustPackage {
          pname = "malde";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.makeWrapper
          ];

          inherit buildInputs;

          postInstall = ''
            wrapProgram $out/bin/malde \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath runtimeLibs}
          '';
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            pkgs.gdb
            (rustToolchain.override {
              extensions = [
                "rust-analyzer"
                "rust-src"
                "rustfmt"
              ];
            })
          ];

          inherit buildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath runtimeLibs}
          '';
        };
      }
    );
}
