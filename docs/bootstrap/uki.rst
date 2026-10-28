.. SPDX-License-Identifier: copyleft-next-0.3.1

==================================
Bootstrap from a UKI in the store
==================================

This guide boots a `NixOS`_ guest in `QEMU`_ under your user's `systemd`_
manager with no disk image and no ``kernel=`` line. The `OVMF`_ firmware
loads a `unified kernel image`_ (UKI), one EFI binary that holds the kernel,
the `initrd`_ and the kernel command line, from a directory the host shares
over `virtiofs`_. The system it boots stays in the host's ``/nix/store``,
shared the same way.

Prepare the host as :doc:`host` shows.

The guest
=========

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

``~/bootstrap/uki/configuration.nix`` sets the guest up as
:ref:`nixos-configuration` describes, with your public key pasted in:

.. code-block:: nix
   :caption: ~/bootstrap/uki/configuration.nix

   { modulesPath, ... }:
   {
     imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

     networking.hostName = "bootstrap";
     services.openssh.enable = true;
     users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ]; # from ~/bootstrap/system/ssh/id_ed25519.pub
     users.users.root.initialHashedPassword = "";

     system.stateVersion = "26.05";
   }

The guest keeps the kernel NixOS picks by default; :ref:`nixos-kernel`
shows how to switch.

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
   $ ls --format=long result/EFI/BOOT
   -r--r--r-- 1 root root 41664512 Jan  1  1970 BOOTX64.EFI

The firmware is a package output named ``fd``, so its link is ``ovmf-fd``.

.. _uki-esp:

1. The firmware and the UKI
===========================

Create ``~/.config/qemu/systemd/bootstrap-uki.qemu``:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-uki.qemu

   [QEMU]
   machine=q35
   nodefaults=
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

``nodefaults=`` leaves out the devices QEMU adds on its own, such as a
VGA card, a CD drive and a network card; the file names every device the
machine has.

:ref:`Regenerate <host-regenerate>` and :ref:`start <host-start>` it; the
journal has the serial console.

The second flash device holds OVMF's variable store, which the firmware
writes; with ``snapshot=on``, QEMU writes to a temporary file instead.

The :key:`[virtiofsd]` section runs a daemon that shares the directory
without root (:ref:`host-virtiofsd`).

With the trailing slash on ``shared-dir=``, the path resolves through
the ``result`` link when the daemon starts, so every start serves the
latest build. Without it, the sandbox cannot bind-mount the link, and
the daemon exits with
``Error entering sandbox: BindMountSharedDir(... "Invalid argument" })``.

OVMF's `EDK II`_ code includes a virtiofs driver, and its boot manager
adds a boot option for a file system that is not on a block device.
That option starts ``BOOTX64.EFI``. The kernel boots, and the initrd
stops where it looks for the store:

.. code-block:: console
   :caption: guest
   :class: cmd-guest

   BdsDxe: loading Boot0002 "UEFI Non-Block Boot Device" from PciRoot(0x0)/Pci(0x1,0x0)
   BdsDxe: starting Boot0002 "UEFI Non-Block Boot Device" from PciRoot(0x0)/Pci(0x1,0x0)
   Booting initrd of NixOS 26.11 (Zokor) (Initrd).
   ...
   [FAILED] Failed to mount /sysroot/nix/store.
   [DEPEND] Dependency failed for Find NixOS closure.
   ...
   You are in emergency mode. After logging in, type "journalctl -xb" to view

Nothing in the guest answers the power button yet, so
:ref:`kill it <host-kill>`.

.. _uki-store:

2. The store
============

Share the host's store under the tag ``store``:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-uki.qemu

    uid-map=:0:%U:1:
    gid-map=:0:%G:1:
   +
   +[virtiofsd]
   +shared-dir=/nix/store
   +tag=store
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:

:ref:`Regenerate <host-regenerate>` and :ref:`start <host-start>`. NixOS
boots in about 14 seconds:

.. code-block:: console
   :class: cmd-guest

   [  OK  ] Mounted /sysroot/nix/store.
   ...
   Welcome to NixOS 26.11 (Zokor)!
   ...
   [  OK  ] Started SSH Daemon.
   [  OK  ] Reached target Multi-User System.

   <<< Welcome to NixOS 26.11.20261020.02c087f (x86_64) - ttyS0 >>>

The guest answers the power button now, so :ref:`stop it <host-stop>` the
normal way, which takes 1.8 seconds:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap-uki.service

.. _uki-ssh:

3. SSH
======

Forward a local port to the guest's SSH over QEMU's `user-mode network`_,
and add the `vsock`_ device with the context id :ref:`host-ssh` names:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-uki.qemu

    drive=if=pflash,format=raw,snapshot=on,file=%h/bootstrap/uki/ovmf-fd/FV/OVMF_VARS.fd
   +netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10046-:22
   +device=virtio-net-pci-non-transitional,netdev=net0,romfile=
   +device=vhost-vsock-pci-non-transitional,guest-cid=46

``romfile=`` with no value leaves out the card's iPXE boot ROM, which
the firmware would otherwise load.

:ref:`Regenerate <host-regenerate>` and :ref:`start <host-start>`.
The machine registers with :cmd:`machined <systemd-machined>`
(:ref:`host-machined`), which records the id:

