.. SPDX-License-Identifier: copyleft-next-0.3.1

=====================================
Bootstrap from a kernel and an initrd
=====================================

This guide boots a `NixOS`_ guest under your user's `systemd`_ manager
with no firmware and no disk. `QEMU`_ loads the guest's kernel and `initrd`_
itself, the root file system is a `tmpfs`_, and ``/nix/store`` is the
host's own, shared over `virtiofs`_. Its :key:`.qemu` file grows by one
feature per step, and no step needs root.

Prepare the host as :doc:`host` shows, once for every guide.

The guest
---------

The guest is an ordinary NixOS configuration. Create
``~/bootstrap/direct/flake.nix``:

.. code-block:: nix
   :caption: ~/bootstrap/direct/flake.nix

   {
     inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

     outputs =
       { self, nixpkgs }:
       {
         nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
           system = "x86_64-linux";
           modules = [
             ./configuration.nix
             ./virtiofs.nix
           ];
         };
         packages.x86_64-linux.default = self.nixosConfigurations.bootstrap.config.system.build.toplevel;
       };
   }

``~/bootstrap/direct/configuration.nix`` sets the guest up as
:ref:`nixos-configuration` describes, with your public key pasted in:

.. code-block:: nix
   :caption: ~/bootstrap/direct/configuration.nix

   { modulesPath, ... }:
   {
     imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

     networking.hostName = "bootstrap";
     services.openssh.enable = true;
     users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
     users.users.root.initialHashedPassword = "";

     system.stateVersion = "26.05";
   }

The guest keeps the kernel NixOS picks by default; :ref:`nixos-kernel`
shows how to switch.

``~/bootstrap/direct/virtiofs.nix`` has no boot loader and no disk.
The root is a tmpfs, and the store arrives over virtiofs under the tag
``store``, which :ref:`step 2 <direct-store>` sets up:

.. code-block:: nix
   :caption: ~/bootstrap/direct/virtiofs.nix

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

Build it with ``nix build``:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/direct
   $ nix build

``result`` now holds the system. Its ``kernel`` and ``initrd`` are what QEMU
boots, and its ``init`` is what the kernel runs once the store is mounted:

.. code-block:: console
   :class: cmd-host

   $ ls --format=long result/kernel result/initrd
   lrwxrwxrwx 1 root root 71 Jan  1  1970 result/initrd -> /nix/store/3rxnmhxn6xfh2hwiakgircj03j4gvd1g-initrd-linux-6.18.57/initrd
   lrwxrwxrwx 1 root root 65 Jan  1  1970 result/kernel -> /nix/store/maq0fxv0r4p9bac4haqy45c4vhlc33ag-linux-6.18.57/bzImage

.. _direct-kernel:

1. A kernel
===========

Create ``~/.config/qemu/systemd/bootstrap.qemu``, pointing QEMU at the
closure's kernel and initrd. ``%h`` is your home directory, one of the
`specifiers`_ systemd expands in a unit:

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
   kernel=%h/bootstrap/direct/result/kernel
   initrd=%h/bootstrap/direct/result/initrd
   append=console=ttyS0

Regenerate and start it as :ref:`host-start` shows; the journal has the
serial console.

No boot loader reads the closure's command line, so ``append=`` is all
the kernel gets. `Linux`_ boots, and the initrd stops where it looks for the
store:

.. code-block:: console
   :caption: guest
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

Kill it, as :ref:`a guest that cannot answer the power button <host-kill>`
needs.

.. _direct-store:

2. The store
============

Two things are missing: the store under the tag ``store``, and ``init=``,
which tells the initrd which system in the store to boot:

.. code-block:: console
   :class: cmd-host

   $ readlink ~/bootstrap/direct/result
   /nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f

Add both, with your own path in ``init=``:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    kernel=%h/bootstrap/direct/result/kernel
    initrd=%h/bootstrap/direct/result/initrd
   -append=console=ttyS0
   +append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
   +
   +[virtiofsd]
   +shared-dir=/nix/store
   +tag=store
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:

The :key:`[virtiofsd]` section runs a daemon that shares the store
without root (:ref:`host-virtiofsd`).

Regenerate and start. NixOS boots in about 14 seconds:

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

The guest answers the power button now, so stop it the normal way:

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

.. _direct-network:

3. A network
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

   $ ssh -i ~/bootstrap/system/ssh/id_ed25519 -p 10022 root@127.0.0.1 \
       uname --kernel-release
   6.18.57

.. _direct-vsock:

4. vsock
========

A vsock device gives the guest a context id, a socket address the host
can reach without any network:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
   +device=vhost-vsock-pci-non-transitional,guest-cid=42

