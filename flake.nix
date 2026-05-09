{
  inputs = {
    self.submodules = true;
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        lib = pkgs.lib;

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        extraExtensions = [
          ".html"
          ".jinja"
          ".json"
          ".md"
          ".xml"
          ".yaml"
        ];

        src = lib.cleanSourceWith {
          src = self;
          filter =
            path: type:
            craneLib.filterCargoSources path type || lib.any (lib.flip lib.hasSuffix path) extraExtensions;
        };

        commonArgs = {
          pname = "hbt";
          inherit src;
          strictDeps = true;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        env = {
          HBT_COMMIT_HASH = self.rev or self.dirtyRev;
          HBT_COMMIT_SHORT_HASH = self.shortRev or self.dirtyShortRev;
        };

        packages = {
          hbt = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts env;
            }
          );
        };

        packagesStatic = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux (
          let
            muslTarget =
              builtins.replaceStrings [ "-gnu" ] [ "-musl" ]
                pkgs.stdenv.hostPlatform.rust.rustcTargetSpec;
            craneLibStatic = (crane.mkLib pkgs).overrideToolchain (
              rustToolchain.override { targets = [ muslTarget ]; }
            );
            cargoEnvStatic = {
              CARGO_BUILD_TARGET = muslTarget;
              CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
            };
            cargoArtifactsStatic = craneLibStatic.buildDepsOnly (commonArgs // cargoEnvStatic);
          in
          {
            hbt-static = craneLibStatic.buildPackage (
              commonArgs
              // cargoEnvStatic
              // {
                inherit env;
                cargoArtifacts = cargoArtifactsStatic;
                pname = "hbt-static";
              }
            );
          }
        );

        checks =
          packages
          // packagesStatic
          // {
            cargo-clippy = craneLib.cargoClippy (commonArgs // { inherit cargoArtifacts; });
            cargo-deny = craneLib.cargoDeny commonArgs;
            cargo-fmt = craneLib.cargoFmt commonArgs;
          };
      in
      {
        inherit checks;

        formatter = pkgs.nixfmt-tree;

        packages =
          packages
          // packagesStatic
          // {
            default = packages.hbt;
          };

        devShells.default = craneLib.devShell {
          inherit checks;
          packages = with pkgs; [
            rust-analyzer
            cargo-deny
            yaml-language-server
          ];
        };
      }
    );
}
