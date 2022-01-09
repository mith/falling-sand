{
  description = "falling-sand";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, naersk, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages."${system}";
        toolchain = fenix.packages.${system}.stable;
        naersk-lib = naersk.lib."${system}";
      in
      rec {
        packages.falling-sand = naersk-lib.buildPackage {
          pname = "falling-sand";
          root = ./.;
          buildInputs = with pkgs; [
            libxkbcommon
          ];
          nativeBuildInputs = with pkgs; [
            pkg-config
            alsaLib
            libudev
            xorg.libX11
            xlibs.libX11
            xlibs.libXcursor
            xlibs.libXi
            xlibs.libXrandr
            libxkbcommon
            python3
            vulkan-loader
            wayland
            mold
          ];
        };
        packages.falling-sand-web =
          let
            target = "wasm32-unknown-unknown";
            toolchain = with fenix.packages.${system};
              combine [
                minimal.rustc
                minimal.cargo
                targets.${target}.latest.rust-std
              ];
            falling-sand-wasm = (naersk.lib.${system}.override {
              cargo = toolchain;
              rustc = toolchain;
            }).buildPackage
              {
                src = ./.;
                cargoBuildOptions = old: old ++ [ "--target wasm32-unknown-unknown"];
                nativeBuildInputs = with pkgs; [
                  pkg-config
                ];
                buildInputs = with pkgs; [
                ];
              };
          in
          pkgs.stdenv.mkDerivation {
            name = "falling-sand-web";
            src = ./.;
            nativeBuildInputs = with pkgs; [
              wasm-bindgen-cli
            ];
            phases = [ "unpackPhase" "installPhase" ];
            installPhase = ''
              mkdir -p $out
              wasm-bindgen --out-dir $out --out-name wasm --target no-modules --no-typescript ${falling-sand-wasm}/bin/falling-sand.wasm
              cp index.html $out/index.html
              cp -r assets $out/assets
            '';
          };

        defaultPackage = packages.falling-sand;

        apps.falling-sand = flake-utils.lib.mkApp {
          drv = packages.falling-sand;
        };
        defaultApp = apps.falling-sand;

        devShell = pkgs.mkShell.override { stdenv = pkgs.clangStdenv; } {
          buildInputs = with pkgs; [ llvmPackages.libclang ];
          shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath (with pkgs; [
            alsaLib
            udev
            vulkan-loader
            libxkbcommon
            wayland
          ])}"'';
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          inputsFrom = [ packages.falling-sand ];
          nativeBuildInputs = [
            pkgs.cargo-edit
            (toolchain.withComponents [ "cargo" "rustc" "rust-src" "rustfmt" "clippy" ])
          ];
        };
      }
    );
}
