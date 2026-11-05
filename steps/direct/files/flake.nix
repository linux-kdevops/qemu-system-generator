# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    {
      nixosConfigurations."$name" = nixpkgs.lib.nixosSystem {
        modules = [
          ./configuration.nix
          ./virtiofs.nix
          ./home.nix
        ];
      };
      packages.x86_64-linux.default = self.nixosConfigurations."$name".config.system.build.toplevel;
    };
}
