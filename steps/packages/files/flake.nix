# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    xfsprogs = {
      url = "git+https://git.kernel.org/pub/scm/fs/xfs/xfsprogs-dev.git?ref=for-next";
      flake = false;
    };
    xfstests = {
      url = "git+https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git?ref=for-next";
      flake = false;
    };
  };

  outputs =
    { self, nixpkgs, ... }@inputs:
    {
      overlays.default = import ./overlay.nix inputs;

      nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
        modules = [
          ./configuration.nix
          ./virtiofs.nix
          ./home.nix
          { nixpkgs.overlays = [ self.overlays.default ]; }
        ];
      };
      packages.x86_64-linux = {
        default = self.nixosConfigurations.bootstrap.config.system.build.toplevel;
        inherit (self.nixosConfigurations.bootstrap.pkgs)
          damo
          fio
          nfstest
          pynfs
          xfsprogs
          xfstests
          ;
      };
    };
}
