{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
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
      overlay = final: prev: {
        hbt = final.pkgs.rustPlatform.buildRustPackage {
          name = "hbt";
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          src = builtins.path {
            path = ./.;
            name = "hbt-src";
          };
        };
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
        packages.hbt = pkgs.hbt;
        packages.default = self.packages.${system}.hbt;
      }
    );
}