The machine registers with machined (:ref:`host-machined`), which
records the id:

.. code-block:: console
   :class: cmd-host

   $ machinectl --user list
   MACHINE   CLASS SERVICE     OS VERSION ADDRESSES
   bootstrap vm    qemu-system -  -       -

NixOS listens for SSH on vsock by itself, and :ref:`host-ssh` already
names the machine by this context id:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap systemctl is-system-running
   Warning: Permanently added 'vsock/42' (ED25519) to the list of known hosts.
   running

.. _direct-poweroff:

5. Poweroff over SSH
====================

With :key:`SshPrivateKey=` in :key:`[VM]`, the stop powers the guest off
over SSH first (:ref:`SSH poweroff <host-poweroff>`):

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

   +[VM]
   +SshPrivateKey=%h/bootstrap/system/ssh/id_ed25519
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

.. _direct-console:

6. A console
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
   [root@bootstrap:~]# uname --kernel-release
   6.18.57

.. _direct-drop-in:

7. The closure in a drop-in
===========================

``kernel=`` and ``initrd=`` follow the ``result`` link, but ``init=`` names one
build, and every ``nix build`` makes a new one. Move ``append=`` to a
drop-in: the files in ``bootstrap.qemu.d/`` are appended to
``bootstrap.qemu``, so a command can rewrite that one line without
touching the file you edit by hand.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    initrd=%h/bootstrap/direct/result/initrd
   -append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
    nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22

.. code-block:: shell
   :class: cmd-host

   mkdir --parents ~/.config/qemu/systemd/bootstrap.qemu.d
   cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 console=hvc0 init=$(readlink ~/bootstrap/direct/result)/init
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

.. _direct-settings:

8. systemd settings
===================

:cmd:`[Unit] <systemd.unit(5)>`, :cmd:`[Service] <systemd.service(5)>` and
``[Install]`` are copied into the generated service, so
the machine takes a description, `resource limits`_ and an install target
the way a hand-written unit does:

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

With :cmd:`WantedBy=machines.target <WantedBy=>`, ``systemctl --user start
machines.target`` starts the machine too. Every generated machine is
also part of :cmd:`machines.target`, so ``systemctl --user stop
machines.target`` stops all of them, not only this one.

.. _direct-share:

9. A share of your own
======================

Sharing a directory of your own takes another ``[virtiofsd]`` section.
Make the directory and put a file in it:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/system/share
   $ echo "hello from the host" > ~/bootstrap/system/share/hello.txt

Then add the section:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    gid-map=:0:%G:1:

   +[virtiofsd]
   +shared-dir=%h/bootstrap/system/share
   +tag=host
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:
   +
    [Unit]

The guest mounts it by tag, so ``virtiofs.nix`` changes too:

.. code-block:: diff
   :caption: ~/bootstrap/direct/virtiofs.nix

      fileSystems."/nix/store" = {
        device = "store";
        fsType = "virtiofs";
      };
   +  fileSystems."/mnt/host" = {
   +    device = "host";
   +    fsType = "virtiofs";
   +  };

Rebuild, rewrite the drop-in as in :ref:`step 7 <direct-drop-in>`,
regenerate and restart:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap 'cat /mnt/host/hello.txt; touch /mnt/host/from-guest'
   Warning: Permanently added 'vsock/42' (ED25519) to the list of known hosts.
   hello from the host
   $ ls --format=long ~/bootstrap/system/share
   -rw-r--r-- 1 user user  0 Oct 22 16:47 from-guest
   -rw-rw-r-- 1 user user 20 Oct 22 16:47 hello.txt

The guest's root wrote ``from-guest``, and on the host the file belongs
to you.

.. _direct-microvm:

10. A microvm
=============

QEMU's `microvm`_ machine type has no PCI bus and no firmware to run: its
devices sit on virtio-mmio, and the kernel reaches its ``/init`` sooner.
It needs the newest kernel here. On 6.18.57, three or more virtio-mmio
devices stop the boot at ``ACPI: Core revision``, and this machine has
five. Switch the guest to ``pkgs.linuxPackages_latest``, as
:ref:`nixos-kernel` shows:

.. code-block:: diff
   :caption: ~/bootstrap/direct/configuration.nix

   -{ modulesPath, ... }:
   +{ modulesPath, pkgs, ... }:
    {
      imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];
   +  boot.kernelPackages = pkgs.linuxPackages_latest;

And rebuild:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/direct
   $ nix build
   $ readlink result/kernel
   /nix/store/sby4x25d1wl7nn915c62jps8nr65sncj-linux-7.3/bzImage

