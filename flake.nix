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
      makeHbt =
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
        };
    in
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.hbt = makeHbt pkgs;
        packages.default = self.packages.${system}.hbt;
      }
    );
}
