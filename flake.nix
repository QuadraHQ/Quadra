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
            lld
            clang
            makeWrapper
          ];

          postFixup = ''
            patchelf --add-rpath ${
              pkgs.lib.makeLibraryPath (
                with pkgs;
                [
                  libGL
                  libxkbcommon
                  vulkan-loader
                  wayland
                  xorg.libX11
                  xorg.libXcursor
                  xorg.libXi
                  xorg.libXrandr
                ]
              )
            } "$out/bin/quadra"
            wrapProgram "$out/bin/quadra" --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.xdg-utils ]}
          '';

          postInstall = ''
            install -Dm644 data/org.quadrahq.Quadra.desktop \
              "$out/share/applications/org.quadrahq.Quadra.desktop"
            install -Dm644 data/org.quadrahq.Quadra.xml \
              "$out/share/mime/packages/org.quadrahq.Quadra.xml"
            mkdir -p "$out/share/icons"
            cp -r data/icons/hicolor "$out/share/icons/"
          '';
        };
        packages.default = self.packages.${system}.quadra;
      }
    );
}
