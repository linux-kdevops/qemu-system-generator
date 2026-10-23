.. SPDX-License-Identifier: copyleft-next-0.3.1

==========================================
Bootstrap: a NixOS guest from a .qemu file
==========================================

This walk-through boots a `NixOS`_ guest under your user's `systemd`_
manager, from one ``.qemu`` file that grows by one feature per step. The
guest's kernel, `initrd`_ and system come from `nixpkgs`_. It first boots
from a disk image, through firmware and a boot loader, the way a
physical machine does. Then QEMU loads the kernel itself, the root file
system becomes a `tmpfs`_, and ``/nix/store`` is the host's own, shared
over `virtiofs`_. No step needs root.

The file format is specified in
:doc:`qemu-systemd.unit(5) <man/qemu-systemd.unit.5>` and the generator in
:doc:`qemu-system-generator(8) <man/qemu-system-generator.8>`. To boot your
own kernel and `QEMU`_ instead, continue with
:doc:`bootstrap-knix` once this page works.

The outputs below are real, trimmed to the lines that matter, with the
home directory shown as ``/home/user``.

What you need
=============

- A Linux host with `KVM`_, where your user can open ``/dev/kvm``.
- systemd with a user manager, which is what
  :cmd:`systemctl` ``--user`` talks to.
- `Nix`_ with flakes enabled.
- A checkout of `this repository`_.

Preparation
===========

Everything lives in one directory:

.. code-block:: console
   :caption: host
   :class: cmd-host

   $ mkdir --parents ~/nixos-bootstrap

A throwaway SSH key
-------------------

The guest accepts this key for root, so keep it for this tutorial only.
Create it with :cmd:`ssh-keygen`:

.. code-block:: console
   :class: cmd-host

   $ ssh-keygen -t ed25519 -N "" -C bootstrap -f ~/nixos-bootstrap/id_ed25519

The guest
---------

The guest is an ordinary NixOS configuration, built two ways: as a disk
image for :ref:`step 2 <bootstrap-disk>` and :ref:`step 3 <bootstrap-uefi>`,
and as a system that boots from the host's store from
:ref:`step 4 <bootstrap-kernel>` on. Create ``~/nixos-bootstrap/vm/flake.nix``:

.. code-block:: nix
   :caption: ~/nixos-bootstrap/vm/flake.nix

   {
     inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

     outputs =
       { self, nixpkgs }:
       let
         system = "x86_64-linux";
       in
       {
         nixosConfigurations.disk = nixpkgs.lib.nixosSystem {
           inherit system;
           modules = [
             ./configuration.nix
             ./disk.nix
           ];
         };
         nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
           inherit system;
           modules = [
             ./configuration.nix
             ./virtiofs.nix
           ];
         };
         packages.${system} = {
           default = self.nixosConfigurations.bootstrap.config.system.build.toplevel;
           disk = self.nixosConfigurations.disk.config.system.build.images.qemu;
           disk-efi = self.nixosConfigurations.disk.config.system.build.images.qemu-efi;
           ovmf = nixpkgs.legacyPackages.${system}.OVMF.fd;
         };
       };
   }

``~/nixos-bootstrap/vm/configuration.nix`` is what both share, with the
public key from ``~/nixos-bootstrap/id_ed25519.pub`` pasted in:

.. code-block:: nix
   :caption: ~/nixos-bootstrap/vm/configuration.nix

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
the initrd. The empty root password is for logging in on the console in
:ref:`step 9 <bootstrap-console>`.

``~/nixos-bootstrap/vm/disk.nix`` names the images and sends the kernel's
messages to the serial port:

.. code-block:: nix
   :caption: ~/nixos-bootstrap/vm/disk.nix

   {
     boot.kernelParams = [ "console=ttyS0" ];
     image.modules.qemu.image.baseName = "bootstrap";
     image.modules.qemu-efi.image.baseName = "bootstrap";
   }

nixpkgs' `image modules`_ add the boot loader and a root file system on
the disk.

``~/nixos-bootstrap/vm/virtiofs.nix`` has neither. QEMU loads the kernel
itself, the root is a tmpfs, and the store arrives over virtiofs under
the tag ``store``, which :ref:`step 5 <bootstrap-store>` sets up:

.. code-block:: nix
   :caption: ~/nixos-bootstrap/vm/virtiofs.nix

   {
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
   }

Build the second one with :cmd:`nix build`:

.. code-block:: console
   :class: cmd-host

   $ cd ~/nixos-bootstrap/vm
   $ nix build

