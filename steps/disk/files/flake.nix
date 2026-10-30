# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      inherit (self.nixosConfigurations.bootstrap.config.system.build) images;
    in
    {
      nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
        modules = [
          ./configuration.nix
          ./home.nix
        ];
      };
      packages.${system} = {
        efi = images.qemu-efi;
        ovmf = self.nixosConfigurations.bootstrap.pkgs.OVMF.fd;
      };
    };
}
