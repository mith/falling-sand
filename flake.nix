{
  description = "falling-sand";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
        rust = fenix.packages.${system}.stable;
        crane-lib = crane.lib."${system}".overrideToolchain rust.toolchain;
        falling-sand-src = crane-lib.cleanCargoSource (crane-lib.path ./.);
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
        build-dist = {
          name,
          bin,
          executable,
        }:
          pkgs.stdenvNoCC.mkDerivation {
            inherit name;
            phases = ["installPhase"];
            installPhase = ''
              mkdir -p $out
              cp ${bin}/bin/${executable} $out/${executable}
              cp -r ${self.packages.${system}.falling-sand-assets}/assets $out/assets
              cp -r ${self.packages.${system}.falling-sand-license}/* $out/
            '';
          };
      in {
        packages = {
          falling-sand-bin = crane-lib.buildPackage {
            name = "falling-sand-bin";
            src = falling-sand-src;
            cargoExtraArgs = "--features=parallel";
            inherit buildInputs;
            inherit nativeBuildInputs;
          };

          falling-sand-bin-wasm = let
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
              cargoExtraArgs = "--features=webgpu";
              inherit nativeBuildInputs;
              doCheck = false;
            };

          falling-sand-bin-win64 = let
            target = "x86_64-pc-windows-gnu";
            toolchain = with fenix.packages.${system};
              combine [
                stable.rustc
                stable.cargo
                targets."${target}".stable.rust-std
              ];
            craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
          in
            craneLib.buildPackage {
              src = falling-sand-src;
              strictDeps = true;
              doCheck = false;
              CARGO_BUILD_TARGET = target;
              cargoExtraArgs = "--features=parallel";

              inherit nativeBuildInputs;

              depsBuildBuild = with pkgs; [
                pkgsCross.mingwW64.stdenv.cc
                pkgsCross.mingwW64.windows.pthreads
              ];
            };

          falling-sand-assets = pkgs.stdenvNoCC.mkDerivation {
            name = "falling-sand-assets";
            src = ./assets;
            phases = ["unpackPhase" "installPhase"];
            installPhase = ''
              mkdir -p $out
              cp -r $src $out/assets
            '';
          };

          falling-sand-license = pkgs.stdenvNoCC.mkDerivation {
            name = "falling-sand-license";
            src = pkgs.lib.sourceByRegex ./. [
              "LICENSE.txt"
              "COPYING"
            ];
            phases = ["unpackPhase" "installPhase"];
            installPhase = ''
              mkdir -p $out
              cp -r $src/LICENSE.txt $src/COPYING $out/
            '';
          };

          falling-sand = build-dist {
            name = "falling-sand";
            bin = self.packages.${system}.falling-sand-bin;
            executable = "falling-sand";
          };

          falling-sand-win64 = build-dist {
            name = "falling-sand-win64";
            bin = self.packages.${system}.falling-sand-bin-win64;
            executable = "falling-sand.exe";
          };

          falling-sand-web = pkgs.stdenvNoCC.mkDerivation {
            name = "falling-sand-web";
            src = ./web;
            nativeBuildInputs = [
              pkgs.wasm-bindgen-cli
              pkgs.binaryen
            ];
            phases = ["unpackPhase" "installPhase"];
            installPhase = ''
              mkdir -p $out
              wasm-bindgen --out-dir $out --out-name falling-sand --target web ${self.packages.${system}.falling-sand-bin-wasm}/bin/falling-sand.wasm
              mv $out/falling-sand_bg.wasm .
              wasm-opt -Oz -o $out/falling-sand_bg.wasm falling-sand_bg.wasm
              cp $src/* $out/
              cp -r ${self.packages.${system}.falling-sand-assets}/assets $out/assets
            '';
          };

          falling-sand-server = pkgs.writeShellScriptBin "run-falling-sand-server" ''
            ${pkgs.simple-http-server}/bin/simple-http-server -i -c=html,wasm,ttf,js -- ${self.packages.${system}.falling-sand-web}/
          '';

          default = self.packages.${system}.falling-sand;
        };

        apps = {
          falling-sand = flake-utils.lib.mkApp {
            drv = self.packages.${system}.falling-sand;
            exePath = "/falling-sand";
          };

          default = self.apps.${system}.falling-sand;
        };

        checks = {
          inherit (self.packages.${system}) falling-sand-bin;
          pre-commit-check = inputs.pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              alejandra.enable = true;
              statix.enable = true;
              rustfmt.enable = true;
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
              (rust.withComponents ["cargo" "rustc" "rust-src" "rustfmt" "clippy"])
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