``result`` now holds the system. Its ``kernel`` and ``initrd`` are what QEMU
boots, and its ``init`` is what the kernel runs once the store is mounted:

.. code-block:: console
   :class: cmd-host

   $ ls -l result/kernel result/initrd
   lrwxrwxrwx 1 root root 71 Jan  1  1970 result/initrd -> /nix/store/3rxnmhxn6xfh2hwiakgircj03j4gvd1g-initrd-linux-6.18.57/initrd
   lrwxrwxrwx 1 root root 65 Jan  1  1970 result/kernel -> /nix/store/maq0fxv0r4p9bac4haqy45c4vhlc33ag-linux-6.18.57/bzImage

The generator
-------------

Add it to your Nix profile with :cmd:`nix profile add`, from the checkout
of this repository:

.. code-block:: console
   :class: cmd-host

   $ nix profile add .

The profile puts ``qemu-system-generator`` on ``PATH``. systemd runs user
generators only from root-owned directories
(:cmd:`systemd.generator(7)`), so a user manager never runs this one on its
own. Two user units do it instead. Enable them through the profile's
link, ``~/.nix-profile`` (``~/.local/state/nix/profile`` with
`use-xdg-base-directories`_):

.. code-block:: console
   :class: cmd-host

   $ systemctl --user enable --now \
       ~/.nix-profile/lib/systemd/user/qemu-system-generator.service \
       ~/.nix-profile/lib/systemd/user/qemu-system-generator.path

``systemctl`` links the units by the path it is given, not by the store
path behind it. After ``nix profile upgrade qemu-system-generator``, a
``systemctl --user daemon-reload`` loads the new generation's units.

The service turns every ``.qemu`` file in ``~/.config/qemu/systemd/`` into
units and reloads the manager; it runs at login, and the
:cmd:`systemd.path(5)` unit runs it again whenever that directory changes.
A save takes a moment to land, so every step below regenerates by hand
before starting the machine, which waits until the reload is done:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service

.. _bootstrap-machine:

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

Each line of ``[QEMU]`` is one QEMU option with its dash removed, passed on
as written: ``m=2G`` is ``-m 2G``, and ``nographic=`` with an empty value is
the bare flag ``-nographic``. :cmd:`qemu-system-x86_64(1)` documents them all.
The file is named after the machine, and the unit after the file:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user start qemu-system@bootstrap.service
   $ journalctl --user-unit=qemu-system@bootstrap.service --follow

:cmd:`journalctl` shows the serial console. With nothing to boot, the
firmware tries the disk, the CD and the network, and gives up:

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

``systemctl --user stop`` would press the power button and wait two
minutes for a guest that is not there. Kill it instead; QEMU exits on
``SIGTERM``:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user kill qemu-system@bootstrap.service

.. _bootstrap-disk:

2. A disk
=========

Build the disk image. The first build takes a couple of minutes:

.. code-block:: console
   :class: cmd-host

   $ cd ~/nixos-bootstrap/vm
   $ nix build .#disk --out-link disk
   $ ls disk
   bootstrap.qcow2  nix-support

Give the machine the disk. ``%h`` is your home directory, one of the
`specifiers`_ systemd expands in a unit:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    nographic=
    serial=mon:stdio
   +drive=file=%h/nixos-bootstrap/vm/disk/bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on

The image is in the store, which is read-only. With ``snapshot=on``, QEMU
writes to a temporary file instead, and what the guest wrote is gone
when it stops.

Regenerate and start as in :ref:`step 1 <bootstrap-machine>`. `SeaBIOS`_
finds the disk, `GRUB`_ loads
the kernel and the initrd with the command line from ``disk.nix``, and
NixOS boots in about 18 seconds:

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
takes 1.5 seconds:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap.service

.. _bootstrap-uefi:

3. UEFI
=======

SeaBIOS is QEMU's default firmware. On the PC machines, a firmware in
the first flash device replaces it. Build the `OVMF`_ firmware from
`EDK II`_, and the image with an EFI system partition and `systemd-boot`_:

.. code-block:: console
   :class: cmd-host

   $ nix build .#ovmf --out-link ovmf
   $ nix build .#disk-efi --out-link disk-efi

