.. SPDX-License-Identifier: copyleft-next-0.3.1

============
NixOS guests
============

The disk, uki and direct guides boot an ordinary `NixOS`_ configuration,
built from a ``flake.nix`` that pins nixos-unstable, a
``configuration.nix`` they share almost line for line, and one file of
each guide's own that says how the guest boots. The `knix`_ guide builds
its configuration from `knix`_'s template instead.

.. _nixos-configuration:

The configuration
=================

``configuration.nix`` imports `nixpkgs`_' :cmd:`qemu-guest.nix` profile,
which puts the virtio and `virtiofs`_ drivers in the `initrd`_. It names
the machine ``bootstrap``, enables :cmd:`sshd <sshd(8)>`, and lets root in
with the key from ``~/bootstrap/system/ssh/id_ed25519.pub``, pasted in
where the guides show ``ssh-ed25519 AAAA... bootstrap``. The empty root
password is for logging in on a console.

.. _nixos-kernel:

The kernel
==========

The guests run the kernel NixOS picks by default, 6.18.57 here. The
`boot.kernelPackages`_ option picks another, and
``pkgs.linuxPackages_latest`` follows the newest release, 7.3 here, which
:cmd:`nix build` fetches from the binary cache like the default
(`Linux kernel`_ in the NixOS manual). The disk and uki guides keep the
default; the direct guide switches for its `microvm`_ stage. To switch,
take ``pkgs`` among the arguments of ``configuration.nix`` and set the
option; the next build puts that kernel in the guest:

.. code-block:: diff

   -{ modulesPath, ... }:
   +{ modulesPath, pkgs, ... }:
    {
      imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];
   +  boot.kernelPackages = pkgs.linuxPackages_latest;

.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _knix: https://github.com/linux-kdevops/knix
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _boot.kernelPackages: https://nixos.org/manual/nixos/stable/options#opt-boot.kernelPackages
.. _Linux kernel: https://nixos.org/manual/nixos/stable/#sec-kernel-config
.. _nixpkgs: https://github.com/NixOS/nixpkgs
.. _virtiofs: https://virtio-fs.gitlab.io
.. _microvm: https://www.qemu.org/docs/master/system/i386/microvm.html
