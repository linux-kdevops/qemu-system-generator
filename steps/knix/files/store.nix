# SPDX-License-Identifier: copyleft-next-0.3.1
# Nix records each store path of the build-qemu shell's inputs that the
# copy names, so the libraries QEMU loads stay with it.
{ destdir }:
let
  flake = builtins.getFlake (toString ../.);
  system = builtins.currentSystem;
  pkgs = flake.inputs.knix.inputs.nixpkgs.legacyPackages.${system};
in
pkgs.runCommandLocal "qemu" {
  inherit (flake.devShells.${system}.build-qemu) inputDerivation;
  src = builtins.path {
    path = destdir;
    name = "qemu-destdir";
  };
} "cp --recursive $src $out"