The firmware is a package output named ``fd``, so its link is ``ovmf-fd``.
The first flash device holds the firmware's code, read-only. The second
holds its variable store, which the firmware writes, so it takes
``snapshot=on`` too:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    serial=mon:stdio
   -drive=file=%h/nixos-bootstrap/vm/disk/bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on
   +drive=if=pflash,format=raw,readonly=on,file=%h/nixos-bootstrap/vm/ovmf-fd/FV/OVMF_CODE.fd
   +drive=if=pflash,format=raw,snapshot=on,file=%h/nixos-bootstrap/vm/ovmf-fd/FV/OVMF_VARS.fd
   +drive=file=%h/nixos-bootstrap/vm/disk-efi/bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on

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

Stop it as in :ref:`step 2 <bootstrap-disk>`.

.. _bootstrap-kernel:

4. A kernel
===========

Every change to the guest above means a new image with the whole system
in it. QEMU can load a kernel and an initrd itself, without firmware
looking for a boot loader, and the system can stay in the host's store.
Drop the disk and point QEMU at the closure's kernel and initrd:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    serial=mon:stdio
   -drive=if=pflash,format=raw,readonly=on,file=%h/nixos-bootstrap/vm/ovmf-fd/FV/OVMF_CODE.fd
   -drive=if=pflash,format=raw,snapshot=on,file=%h/nixos-bootstrap/vm/ovmf-fd/FV/OVMF_VARS.fd
   -drive=file=%h/nixos-bootstrap/vm/disk-efi/bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on
   +kernel=%h/nixos-bootstrap/vm/result/kernel
   +initrd=%h/nixos-bootstrap/vm/result/initrd
   +append=console=ttyS0

No boot loader reads the closure's command line, so ``append=`` is all
the kernel gets. Regenerate and start. Linux boots, and the initrd stops
where it looks for the store:

.. code-block:: console
   :class: cmd-guest

   [    0.000000] Linux version 6.18.57 (nixbld@localhost) (gcc (GCC) 16.2.0, GNU ld (GNU Binutils) 2.46) #1-NixOS SMP PREEMPT_DYNAMIC @1792058300
   [    0.000000] Command line: console=ttyS0
   ...
   [    3.121002] virtio-fs: tag <store> not found
   [FAILED] Failed to mount /sysroot/nix/store.
   [DEPEND] Dependency failed for Initrd File Systems.
   [DEPEND] Dependency failed for Find NixOS closure.
   ...
   You are in emergency mode. After logging in, type "journalctl -xb" to view

Kill it as in :ref:`step 1 <bootstrap-machine>`.

.. _bootstrap-store:

5. The store
============

Two things are missing: the store under the tag ``store``, and ``init=``,
which tells the initrd which system in the store to boot:

.. code-block:: console
   :class: cmd-host

   $ readlink ~/nixos-bootstrap/vm/result
   /nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f

Add both, with your own path in ``init=``:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    kernel=%h/nixos-bootstrap/vm/result/kernel
    initrd=%h/nixos-bootstrap/vm/result/initrd
   -append=console=ttyS0
   +append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
   +
   +[virtiofsd]
   +shared-dir=/nix/store
   +tag=store
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:

Each ``[virtiofsd]`` section runs one `virtiofsd`_ daemon for one shared
directory. Its keys are the long options of ``virtiofsd --help``
without the dashes. The generator runs the daemon from its own socket
unit and adds the QEMU lines that connect to it. ``sandbox=``,
``uid-map=`` and ``gid-map=`` let it run without root: ``%U`` and ``%G``
are your user and group ids, which the guest's root maps to.

Regenerate and start. NixOS boots in about 14 seconds, faster than
from the disk:

.. code-block:: console
   :class: cmd-guest

   [  OK  ] Mounted /sysroot/nix/store.
   ...
   [  OK  ] Started SSH Daemon.
   [  OK  ] Reached target Multi-User System.

   <<< Welcome to NixOS 26.11.20261020.02c087f (x86_64) - ttyS0 >>>

The daemon came up with the machine:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user list-units 'virtiofsd@bootstrap-*'
     UNIT                              LOAD   ACTIVE SUB     DESCRIPTION
     virtiofsd@bootstrap-store.service loaded active running virtiofsd for bootstrap-store
     virtiofsd@bootstrap-store.socket  loaded active running virtiofsd socket for bootstrap-store

Stop it the normal way:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap.service

.. code-block:: console
   :class: cmd-guest

   [  OK  ] Reached target System Power Off.
   [   15.420532] reboot: Power down
   Stopped qemu-system@bootstrap.service - QEMU System Virtual Machine bootstrap.

That took 1.7 seconds. The daemon stops with the machine, and its
socket stays listening for the next start.

.. _bootstrap-network:

6. A network
============

