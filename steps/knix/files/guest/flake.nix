# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.knix.url = "github:linux-kdevops/knix/8816d6c12b9b422812757d0e6ed5508e5eff8971";
  inputs.nixpkgs.follows = "knix/nixpkgs";

  outputs =
    {
      self,
      nixpkgs,
      knix,
      ...
    }@inputs:
    {
      nixosConfigurations.vm = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inherit inputs knix; };
        modules = [
          knix.nixosModules.backends.imageless
          knix.nixosModules.user
          { nixpkgs.overlays = [ knix.overlays.default ]; }
          ./default.nix
        ];
      };
      packages.x86_64-linux.toplevel = self.nixosConfigurations.vm.config.system.build.toplevel;
    };
}
