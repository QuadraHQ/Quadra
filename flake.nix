{
  description = "Quadra Roblox launcher with a bundled Darling or Rosemary runtime";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      quadrahq = pkgs.callPackage ./packaging/nix { };
      runtimeCheck = package: {
        type = "app";
        program = "${
          pkgs.callPackage ./packaging/nix/check-runtime.nix { quadrahq = package; }
        }/bin/quadrahq-runtime-check";
        meta.description = "Check Quadra's ${package.runtime} runtime in a disposable prefix";
      };
      launcherPython = pkgs.python3.withPackages (ps: [ ps.pygobject3 ]);
    in
    {
      packages.${system} = {
        inherit quadrahq;
        default = quadrahq;
        quadrahq-regular = quadrahq.override { runtime = "regular"; };
        quadrahq-rosemary = quadrahq.override { runtime = "rosemary"; };
      };

      apps.${system} = {
        default = {
          type = "app";
          program = "${quadrahq}/bin/quadrahq";
          meta.description = quadrahq.meta.description;
        };
        check-runtime = runtimeCheck quadrahq;
        check-runtime-regular = runtimeCheck self.packages.${system}.quadrahq-regular;
        check-runtime-rosemary = runtimeCheck self.packages.${system}.quadrahq-rosemary;
      };

      overlays.default = final: prev: {
        quadrahq = final.callPackage ./packaging/nix { };
      };

      nixosModules.default = import ./packaging/nix/module.nix self;

      checks.${system} = {
        inherit quadrahq;
        regular = self.packages.${system}.quadrahq-regular;
        rosemary = self.packages.${system}.quadrahq-rosemary;
      };

      devShells.${system} = rec {
        dev-env = pkgs.mkShell {
          packages = [
            launcherPython
            pkgs.gtk4
            pkgs.libadwaita
            pkgs.gobject-introspection
            pkgs.xdg-utils
          ];
        };
        default = dev-env;
      };

      formatter.${system} = pkgs.nixfmt;
    };
}