QEMU's `user-mode network`_ needs no privileges. Forward a local port to
the guest's SSH:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
   +nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22

Regenerate, restart with ``systemctl --user restart``, and log in with
:cmd:`ssh`:

.. code-block:: console
   :class: cmd-host

   $ ssh -i ~/nixos-bootstrap/id_ed25519 -p 10022 root@127.0.0.1 uname -r
   6.18.57

.. _bootstrap-vsock:

7. vsock
========

A `vsock`_ device gives the guest a context id, a socket address the host
can reach without any network:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
   +device=vhost-vsock-pci-non-transitional,guest-cid=42

The generated unit registers each machine with your user's
:cmd:`systemd-machined.service(8)`. With a vsock device it records the id,
and :cmd:`machinectl` lists the machine:

.. code-block:: console
   :class: cmd-host

   $ machinectl --user list
   MACHINE   CLASS SERVICE     OS VERSION ADDRESSES
   bootstrap vm    qemu-system -  -       -

NixOS listens for SSH on vsock by itself, through
:cmd:`systemd-ssh-generator(8)`, and :cmd:`systemd-ssh-proxy(1)` lets ``ssh``
on the host reach a registered machine by name:

.. code-block:: console
   :class: cmd-host

   $ ssh -i ~/nixos-bootstrap/id_ed25519 root@machine/bootstrap systemctl is-system-running
   running

``machine/`` needs the ``ssh`` configuration that systemd ships as
``20-systemd-ssh-proxy.conf``, which most distributions link into
``/etc/ssh/ssh_config.d/``. Without it,
:ref:`step 6 <bootstrap-network>`'s forwarded port still works.

.. _bootstrap-poweroff:

8. Poweroff over SSH
====================

``[VM]`` holds what is not a QEMU option. With ``SshPrivateKey=``, stopping
the machine first runs ``systemctl poweroff`` in the guest over vsock, and
presses the power button only if that fails:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

   +[VM]
   +SshPrivateKey=%h/nixos-bootstrap/id_ed25519
   +
    [QEMU]
    machine=q35

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap.service
   ...
   Connection to vsock/42 closed by remote host.
   ...
   [  OK  ] Reached target System Power Off.
   [   16.084116] reboot: Power down

.. _bootstrap-console:

9. A console
============

The serial console goes to the journal, which you read but cannot type
into. Give the guest a second console, a virtio console on a socket in
the machine's runtime directory. ``%t`` is that directory's root and ``%i``
the machine name:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

   -append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
   +append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
    nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
    device=vhost-vsock-pci-non-transitional,guest-cid=42
   +device=virtio-serial-pci-non-transitional
   +chardev=socket,id=console0,path=%t/qemu-system/%i/console.sock,server=on,wait=off
   +device=virtconsole,chardev=console0

Restart, then attach with :cmd:`socat`. Press Enter for the prompt, log in
as root with no password, and leave with Ctrl-O:

.. code-block:: console
   :class: cmd-host

   $ socat -,rawer,escape=0x0f UNIX-CONNECT:$XDG_RUNTIME_DIR/qemu-system/bootstrap/console.sock

   bootstrap login: root
   [root@bootstrap:~]# uname -r
   6.18.57

.. _bootstrap-drop-in:

10. The closure in a drop-in
============================

``kernel=`` and ``initrd=`` follow the ``result`` link, but ``init=`` names one
build, and every ``nix build`` makes a new one. Move ``append=`` to a
drop-in: the files in ``bootstrap.qemu.d/`` are appended to
``bootstrap.qemu``, so a command can rewrite that one line without
touching the file you edit by hand.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    initrd=%h/nixos-bootstrap/vm/result/initrd
   -append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
    nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22

.. code-block:: shell
   :class: cmd-host

   mkdir --parents ~/.config/qemu/systemd/bootstrap.qemu.d
   cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 console=hvc0 init=$(readlink ~/nixos-bootstrap/vm/result)/init
   EOF

:cmd:`readlink` fills in the path. To see what the generator makes of the
two files without writing anything:

.. code-block:: console
   :class: cmd-host

   $ qemu-system-generator --user --dryrun
   ---qemu-system@bootstrap.service---
   ...
       -append "console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init" \
   ...
   ---virtiofsd@bootstrap-store.socket---
   ...
   ---virtiofsd@bootstrap-store.service---
   ...

The path unit does not watch the drop-in directory, so regenerate by
hand after rewriting it.

.. _bootstrap-settings:

11. systemd settings
====================

