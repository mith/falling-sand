{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    wasm-bindgen-269.url = "github:NixOS/nixpkgs?rev=9bb3fccb5b55326cb3c2c507464a8a28d44d1730";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    naersk.url = "github:nmattia/naersk";
  };

  outputs = { self, nixpkgs, utils, rust-overlay, naersk, wasm-bindgen-269, ... }:
    utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        rust = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlay
            (self: super: {
              rustc = rust;
              cargo = rust;
              wasm-bindgen-cli = (import wasm-bindgen-269 { inherit system; }).wasm-bindgen-cli;
            })
          ];
        };
        naersk-lib = naersk.lib."${system}".override {
          cargo = rust;
          rustc = rust;
        };
      in
      rec {
        packages =
          rec {
            falling-sand = naersk-lib.buildPackage {
              pname = "falling-sand";
              root = ./.;
              nativeBuildInputs = with pkgs; [
                pkg-config
              ];
              buildInputs = with pkgs; [
                alsaLib
                libudev
                xorg.libX11
                xorg.libX11
                xorg.libXcursor
                xorg.libXi
                xorg.libXrandr
                python3
              ];
            };
            falling-sand-web =
              let
                falling-sand-wasm = (falling-sand.overrideAttrs (attrs: {
                  CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
                }));
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
                  wasm-bindgen --out-dir $out --out-name wasm --target web --no-typescript ${falling-sand-wasm}/bin/falling-sand.wasm
                  cp index.html $out/index.html
                  cp -r assets $out/assets
                '';
              };
          };

        defaultPackage = packages.falling-sand;

        apps.falling-sand = utils.lib.mkApp {
          drv = packages.falling-sand;
        };
        defaultApp = apps.falling-sand;

        devShell = pkgs.mkShell {
          inputsFrom = [ packages.falling-sand ];
          RUST_SRC_PATH = "${pkgs.rust-bin.stable.latest.rust-src}/lib/rustlib/src/rust/library/";
          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
            cargo-edit
            wasm-bindgen-cli
            simple-http-server
            vulkan-loader
            lldb
          ];
        };
      });
}