Then change the machine. On microvm the virtio devices take their
``-device`` names instead of the ``-pci-non-transitional`` ones. The
network card becomes a ``netdev=`` and a ``device=`` that names it:
``nic=`` would still open the forwarded port, but microvm never creates
the card it asks for, since only the PC machines' PCI setup does
(``pci_init_nic_devices()`` in ``hw/i386/pc.c``). The journal shows
QEMU's warning, ``requested NIC (anonymous, model virtio-net-device) was
not created (not supported by this machine?)``. The ``global=`` line has
QEMU's virtio-mmio transport offer virtio 1.0, as a non-transitional PCI
device does; QEMU defaults to the legacy transport (``force-legacy`` in
``hw/virtio/virtio-mmio.c``). Under KVM, microvm leaves the RTC out unless
``rtc=on`` asks for it (``hw/i386/microvm.c``). Without it the kernel's
RTC probe fails, and ``/init`` started at 1.66 seconds instead of 0.49
here:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

   -machine=q35
   +machine=microvm,rtc=on
    accel=kvm
    cpu=host
    m=2G
    smp=2
    nographic=
    serial=mon:stdio
    kernel=%h/bootstrap/direct/result/kernel
    initrd=%h/bootstrap/direct/result/initrd
   -nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
   -device=vhost-vsock-pci-non-transitional,guest-cid=42
   -device=virtio-serial-pci-non-transitional
   +global=virtio-mmio.force-legacy=false
   +netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10022-:22
   +device=virtio-net-device,netdev=net0
   +device=vhost-vsock-device,guest-cid=42
   +device=virtio-serial-device

Keep `ACPI`_, microvm's default. With ``acpi=off`` the guest cannot power
off, and every stop waits out :cmd:`TimeoutStopSec=`. The generator
follows the machine type for the virtiofs devices it adds:

.. code-block:: console
   :class: cmd-host

   $ qemu-system-generator --user --dryrun | grep vhost-user-fs
       -device vhost-user-fs-device,queue-size=1024,chardev=virtiofs_store,tag=store \
       -device vhost-user-fs-device,queue-size=1024,chardev=virtiofs_host,tag=host \

On microvm this guest's ``systemd-ssh-generator`` opens no vsock
listener by itself, so ask for one on the kernel command line with
``systemd.ssh_listen=vsock::22``. Rewrite the drop-in with the new
closure and that option:

.. code-block:: shell
   :class: cmd-host

   cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 console=hvc0 systemd.ssh_listen=vsock::22 init=$(readlink ~/bootstrap/direct/result)/init
   EOF

Regenerate, restart and log in:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap.service
   $ ssh bootstrap uname --kernel-release
   Warning: Permanently added 'vsock/42' (ED25519) to the list of known hosts.
   7.3.0

On this guest the kernel reached ``/init`` at 0.49 seconds against 0.75
on q35 with the same kernel, but the initrd mounted the store later, at
1.85 seconds against 1.14, and ``ssh`` answered after 8.9 to 9.1 seconds
against 8.2 to 8.3. What microvm saves in the kernel, NixOS's userspace
outweighs, so the other guides keep q35.

.. _direct-peer:

11. A second machine on one network
===================================

A second machine, ``peer``, joins ``bootstrap`` on a multicast segment
and boots the same closure, which holds both machines' addresses
(:doc:`network`):

.. code-block:: diff
   :caption: ~/bootstrap/direct/configuration.nix

      networking.hostName = "bootstrap";
   +  networking.useNetworkd = true;
   +  systemd.network.networks = {
   +    "10-lan-bootstrap" = {
   +      matchConfig.MACAddress = "52:54:00:00:00:42";
   +      address = [ "10.0.100.42/24" ];
   +    };
   +    "10-lan-peer" = {
   +      matchConfig.MACAddress = "52:54:00:00:00:46";
   +      address = [ "10.0.100.46/24" ];
   +    };
   +  };
      services.openssh.enable = true;

With ``networking.useNetworkd``, NixOS asks for the user-mode card's
address over DHCP through systemd-networkd too, from its
``99-ethernet-default-dhcp`` network, which the two ``10-`` networks
come before. Rebuild, and rewrite the drop-in as in
:ref:`step 10 <direct-microvm>`:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/direct
   $ nix build

.. code-block:: shell
   :class: cmd-host

   cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 console=hvc0 systemd.ssh_listen=vsock::22 init=$(readlink ~/bootstrap/direct/result)/init
   EOF

Give ``bootstrap`` its card:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10022-:22
    device=virtio-net-device,netdev=net0
   +netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
   +device=virtio-net-device,netdev=lan,mac=52:54:00:00:00:42
    device=vhost-vsock-device,guest-cid=42

