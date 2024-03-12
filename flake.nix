{
  description = "falling-sand";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
    crane.url = "github:ipetkov/crane";
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";

    fenix.inputs.nixpkgs.follows = "nixpkgs";
    crane.inputs.nixpkgs.follows = "nixpkgs";
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
        pkgs = nixpkgs.legacyPackages.${system};
        rust = fenix.packages.${system}.stable;
        crane-lib = crane.lib.${system}.overrideToolchain rust.toolchain;
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
          pkgs.stdenv.cc.cc.lib
        ];
        nativeBuildInputs = with pkgs; [
          mold
          clang
          pkg-config
        ];
        cross-build-bin = args @ {target, ...}: let
          toolchain = with fenix.packages.${system};
            combine [
              stable.rustc
              stable.cargo
              targets.${target}.stable.rust-std
            ];
          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
          cleanedArgs = builtins.removeAttrs args ["target"];
        in
          craneLib.buildPackage (
            cleanedArgs
            // {
              src = falling-sand-src;
              doCheck = false;
              CARGO_BUILD_TARGET = target;
              inherit nativeBuildInputs;
            }
          );
        pack-dist = {
          name,
          bin,
          executable,
        }:
          pkgs.stdenvNoCC.mkDerivation {
            inherit name;
            dontUnpack = true;
            installPhase = ''
              mkdir -p $out
              cp ${bin}/bin/${executable} $out/${executable}
              cp -r ${self.packages.${system}.falling-sand-assets}/assets $out/assets
              cp -r ${self.packages.${system}.falling-sand-license}/* $out/
            '';
          };
      in {
        packages = {
          falling-sand-deps = crane-lib.buildDepsOnly {
            pname = "falling-sand-deps";

            strictDeps = true;
            src = falling-sand-src;

            cargoExtraArgs = "--features=parallel";

            inherit buildInputs;
            inherit nativeBuildInputs;
          };

          falling-sand-bin = crane-lib.buildPackage {
            src = falling-sand-src;
            cargoArtifacts = self.packages.${system}.falling-sand-deps;
            cargoExtraArgs = "--features=parallel";
            inherit buildInputs;
            inherit nativeBuildInputs;
          };

          falling-sand-bin-wasm = cross-build-bin {
            target = "wasm32-unknown-unknown";
            RUSTFLAGS = "--cfg=web_sys_unstable_apis";
            cargoExtraArgs = "--features=webgpu";
          };

          falling-sand-bin-win64 = cross-build-bin {
            target = "x86_64-pc-windows-gnu";
            strictDeps = true;
            cargoExtraArgs = "--features=parallel";
            depsBuildBuild = with pkgs; [
              pkgsCross.mingwW64.stdenv.cc
              pkgsCross.mingwW64.windows.pthreads
            ];
          };

          falling-sand-wasm-processed = pkgs.stdenvNoCC.mkDerivation {
            name = "falling-sand-wasm-processed";

            nativeBuildInputs = [
              pkgs.wasm-bindgen-cli
              pkgs.binaryen
            ];

            dontUnpack = true;

            buildPhase = ''
              wasm-bindgen --out-dir . --out-name falling-sand --target web ${self.packages.${system}.falling-sand-bin-wasm}/bin/falling-sand.wasm
              wasm-opt -Oz -o falling-sand_bg.wasm falling-sand_bg.wasm
            '';

            installPhase = ''
              mkdir -p $out
              cp falling-sand_bg.wasm $out/
              cp falling-sand.js $out/
            '';
          };

          falling-sand-assets = pkgs.stdenvNoCC.mkDerivation {
            name = "falling-sand-assets";
            src = ./assets;
            installPhase = ''
              mkdir -p $out
              cp -r $src $out/assets
            '';
          };

          falling-sand-attribution = crane-lib.mkCargoDerivation {
            buildPhaseCargoCommand = "cargo --offline about generate about.hbs > third-party.html";

            src = pkgs.lib.cleanSourceWith {
              src = ./.;
              filter = path: type:
                type
                == "directory"
                || pkgs.lib.hasSuffix "Cargo.toml" path
                || pkgs.lib.hasSuffix "Cargo.lock" path
                || pkgs.lib.hasSuffix "about.hbs" path
                || pkgs.lib.hasSuffix "about.toml" path
                || pkgs.lib.hasSuffix "main.rs" path;
            };

            installPhase = ''
              mkdir -p $out;
              cp third-party.html $out/third-party.html
            '';

            cargoArtifacts = self.packages.${system}.falling-sand-deps;

            nativeBuildInputs = [pkgs.cargo-about];
          };

          falling-sand-license = pkgs.stdenvNoCC.mkDerivation {
            name = "falling-sand-license";
            src = pkgs.lib.sourceByRegex ./. [
              "LICENSE.txt"
              "COPYING"
              "third-party.html"
            ];
            installPhase = ''
              mkdir -p $out
              cp $src/LICENSE.txt $src/COPYING $out/
              cp ${self.packages.${system}.falling-sand-attribution}/third-party.html $out/third-party.html
            '';
          };

          falling-sand = pack-dist {
            name = "falling-sand";
            bin = self.packages.${system}.falling-sand-bin;
            executable = "falling-sand";
          };

          falling-sand-win64 = pack-dist {
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
            installPhase = ''
              mkdir -p $out
              cp $src/* $out/
              cp ${self.packages.${system}.falling-sand-wasm-processed}/falling-sand_bg.wasm $out/
              cp ${self.packages.${system}.falling-sand-wasm-processed}/falling-sand.js $out/
              cp -r ${self.packages.${system}.falling-sand-assets}/assets $out/assets
              cp -r ${self.packages.${system}.falling-sand-license}/* $out/
            '';
          };

          falling-sand-server = pkgs.writeShellScriptBin "run-falling-sand-server" ''
            ${pkgs.simple-http-server}/bin/simple-http-server -i -c=html,wasm,ttf,js -- ${self.packages.${system}.falling-sand-web}/
          '';
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

          cargo-nextest = crane-lib.cargoNextest {
            src = falling-sand-src;
            cargoArtifacts = self.packages.${system}.falling-sand-deps;
            inherit buildInputs;
            inherit nativeBuildInputs;
          };

          cargo-clippy = crane-lib.cargoClippy {
            src = falling-sand-src;
            cargoArtifacts = self.packages.${system}.falling-sand-deps;
            inherit buildInputs;
            inherit nativeBuildInputs;
          };

          pre-commit-check = inputs.pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              alejandra.enable = true;
              statix.enable = true;
              rustfmt.enable = true;
              markdownlint.enable = true;
              taplo.enable = true;
              actionlint.enable = true;
            };
          };
        };

        devShell = crane-lib.devShell {
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath buildInputs}"
            ${self.checks.${system}.pre-commit-check.shellHook}
          '';
          inputsFrom = [self.packages.${system}.falling-sand-bin];
          nativeBuildInputs = with pkgs;
            [
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
