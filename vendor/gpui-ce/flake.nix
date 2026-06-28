{
  description = "Standalone GPUI build";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      crane,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;

        toolchain = fenix.packages.${system}.latest.withComponents [
          "cargo"
          "rustc"
          "rust-src"
          "rustfmt"
          "clippy"
        ];

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        src = lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter =
            path: type:
            (craneLib.filterCargoSources path type)
            || (lib.hasSuffix ".metal" path)
            || (lib.hasSuffix ".wgsl" path)
            || (lib.hasSuffix ".hlsl" path)
            || (lib.hasSuffix ".glsl" path);
        };

        linuxLibs = with pkgs; [
          alsa-lib
          libdrm
          mesa # provides libgbm
          libxkbcommon
          libva
          vulkan-loader
          wayland
          xorg.libX11
          xorg.libxcb
        ];

        commonArgs = {
          pname = "gpui-ce";
          version = "0.3.3";

          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            cmake
            pkg-config
            rustPlatform.bindgenHook
          ];

          buildInputs =
            with pkgs;
            [
              fontconfig
              freetype
              openssl
              zlib
            ]
            ++ lib.optionals stdenv.isLinux linuxLibs
            ++ lib.optionals stdenv.isDarwin [
              apple-sdk_15
              (darwinMinVersionHook "11.0")
            ];

          env = lib.optionalAttrs pkgs.stdenv.isLinux {
            LD_LIBRARY_PATH = lib.makeLibraryPath linuxLibs;
          };

          cargoExtraArgs = "--features runtime_shaders";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        gpui = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
          }
        );
      in
      {
        packages.default = gpui;

        devShells.default = pkgs.mkShell {
          inputsFrom = [ gpui ];
          packages = [
            toolchain
            pkgs.cargo-machete
            pkgs.taplo
            pkgs.typos
            pkgs.just
          ];

          shellHook = ''
            export RUST_BACKTRACE=1
            export RUST_SRC_PATH="${toolchain}/lib/rustlib/src/rust/library"
            ${lib.optionalString pkgs.stdenv.isLinux ''
              export LD_LIBRARY_PATH="${lib.makeLibraryPath linuxLibs}:$LD_LIBRARY_PATH"
            ''}
          '';
        };
      }
    );
}