``peer`` needs the store and the share, which the closure mounts, a
context id of its own and no forwarded port, since ``bootstrap`` holds
10022:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/peer.qemu

   [VM]
   SshPrivateKey=%h/bootstrap/system/ssh/id_ed25519

   [QEMU]
   machine=microvm,rtc=on
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio
   kernel=%h/bootstrap/direct/result/kernel
   initrd=%h/bootstrap/direct/result/initrd
   global=virtio-mmio.force-legacy=false
   netdev=user,id=net0
   device=virtio-net-device,netdev=net0
   netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
   device=virtio-net-device,netdev=lan,mac=52:54:00:00:00:46
   device=vhost-vsock-device,guest-cid=46

   [virtiofsd]
   shared-dir=/nix/store
   tag=store
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h/bootstrap/system/share
   tag=host
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

Its drop-in names the same closure, and gives the machine its own
hostname:

.. code-block:: shell
   :class: cmd-host

   mkdir --parents ~/.config/qemu/systemd/peer.qemu.d
   cat > ~/.config/qemu/systemd/peer.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 systemd.hostname=peer systemd.ssh_listen=vsock::22 init=$(readlink ~/bootstrap/direct/result)/init
   EOF

Regenerate, restart ``bootstrap``, start ``peer``, and each reaches the
other:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap.service
   $ systemctl --user start qemu-system@peer.service
   $ ssh peer 'hostname; ping -c 1 10.0.100.42'
   Warning: Permanently added 'vsock/46' (ED25519) to the list of known hosts.
   peer
   PING 10.0.100.42 (10.0.100.42) 56(84) bytes of data.
   64 bytes from 10.0.100.42: icmp_seq=1 ttl=64 time=0.501 ms
   ...
   $ ssh bootstrap ping -c 1 10.0.100.46
   PING 10.0.100.46 (10.0.100.46) 56(84) bytes of data.
   64 bytes from 10.0.100.46: icmp_seq=1 ttl=64 time=0.276 ms
   ...

.. _direct-fast:

12. A faster boot
=================

Each step is timed as :ref:`fast-measure` shows.

The machine as it stands takes 8.9 to 9.4 seconds.

The console
-----------

:ref:`fast-console` explains these settings.

Add them to the drop-in:

.. code-block:: shell
   :class: cmd-host

   cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 console=hvc0 systemd.ssh_listen=vsock::22 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 init=$(readlink ~/bootstrap/direct/result)/init
   EOF

Regenerate and restart: 7.3 seconds, and the initrd's share falls from
3.1 to 2.1.

The store
---------

:ref:`fast-store` explains why the store, and only the store.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    [virtiofsd]
    shared-dir=/nix/store
    tag=store
   +cache=always
    sandbox=namespace

6.1 seconds.

The closure
-----------

:ref:`fast-closure` explains these four settings.

.. code-block:: diff
   :caption: ~/bootstrap/direct/configuration.nix

   -{ modulesPath, pkgs, ... }:
   +{ lib, modulesPath, pkgs, ... }:
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

Rebuild, and rewrite the drop-in with the new closure:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/direct
   $ nix build

.. code-block:: shell
   :class: cmd-host

   cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 console=hvc0 systemd.ssh_listen=vsock::22 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 init=$(readlink ~/bootstrap/direct/result)/init
   EOF

5.1 seconds. ``peer`` keeps the closure its own drop-in names until you
rewrite that one too.

More processors
---------------

systemd starts what it can in parallel:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

   -smp=2
   +smp=4

5.0 seconds.

Quiet
-----

:ref:`fast-quiet` explains what it costs.

.. code-block:: shell
   :class: cmd-host

   cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 console=hvc0 systemd.ssh_listen=vsock::22 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 quiet init=$(readlink ~/bootstrap/direct/result)/init
   EOF

4.7 seconds, half the time this section started from.

The initrd still waits about 0.8 seconds on systemd's mount rate limit,
which this guest cannot lift safely (:ref:`fast-mounts`).

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
- A :key:`qemu-system@.service` template of your own in
  ``~/.config/systemd/user/`` serves any machine the generator has not
  written, and drop-ins under ``qemu-system@name.service.d/`` apply to
  generated units too.

.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _systemd: https://systemd.io
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _tmpfs: https://docs.kernel.org/filesystems/tmpfs.html
.. _virtiofs: https://virtio-fs.gitlab.io
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _microvm: https://www.qemu.org/docs/master/system/i386/microvm.html
.. _ACPI: https://uefi.org/specifications
.. _Linux: https://www.kernel.org
.. _resource limits: https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html
