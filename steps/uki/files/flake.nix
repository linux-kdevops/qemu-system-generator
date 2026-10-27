# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      inherit (self.nixosConfigurations.bootstrap) config;
    in
    {
      nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [ ./configuration.nix ];
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
