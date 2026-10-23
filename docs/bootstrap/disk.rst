.. SPDX-License-Identifier: copyleft-next-0.3.1

===========================
Bootstrap from a disk image
===========================

This guide boots a `NixOS`_ guest in `QEMU`_ under your user's `systemd`_
manager from a disk image that `nixpkgs`_ builds, through firmware and a
boot loader, the way a physical machine boots: first `SeaBIOS`_ and `GRUB`_,
then `OVMF`_ and `systemd-boot`_. Its :key:`.qemu` file grows by one feature
per step, and no step needs root.

The file format is specified in
:doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` and the generator in
:doc:`qemu-system-generator(8) </man/qemu-system-generator.8>`. The outputs
below are real, trimmed to the lines that matter, with the home directory
shown as ``/home/user``.

.. include:: _prepare.rst

The guest
---------

The guest is an ordinary NixOS configuration. Create
``~/bootstrap/disk/flake.nix``:

.. code-block:: nix
   :caption: ~/bootstrap/disk/flake.nix

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
           inherit system;
           modules = [
             ./configuration.nix
             ./disk.nix
           ];
         };
         packages.${system} = {
           default = images.qemu;
           efi = images.qemu-efi;
           ovmf = nixpkgs.legacyPackages.${system}.OVMF.fd;
         };
       };
   }

``~/bootstrap/disk/configuration.nix`` sets the guest up, with the
public key from ``~/bootstrap/ssh/id_ed25519.pub`` pasted in:

.. code-block:: nix
   :caption: ~/bootstrap/disk/configuration.nix

   { modulesPath, ... }:
   {
     imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

     networking.hostName = "bootstrap";
     services.openssh.enable = true;
     users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
     users.users.root.initialHashedPassword = "";

     system.stateVersion = "26.05";
   }

The :cmd:`qemu-guest.nix` profile puts the virtio drivers in the
`initrd`_. The empty root password lets you log in on a console.

.. include:: _kernel.rst

``~/bootstrap/disk/disk.nix`` names the images and sends the
kernel's messages to the serial port:

.. code-block:: nix
   :caption: ~/bootstrap/disk/disk.nix

   {
     boot.kernelParams = [ "console=ttyS0" ];
     image.modules.qemu.image.baseName = "bootstrap";
     image.modules.qemu-efi.image.baseName = "bootstrap";
   }

nixpkgs' `image modules`_ add the boot loader and a root file system on
the disk.

.. include:: _generator.rst

.. _disk-machine:

1. A machine
============

Create ``~/.config/qemu/systemd/bootstrap.qemu``:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

   [QEMU]
   machine=q35
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio

.. include:: _start.rst

With nothing to boot, the firmware tries the disk, the CD and the
network, and gives up:

.. code-block:: console
   :caption: guest
   :class: cmd-guest

   SeaBIOS (version rel-1.17.0-0-gb52ca86e094d-prebuilt.qemu.org)
   Booting from Hard Disk...
   Boot failed: could not read the boot disk
   Booting from DVD/CD...
   Boot failed: Could not read from CDROM (code 0003)
   Booting from ROM...
   iPXE (PCI 00:02.0) starting execution...ok
   ...
   Nothing to boot: No such file or directory (http://ipxe.org/2d03e13b)
   No more network devices
   Boot failed: could not read the boot disk
   No bootable device.

.. include:: _kill.rst

.. _disk-bios:

2. A disk
=========

Build the disk image with ``nix build``. The first build takes a
couple of minutes:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/disk
   $ nix build
   $ ls result
   bootstrap.qcow2  nix-support

Give the machine the disk. ``%h`` is your home directory, one of the
`specifiers`_ systemd expands in a unit:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    nographic=
    serial=mon:stdio
   +drive=file=%h/bootstrap/disk/result/bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on

The image is in the store, which is read-only. With ``snapshot=on``, QEMU
writes to a temporary file instead, and what the guest wrote is gone
when it stops.

Regenerate and start as in :ref:`step 1 <disk-machine>`. SeaBIOS finds
the disk, GRUB loads the kernel and the initrd with the command line
from ``disk.nix``, and NixOS boots in about 18 seconds:

.. code-block:: console
   :class: cmd-guest

   SeaBIOS (version rel-1.17.0-0-gb52ca86e094d-prebuilt.qemu.org)
   Booting from Hard Disk...
   GRUB loading.
   Welcome to GRUB!
   Booting initrd of NixOS 26.11 (Zokor) (Initrd).
   ...
   [  OK  ] Found device /dev/disk/by-label/nixos.
   ...
   <<< Welcome to NixOS qcow2-26.11.20261020.02c087f (x86_64) - ttyS0 >>>

The guest answers the power button now, so stop it the normal way, which
takes 1.4 seconds:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap.service

.. _disk-uefi:

3. UEFI
=======

SeaBIOS is QEMU's default firmware. On the PC machines, a firmware in
the first flash device replaces it. Build the OVMF firmware from
`EDK II`_, and the image with an EFI system partition and systemd-boot:

.. code-block:: console
   :class: cmd-host

   $ nix build .#ovmf --out-link ovmf
   $ nix build .#efi --out-link efi

The firmware is a package output named ``fd``, so its link is ``ovmf-fd``.
The first flash device holds the firmware's code, read-only. The second
holds its variable store, which the firmware writes, so it takes
``snapshot=on`` too:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    serial=mon:stdio
   -drive=file=%h/bootstrap/disk/result/bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on
   +drive=if=pflash,format=raw,readonly=on,file=%h/bootstrap/disk/ovmf-fd/FV/OVMF_CODE.fd
   +drive=if=pflash,format=raw,snapshot=on,file=%h/bootstrap/disk/ovmf-fd/FV/OVMF_VARS.fd
   +drive=file=%h/bootstrap/disk/efi/bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on

Regenerate and start. The firmware starts the disk's boot loader, whose
menu counts down on the serial console:

.. code-block:: console
   :class: cmd-guest

   BdsDxe: loading Boot0002 "UEFI Misc Device" from PciRoot(0x0)/Pci(0x3,0x0)
   BdsDxe: starting Boot0002 "UEFI Misc Device" from PciRoot(0x0)/Pci(0x3,0x0)
   NixOS
   Reboot Into Firmware Interface
   Boot in 5s.
   ...
   <<< Welcome to NixOS efi-qcow2-26.11.20261020.02c087f (x86_64) - ttyS0 >>>

.. _disk-ssh:

4. SSH
======

QEMU's `user-mode network`_ needs no privileges, and a vsock device
gives the guest a context id, a socket address the host can reach without
any network. Forward a local port to the guest's SSH, and add the device:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    drive=file=%h/bootstrap/disk/efi/bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on
   +nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
   +device=vhost-vsock-pci-non-transitional,guest-cid=42

NixOS listens for SSH on vsock by itself, through
:cmd:`systemd-ssh-generator(8)`. Give the machine a ``Host`` block:

.. code-block:: text
   :caption: ~/bootstrap/ssh/config.d/bootstrap.conf

   Host bootstrap
       HostName vsock/42
       ProxyCommand /usr/lib/systemd/systemd-ssh-proxy %h %p
       ProxyUseFdpass yes
       IdentityFile ~/bootstrap/ssh/id_ed25519
       IdentitiesOnly yes
       User root
       StrictHostKeyChecking accept-new
       UserKnownHostsFile /dev/null

.. include:: _ssh_host.rst

Regenerate and restart with ``systemctl --user restart``. With the
``Include`` in your ``~/.ssh/config``, the machine's name is an :cmd:`ssh`
alias; without it, the forwarded port takes the key on the command line:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap uname -r
   Warning: Permanently added 'vsock/42' (ED25519) to the list of known hosts.
   6.18.57
   $ ssh -i ~/bootstrap/ssh/id_ed25519 -p 10022 root@127.0.0.1 uname -r
   6.18.57

Every change to the guest means a new image with the whole system in
it. :doc:`uki` and :doc:`kernel` keep the system in the host's store
instead.

.. _systemd: https://systemd.io
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _nixpkgs: https://github.com/NixOS/nixpkgs
.. _SeaBIOS: https://www.seabios.org/
.. _GRUB: https://www.gnu.org/software/grub/manual/grub/grub.html
.. _OVMF: https://github.com/tianocore/tianocore.github.io/wiki/OVMF
.. _systemd-boot: https://www.freedesktop.org/software/systemd/man/latest/systemd-boot.html
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _image modules: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/image/images.nix
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _EDK II: https://github.com/tianocore/edk2
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
