# SPDX-License-Identifier: copyleft-next-0.3.1
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in
    {
      packages.x86_64-linux = {
        keyring = pkgs.debian-archive-keyring;
        ovmf = pkgs.OVMF.fd;
      };

      devShells.x86_64-linux.default = pkgs.mkShell {
        packages = [
          pkgs.mkosi
          pkgs.apt
          pkgs.dpkg
          pkgs.gnupg
        ];
      };
    };
}
