.. SPDX-License-Identifier: copyleft-next-0.3.1

The guest runs the kernel NixOS picks by default, 6.18.57 here. The
`boot.kernelPackages`_ option picks another, and
``pkgs.linuxPackages_latest`` follows the newest release, 7.3 here, which
:cmd:`nix build` fetches from the binary cache like the default
(`Linux kernel`_ in the NixOS manual). This guide keeps the default. To
switch, take ``pkgs`` among the arguments of ``configuration.nix`` and
set the option; the next build puts that kernel in the guest:

.. code-block:: diff

   -{ modulesPath, ... }:
   +{ modulesPath, pkgs, ... }:
    {
      imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];
   +  boot.kernelPackages = pkgs.linuxPackages_latest;

.. _boot.kernelPackages: https://nixos.org/manual/nixos/stable/options#opt-boot.kernelPackages
.. _Linux kernel: https://nixos.org/manual/nixos/stable/#sec-kernel-config
