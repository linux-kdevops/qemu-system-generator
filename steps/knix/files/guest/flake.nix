# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.knix.url = "github:linux-kdevops/knix/0c95b7e564bda2d1ca91d5e8e832ea11508d97dd";
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
      packages.x86_64-linux.initrd = self.nixosConfigurations.vm.config.system.build.initialRamdisk;
    };
}
