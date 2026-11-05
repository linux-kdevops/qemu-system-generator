# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      inherit (self.nixosConfigurations.bootstrap) config pkgs;
    in
    {
      nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
        modules = [
          ./configuration.nix
          ./uki.nix
          ./home.nix
        ];
      };
      packages.${system} = {
        default = pkgs.runCommand "esp" { } ''
          mkdir --parents $out/EFI/BOOT
          cp ${config.system.build.uki}/${config.system.boot.loader.ukiFile} $out/EFI/BOOT/BOOTX64.EFI
        '';
        ovmf = pkgs.OVMF.fd;
      };
    };
}
