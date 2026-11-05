# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs = {
    knix.url = "github:linux-kdevops/knix/8816d6c12b9b422812757d0e6ed5508e5eff8971";
    nixpkgs.follows = "knix/nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      knix,
      ...
    }:
    let
      system = "x86_64-linux";
    in
    {
      nixosConfigurations.vm = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { inherit knix; };
        modules = [
          knix.nixosModules.backends.imageless
          knix.nixosModules.user
          { nixpkgs.overlays = [ knix.overlays.default ]; }
          ./default.nix
        ];
      };

      packages.${system}.toplevel = self.nixosConfigurations.vm.config.system.build.toplevel;
    };
}