``[Unit]``, ``[Service]`` and ``[Install]`` are copied into the generated
service, so the machine takes a description, `resource limits`_ and an
install target the way a hand-written unit does:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    gid-map=:0:%G:1:
   +
   +[Unit]
   +Description=NixOS bootstrap guest
   +
   +[Service]
   +MemoryMax=3G
   +CPUWeight=50
   +
   +[Install]
   +WantedBy=machines.target

.. code-block:: console
   :class: cmd-host

   $ systemctl --user status qemu-system@bootstrap.service
   ● qemu-system@bootstrap.service - NixOS bootstrap guest
        Loaded: loaded (/home/user/.config/qemu/systemd/bootstrap.qemu; enabled-runtime; preset: enabled)
        Active: active (running) since Thu 2026-10-22 16:46:55 CEST; 13s ago
        Memory: 340.4M (max: 3G, available: 2.6G, peak: 341.5M)

With ``WantedBy=machines.target``, ``systemctl --user start
machines.target`` starts the machine too. Every generated machine is
also part of :cmd:`machines.target`, so ``systemctl --user stop
machines.target`` stops all of them, not only this one.

.. _bootstrap-share:

12. A share of your own
=======================

Sharing a directory of your own takes another ``[virtiofsd]`` section.
Make the directory and put a file in it:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/nixos-bootstrap/share
   $ echo "hello from the host" > ~/nixos-bootstrap/share/hello.txt

Then add the section:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    gid-map=:0:%G:1:

   +[virtiofsd]
   +shared-dir=%h/nixos-bootstrap/share
   +tag=host
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:
   +
    [Unit]

The guest mounts it by tag, so ``virtiofs.nix`` changes too:

.. code-block:: diff
   :caption: ~/nixos-bootstrap/vm/virtiofs.nix

      fileSystems."/nix/store" = {
        device = "store";
        fsType = "virtiofs";
      };
   +  fileSystems."/mnt/host" = {
   +    device = "host";
   +    fsType = "virtiofs";
   +  };

Rebuild, rewrite the drop-in as in :ref:`step 10 <bootstrap-drop-in>`,
regenerate and restart:

.. code-block:: console
   :class: cmd-host

   $ ssh -i ~/nixos-bootstrap/id_ed25519 root@machine/bootstrap 'cat /mnt/host/hello.txt; touch /mnt/host/from-guest'
   hello from the host
   $ ls -l ~/nixos-bootstrap/share
   -rw-r--r-- 1 user user  0 Oct 22 16:47 from-guest
   -rw-rw-r-- 1 user user 20 Oct 22 16:47 hello.txt

The guest's root wrote ``from-guest``, and on the host the file belongs
to you.

Notes
=====

- ``journalctl --user-unit=`` finds a user service's messages;
  ``journalctl --user --unit=`` may not, depending on the journal's
  configuration.
- A machine stopped in the first seconds of its boot can neither take
  the SSH poweroff nor answer the power button yet. The stop then waits
  for ``TimeoutStopSec=``, two minutes, before killing it;
  ``systemctl --user kill`` is quicker.
- In a ``[virtiofsd]`` section, ``%i`` is ``name-tag``, the daemon's
  instance, not the machine name.
- A ``qemu-system@.service`` template of your own in
  ``~/.config/systemd/user/`` serves any machine the generator has not
  written, and drop-ins under ``qemu-system@name.service.d/`` apply to
  generated units too.

.. _this repository: https://github.com/linux-kdevops/qemu-system-generator
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _systemd: https://systemd.io
.. _nixpkgs: https://github.com/NixOS/nixpkgs
.. _virtiofs: https://virtio-fs.gitlab.io
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _KVM: https://docs.kernel.org/virt/kvm/index.html
.. _Nix: https://nix.dev/manual/nix/latest/
.. _image modules: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/image/images.nix
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _GRUB: https://www.gnu.org/software/grub/manual/grub/grub.html
.. _OVMF: https://github.com/tianocore/tianocore.github.io/wiki/OVMF
.. _EDK II: https://github.com/tianocore/edk2
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _tmpfs: https://docs.kernel.org/filesystems/tmpfs.html
.. _SeaBIOS: https://www.seabios.org/
.. _systemd-boot: https://www.freedesktop.org/software/systemd/man/latest/systemd-boot.html
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _use-xdg-base-directories: https://nix.dev/manual/nix/latest/command-ref/conf-file.html#conf-use-xdg-base-directories
.. _resource limits: https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html
