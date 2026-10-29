# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    {
      nixosConfigurations.bootstrap-nvme = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [ ./configuration.nix ];
      };
      packages.x86_64-linux.default =
        self.nixosConfigurations.bootstrap-nvme.config.system.build.toplevel;
    };
}
