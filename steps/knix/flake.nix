# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.knix.url = "github:linux-kdevops/knix";

  outputs =
    { knix, ... }:
    let
      pkgs = knix.inputs.nixpkgs.legacyPackages.x86_64-linux;
    in
    {
      packages.x86_64-linux.busybox = pkgs.pkgsStatic.busybox;

      devShells.x86_64-linux = {
        inherit (knix.devShells.x86_64-linux) build-kernel build-qemu;
      };
    };
}
