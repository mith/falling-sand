{
  description = "falling-sand";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-local.url = "github:mith/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages."${system}";
        toolchain = fenix.packages.${system}.stable;
        crane-lib = crane.lib."${system}";
        falling-sand-src = crane-lib.cleanCargoSource ./.;
        buildInputs = with pkgs; [
          libxkbcommon
          alsa-lib
          udev
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          libxkbcommon
          python3
          vulkan-loader
          wayland
        ];
        nativeBuildInputs = with pkgs; [
          mold
          clang
          pkg-config
        ];
      in {
        packages = {
          falling-sand-bin = crane-lib.buildPackage {
            name = "falling-sand-bin";
            src = falling-sand-src;
            inherit buildInputs;
            inherit nativeBuildInputs;
          };
          falling-sand = pkgs.stdenv.mkDerivation {
            name = "falling-sand";
            src = ./assets;
            phases = ["unpackPhase" "installPhase"];
            installPhase = ''
              mkdir -p $out
              cp ${self.packages.${system}.falling-sand-bin}/bin/falling-sand $out/falling-sand
              cp -r $src $out/assets
            '';
          };

          falling-sand-wasm = let
            target = "wasm32-unknown-unknown";
            toolchain = with fenix.packages.${system};
              combine [
                stable.rustc
                stable.cargo
                targets.${target}.stable.rust-std
              ];
            craneWasm = (crane.mkLib pkgs).overrideToolchain toolchain;
          in
            craneWasm.buildPackage {
              src = falling-sand-src;
              CARGO_BUILD_TARGET = target;
              CARGO_PROFILE = "release";
              RUSTFLAGS = "--cfg=web_sys_unstable_apis";
              inherit nativeBuildInputs;
              doCheck = false;
            };

          falling-sand-web = let
            local = import inputs.nixpkgs-local {inherit system;};
          in
            pkgs.stdenv.mkDerivation {
              name = "falling-sand-web";
              src = ./.;
              nativeBuildInputs = [
                pkgs.wasm-bindgen-cli
                pkgs.binaryen
              ];
              phases = ["unpackPhase" "installPhase"];
              installPhase = ''
                mkdir -p $out
                wasm-bindgen --out-dir $out --out-name falling-sand --target web ${self.packages.${system}.falling-sand-wasm}/bin/falling-sand.wasm
                mv $out/falling-sand_bg.wasm .
                wasm-opt -Oz -o $out/falling-sand_bg.wasm falling-sand_bg.wasm
                cp web/* $out/
                cp -r assets $out/assets
              '';
            };

          falling-sand-server = pkgs.writeShellScriptBin "run-falling-sand-server" ''
            ${pkgs.simple-http-server}/bin/simple-http-server -i -c=html,wasm,ttf,js -- ${self.packages.${system}.falling-sand-web}/
          '';
        };

        defaultPackage = self.packages.${system}.falling-sand;

        apps.falling-sand = flake-utils.lib.mkApp {
          drv = self.packages.${system}.falling-sand;
          exePath = "/falling-sand";
        };
        defaultApp = self.apps.${system}.falling-sand;

        checks = {
          pre-commit-check = inputs.pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              alejandra.enable = true;
              statix.enable = true;
              rustfmt.enable = true;
              clippy = {
                enable = false;
                entry = let
                  rust = toolchain.withComponents ["clippy"];
                in
                  pkgs.lib.mkForce "${rust}/bin/cargo-clippy clippy";
              };
            };
          };
        };

        devShell = pkgs.mkShell {
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath buildInputs}"
            ${self.checks.${system}.pre-commit-check.shellHook}
          '';
          inputsFrom = [self.packages.${system}.falling-sand-bin];
          nativeBuildInputs = with pkgs;
            [
              (toolchain.withComponents ["cargo" "rustc" "rust-src" "rustfmt" "clippy"])
              rust-analyzer
              lldb
              nil
              rr
              gdb
              tracy
            ]
            ++ nativeBuildInputs;
        };
      }
    );
}
