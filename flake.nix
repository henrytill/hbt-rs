{
  inputs = {
    self.submodules = true;
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }:
    let
      mkHbt =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          name = "hbt";
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          src = builtins.path {
            path = ./.;
            name = "hbt-src";
          };
          env = {
            HBT_COMMIT_HASH = "${self.rev or self.dirtyRev}";
            HBT_COMMIT_SHORT_HASH = "${self.shortRev or self.dirtyShortRev}";
          };
        };
      overlay = final: prev: {
        hbt = mkHbt final;
        hbt-static = mkHbt final.pkgsStatic;
      };
    in
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ overlay ];
        };
      in
      {
        packages = {
          hbt = pkgs.hbt;
          hbt-static = pkgs.hbt-static;
          default = self.packages.${system}.hbt;
        };
        devShells.default = pkgs.mkShell {
          inputsFrom = [ pkgs.hbt ];
          packages = with pkgs; [
            rust-analyzer
            rustfmt
            clippy
            cargo-deny
            yaml-language-server
          ];
        };
      }
    );
}
