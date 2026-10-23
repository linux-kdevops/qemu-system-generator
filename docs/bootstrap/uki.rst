.. SPDX-License-Identifier: copyleft-next-0.3.1

==================================
Bootstrap from a UKI in the store
==================================

This guide boots a `NixOS`_ guest in `QEMU`_ under your user's `systemd`_
manager with no disk image and no ``kernel=`` line. The `OVMF`_ firmware
loads a `unified kernel image`_ (UKI), one EFI binary that holds the kernel,
the `initrd`_ and the kernel command line, from a directory the host shares
over `virtiofs`_. The system it boots stays in the host's ``/nix/store``,
shared the same way. Its :key:`.qemu` file grows by one feature per step,
and no step needs root.

The file format is specified in
:doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` and the generator in
:doc:`qemu-system-generator(8) </man/qemu-system-generator.8>`. The outputs
below are real, trimmed to the lines that matter, with the home directory
shown as ``/home/user``.

.. include:: _prepare.rst

The guest
---------

The guest is an ordinary NixOS configuration, and the UKI is built from
it. Create ``~/bootstrap/uki/flake.nix``:

.. code-block:: nix
   :caption: ~/bootstrap/uki/flake.nix

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
           modules = [
             ./configuration.nix
             ./uki.nix
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

NixOS's `UKI module`_ builds ``system.build.uki`` from the configuration's
kernel and initrd, with the command line
``init=<system>/init`` followed by ``boot.kernelParams``. The default
package puts it where the firmware looks on a medium without boot
entries, ``\EFI\BOOT\BOOTX64.EFI``.

``~/bootstrap/uki/configuration.nix`` sets the guest up, with the
public key from ``~/bootstrap/ssh/id_ed25519.pub`` pasted in:

.. code-block:: nix
   :caption: ~/bootstrap/uki/configuration.nix

   { modulesPath, ... }:
   {
     imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

     networking.hostName = "bootstrap";
     services.openssh.enable = true;
     users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
     users.users.root.initialHashedPassword = "";

     system.stateVersion = "26.05";
   }

The :cmd:`qemu-guest.nix` profile puts the virtio and virtiofs drivers in
the initrd. The empty root password lets you log in on a console.

``~/bootstrap/uki/uki.nix`` has no boot loader and no disk. The
root is a `tmpfs`_, and the store arrives over virtiofs under the tag
``store``, which :ref:`step 2 <uki-store>` sets up:

.. code-block:: nix
   :caption: ~/bootstrap/uki/uki.nix

   {
     boot.kernelParams = [ "console=ttyS0" ];
     boot.loader.grub.enable = false;
     fileSystems."/" = {
       device = "none";
       fsType = "tmpfs";
       options = [ "mode=0755" ];
     };
     fileSystems."/nix/store" = {
       device = "store";
       fsType = "virtiofs";
     };
     systemd.services.systemd-boot-random-seed.enable = false;
   }

The UKI's stub sets the ``StubFeatures`` EFI variable, which makes
``systemd-boot-random-seed.service`` try to write a seed to the EFI system
partition. The guest has none mounted, and the unit fails with
"Failed to start Update Boot Loader Random Seed"; the last line masks it.

Build the UKI and the firmware with :cmd:`nix build`:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/uki
   $ nix build
   $ nix build .#ovmf --out-link ovmf
   $ ls -l result/EFI/BOOT
   -r--r--r-- 1 root root 41664512 Jan  1  1970 BOOTX64.EFI

The firmware is a package output named ``fd``, so its link is ``ovmf-fd``.

.. include:: _generator.rst

.. _uki-esp:

1. The firmware and the UKI
===========================

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
   drive=if=pflash,format=raw,readonly=on,file=%h/bootstrap/uki/ovmf-fd/FV/OVMF_CODE.fd
   drive=if=pflash,format=raw,snapshot=on,file=%h/bootstrap/uki/ovmf-fd/FV/OVMF_VARS.fd

   [virtiofsd]
   shared-dir=%h/bootstrap/uki/result/
   tag=esp
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

.. include:: _start.rst

On the PC machines, a firmware in the first flash device replaces QEMU's
default `SeaBIOS`_. The first flash device holds OVMF's code, read-only.
The second holds its variable store, which the firmware writes; with
``snapshot=on``, QEMU writes to a temporary file instead.

Each :key:`[virtiofsd]` section runs one `virtiofsd`_ daemon for one shared
directory. Its keys are the long options of ``virtiofsd --help`` without
the dashes. The generator runs the daemon from its own socket unit and
adds the QEMU lines that connect to it. ``sandbox=``, ``uid-map=`` and
``gid-map=`` let it run without root: ``%U`` and ``%G`` are your user and
group ids, which the guest's root maps to, and ``%h`` is your home
directory, among the `specifiers`_ systemd expands in a unit.

With the trailing slash on ``shared-dir=``, the path resolves through
the ``result`` link when the daemon starts, so every start serves the
latest build. Without it, the sandbox cannot bind-mount the link and the daemon
exits with
``Error entering sandbox: BindMountSharedDir(... "Invalid argument" })``.

OVMF's `EDK II`_ code includes a virtiofs driver, and its boot manager
adds a boot option for a file system that is not on a block device.
That option starts ``BOOTX64.EFI``. The kernel boots, and the initrd
stops where it looks for the store:

.. code-block:: console
   :caption: guest
   :class: cmd-guest

   BdsDxe: loading Boot0002 "UEFI Non-Block Boot Device" from PciRoot(0x0)/Pci(0x3,0x0)
   BdsDxe: starting Boot0002 "UEFI Non-Block Boot Device" from PciRoot(0x0)/Pci(0x3,0x0)
   Booting initrd of NixOS 26.11 (Zokor) (Initrd).
   ...
   [FAILED] Failed to mount /sysroot/nix/store.
   [DEPEND] Dependency failed for Find NixOS closure.
   ...
   You are in emergency mode. After logging in, type "journalctl -xb" to view

.. include:: _kill.rst

.. _uki-store:

2. The store
============

Share the host's store under the tag ``store``:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    uid-map=:0:%U:1:
    gid-map=:0:%G:1:
   +
   +[virtiofsd]
   +shared-dir=/nix/store
   +tag=store
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:

Regenerate and start. NixOS boots in about 14 seconds:

.. code-block:: console
   :class: cmd-guest

   [  OK  ] Mounted /sysroot/nix/store.
   ...
   Welcome to NixOS 26.11 (Zokor)!
   ...
   [  OK  ] Started SSH Daemon.
   [  OK  ] Reached target Multi-User System.

   <<< Welcome to NixOS 26.11.20261020.02c087f (x86_64) - ttyS0 >>>

The guest answers the power button now, so stop it the normal way, which
takes 1.8 seconds:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap.service

.. _uki-ssh:

3. SSH
======

QEMU's `user-mode network`_ needs no privileges, and a vsock device
gives the guest a context id, a socket address the host can reach without
any network. Forward a local port to the guest's SSH, and add the device:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    drive=if=pflash,format=raw,snapshot=on,file=%h/bootstrap/uki/ovmf-fd/FV/OVMF_VARS.fd
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

.. _uki-share:

4. A share of your own
======================

A change to the guest is a rebuild and a restart. Sharing a directory of
your own takes both: another ``[virtiofsd]`` section in the file, and a
mount in the guest. Make the directory and put a file in it:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/share
   $ echo "hello from the host" > ~/bootstrap/share/hello.txt

Add the section:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    shared-dir=/nix/store
    tag=store
    sandbox=namespace
    uid-map=:0:%U:1:
    gid-map=:0:%G:1:
   +
   +[virtiofsd]
   +shared-dir=%h/bootstrap/share
   +tag=host
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:

Mount it by its tag:

.. code-block:: diff
   :caption: ~/bootstrap/uki/uki.nix

      fileSystems."/nix/store" = {
        device = "store";
        fsType = "virtiofs";
      };
   +  fileSystems."/mnt/host" = {
   +    device = "host";
   +    fsType = "virtiofs";
   +  };
      systemd.services.systemd-boot-random-seed.enable = false;

Rebuild, regenerate and restart:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/uki
   $ nix build
   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap.service
   $ ssh bootstrap 'cat /mnt/host/hello.txt; touch /mnt/host/from-guest'
   Warning: Permanently added 'vsock/42' (ED25519) to the list of known hosts.
   hello from the host
   $ ls -l ~/bootstrap/share
   -rw-r--r-- 1 user user  0 Oct 23 09:08 from-guest
   -rw-rw-r-- 1 user user 20 Oct 23 09:08 hello.txt

The guest's root wrote ``from-guest``, and on the host the file belongs
to you. :doc:`kernel` boots the same kind of system without firmware and
goes on to SSH poweroff, consoles and systemd settings, which work the
same way here.

.. _systemd: https://systemd.io
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _OVMF: https://github.com/tianocore/tianocore.github.io/wiki/OVMF
.. _unified kernel image: https://uapi-group.org/specifications/specs/unified_kernel_image/
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _virtiofs: https://virtio-fs.gitlab.io
.. _UKI module: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/system/boot/uki.nix
.. _tmpfs: https://docs.kernel.org/filesystems/tmpfs.html
.. _SeaBIOS: https://www.seabios.org/
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _EDK II: https://github.com/tianocore/edk2
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
