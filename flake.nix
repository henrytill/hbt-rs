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

        craneLib = crane.mkLib pkgs;

        muslTarget =
          builtins.replaceStrings [ "-gnu" ] [ "-musl" ]
            pkgs.stdenv.hostPlatform.rust.rustcTargetSpec;

        craneLibStatic = (crane.mkLib pkgs).overrideToolchain (
          p:
          p.rust-bin.stable.latest.default.override {
            targets = [ muslTarget ];
          }
        );

        extraExtensions = [
          ".html"
          ".jinja"
          ".json"
          ".md"
          ".xml"
          ".yaml"
        ];

        src = pkgs.lib.cleanSourceWith {
          src = self;
          filter =
            path: type:
            craneLib.filterCargoSources path type
            || builtins.any (ext: pkgs.lib.hasSuffix ext path) extraExtensions;
        };

        commonArgs = {
          pname = "hbt";
          inherit src;
          strictDeps = true;
        };

        cargoEnvStatic = {
          CARGO_BUILD_TARGET = muslTarget;
          CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        cargoArtifactsStatic = craneLibStatic.buildDepsOnly (commonArgs // cargoEnvStatic);

        mkCrate =
          pname: extra:
          commonArgs
          // {
            inherit pname cargoArtifacts;
            cargoExtraArgs = "-p ${pname}";
          }
          // extra;

        mkCrateStatic =
          pname: extra:
          commonArgs
          // cargoEnvStatic
          // {
            inherit pname;
            cargoArtifacts = cargoArtifactsStatic;
            cargoExtraArgs = "-p ${pname}";
          }
          // extra;

        crateConfigs = {
          hbt = {
            env = {
              HBT_COMMIT_HASH = "${self.rev or self.dirtyRev}";
              HBT_COMMIT_SHORT_HASH = "${self.shortRev or self.dirtyShortRev}";
            };
          };
          hbt-attic = { };
          hbt-core = { };
          hbt-pinboard = { };
          hbt-test = { };
          hbt-test-macros = { };
        };

        crates = builtins.mapAttrs mkCrate crateConfigs;

        cratesStatic = pkgs.lib.mapAttrs' (
          name: extra: pkgs.lib.nameValuePair "${name}-static" (mkCrateStatic name extra)
        ) crateConfigs;

        packages = builtins.mapAttrs (_: craneLib.buildPackage) crates;

        packagesStatic = pkgs.lib.optionalAttrs pkgs.stdenv.isLinux (
          builtins.mapAttrs (_: craneLibStatic.buildPackage) cratesStatic
        );

        checks =
          packages
          // pkgs.lib.mapAttrs' (
            name: value: pkgs.lib.nameValuePair "${name}-clippy" (craneLib.cargoClippy value)
          ) crates
          // pkgs.lib.mapAttrs' (
            name: value: pkgs.lib.nameValuePair "${name}-fmt" (craneLib.cargoFmt value)
          ) crates
          // {
            cargo-deny = craneLib.cargoDeny commonArgs;
          };
      in
      {
        inherit checks;

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
