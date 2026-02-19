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

        buildEnv = {
          HBT_COMMIT_HASH = "${self.rev or self.dirtyRev}";
          HBT_COMMIT_SHORT_HASH = "${self.shortRev or self.dirtyShortRev}";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        mkCrate =
          pname:
          commonArgs
          // {
            inherit pname cargoArtifacts;
            env = buildEnv;
            cargoExtraArgs = "-p ${pname}";
          };

        crates = {
          hbt = mkCrate "hbt";
          hbt-core = mkCrate "hbt-core";
          hbt-pinboard = mkCrate "hbt-pinboard";
          hbt-attic = mkCrate "hbt-attic";
        };

        packages = builtins.mapAttrs (_: craneLib.buildPackage) crates;

        # Static build using musl via rust-overlay
        cargoArtifactsStatic = craneLibStatic.buildDepsOnly (
          commonArgs
          // {
            CARGO_BUILD_TARGET = muslTarget;
            CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
          }
        );

        hbt-static = craneLibStatic.buildPackage (
          commonArgs
          // {
            pname = "hbt-static";
            env = buildEnv;
            cargoArtifacts = cargoArtifactsStatic;
            cargoExtraArgs = "-p hbt";
            CARGO_BUILD_TARGET = muslTarget;
            CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
          }
        );

        checks =
          packages
          // pkgs.lib.mapAttrs' (
            name: value: pkgs.lib.nameValuePair "${name}-clippy" (craneLib.cargoClippy value)
          ) crates
          // pkgs.lib.mapAttrs' (
            name: value: pkgs.lib.nameValuePair "${name}-fmt" (craneLib.cargoFmt value)
          ) crates;
      in
      {
        inherit checks;

        packages =
          packages
          // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux { inherit hbt-static; }
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
