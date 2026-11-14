{
  description = "Nix flake for Quadra";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
      in
      {
        packages.quadra = pkgs.rustPlatform.buildRustPackage {
          pname = "quadra";
          version = cargoToml.package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
            lld
            clang
          ];
          buildInputs = with pkgs; [
            gtk4
            gtk4-layer-shell
            libadwaita
          ];
        };
        packages.default = self.packages.${system}.quadra;
      }
    );
}