.. code-block:: console
   :class: cmd-host

   $ machinectl --user list
   MACHINE       CLASS SERVICE     OS VERSION ADDRESSES
   bootstrap-uki vm    qemu-system -  -       -

:ref:`host-ssh` already names the machine by this context id; without that
``Include`` in your ``~/.ssh/config``, the forwarded port takes the key on
the command line:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-uki uname --kernel-release
   Warning: Permanently added 'vsock/46' (ED25519) to the list of known hosts.
   6.18.57
   $ ssh -i ~/bootstrap/system/ssh/id_ed25519 -p 10046 root@127.0.0.1 \
       uname --kernel-release
   6.18.57

.. _uki-share:

4. Your home
============

A change to the guest is a rebuild and a :ref:`restart <host-start>`.
Sharing your home, as :doc:`home` describes, takes both: another
``[virtiofsd]`` section in the file, and a module in the guest. Add the
section:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-uki.qemu

    shared-dir=/nix/store
    tag=store
    sandbox=namespace
    uid-map=:0:%U:1:
    gid-map=:0:%G:1:
   +
   +[virtiofsd]
   +shared-dir=%h
   +tag=home
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:

Save :ref:`home.nix <home-guest>` as ``~/bootstrap/uki/home.nix``, with
your home in place of ``/home/user``, and add it to the guest:

.. code-block:: diff
   :caption: ~/bootstrap/uki/flake.nix

            modules = [
              ./configuration.nix
              ./uki.nix
   +          ./home.nix
            ];

Rebuild, :ref:`regenerate <host-regenerate>` and
:ref:`restart <host-start>`. Root's home is now yours, with the two
systemd directories empty, and a file the guest writes belongs to you on
the host:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/uki
   $ nix build
   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap-uki.service
   $ ssh bootstrap-uki 'pwd; ls --almost-all ~/.config/systemd ~/.local/share/systemd; touch ~/bootstrap/uki/from-guest'
   Warning: Permanently added 'vsock/46' (ED25519) to the list of known hosts.
   /home/user
   /home/user/.config/systemd:

   /home/user/.local/share/systemd:
   $ ls --format=long ~/bootstrap/uki/from-guest
   -rw-r--r-- 1 user user 0 Oct 27 13:47 /home/user/bootstrap/uki/from-guest

.. _uki-fast:

5. A faster boot
================

Each step is timed as :ref:`fast-measure` shows.

The machine as it stands takes 8.7 seconds.

The console
-----------

:ref:`fast-console` explains these settings.

The UKI carries its command line, so the settings go in ``uki.nix``:

.. code-block:: diff
   :caption: ~/bootstrap/uki/uki.nix

    {
   -  boot.kernelParams = [ "console=ttyS0" ];
   +  boot.kernelParams = [
   +    "console=ttyS0"
   +    "systemd.tty.term.console=vt220"
   +    "systemd.tty.rows.console=24"
   +    "systemd.tty.columns.console=80"
   +  ];

Rebuild after each change to ``uki.nix`` or ``configuration.nix``; the
share serves the new UKI at the next start:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/uki
   $ nix build

6.9 seconds.

The store
---------

:ref:`fast-store` explains why the store, and only the store.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-uki.qemu

    [virtiofsd]
    shared-dir=/nix/store
    tag=store
   +cache=always
    sandbox=namespace

5.9 seconds.

The closure
-----------

:ref:`fast-closure` explains these four settings.

.. code-block:: diff
   :caption: ~/bootstrap/uki/configuration.nix

   -{ modulesPath, ... }:
   +{ lib, modulesPath, ... }:
   ...
      services.openssh.enable = true;
   +  services.openssh.hostKeys = [
   +    { type = "ed25519"; path = "/etc/ssh/ssh_host_ed25519_key"; }
   +  ];
   +  security.pam.services.sshd.startSession = lib.mkForce false;
   +  console.enable = false;
   +  systemd.suppressedSystemUnits = [
   +    "systemd-journal-catalog-update.service"
   +    "systemd-update-done.service"
   +  ];

.. code-block:: console
   :class: cmd-host

   $ nix build

5.5 seconds.

More processors
---------------

systemd starts what it can in parallel:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-uki.qemu

   -smp=2
   +smp=4

5.4 seconds.

Quiet
-----

:ref:`fast-quiet` explains what it costs.

.. code-block:: diff
   :caption: ~/bootstrap/uki/uki.nix

        "systemd.tty.columns.console=80"
   +    "quiet"
      ];

.. code-block:: console
   :class: cmd-host

   $ nix build

5.3 seconds, from 8.7. The login is ready before the boot is:
``systemd-analyze critical-chain`` in the guest shows
``multi-user.target`` after ``dhcpcd.service``, which takes 4.2 seconds
and is not on the way to a login.

.. _systemd: https://systemd.io
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _OVMF: https://github.com/tianocore/tianocore.github.io/wiki/OVMF
.. _unified kernel image: https://uapi-group.org/specifications/specs/unified_kernel_image/
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _virtiofs: https://virtio-fs.gitlab.io
.. _UKI module: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/system/boot/uki.nix
.. _tmpfs: https://docs.kernel.org/filesystems/tmpfs.html
.. _EDK II: https://github.com/tianocore/edk2
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
