.. SPDX-License-Identifier: copyleft-next-0.3.1

=====================================
Bootstrap from a kernel and an initrd
=====================================

This guide boots a `NixOS`_ guest under your user's `systemd`_ manager
with no firmware and no disk. `QEMU`_ loads the guest's kernel and `initrd`_
itself, the root file system is a `tmpfs`_, and ``/nix/store`` is the
host's own, shared over `virtiofs`_.

Prepare the host as :doc:`host` shows.

The guest
=========

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
     users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ]; # from ~/bootstrap/system/ssh/id_ed25519.pub
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

Build it with :cmd:`nix build`:

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

Create ``~/.config/qemu/systemd/bootstrap-direct.qemu``, pointing QEMU at
the closure's kernel and initrd; ``%h`` is your home directory
(`specifiers`_):

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

   [QEMU]
   machine=q35
   nodefaults=
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio
   kernel=%h/bootstrap/direct/result/kernel
   initrd=%h/bootstrap/direct/result/initrd
   append=console=ttyS0

``nodefaults=`` leaves out the devices QEMU adds on its own, such as a
VGA card, a CD drive and a network card; the file names every device the
machine has.

:ref:`Regenerate <host-regenerate>` and :ref:`start <host-start>` it; the
journal has the serial console.

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

Nothing in the guest answers the power button yet, so
:ref:`kill it <host-kill>`.

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
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

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

:ref:`Regenerate <host-regenerate>` and :ref:`start <host-start>`. NixOS
boots in about 14 seconds:

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

   $ systemctl --user list-units 'virtiofsd@bootstrap-direct-*'
     UNIT                                     LOAD   ACTIVE SUB     DESCRIPTION
     virtiofsd@bootstrap-direct-store.service loaded active running virtiofsd Service for bootstrap-direct-store
     virtiofsd@bootstrap-direct-store.socket  loaded active running virtiofsd Socket for bootstrap-direct-store

The guest answers the power button now, so :ref:`stop it <host-stop>` the
normal way:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap-direct.service

.. code-block:: console
   :class: cmd-guest

   [  OK  ] Reached target System Power Off.
   [   15.420532] reboot: Power down
   Stopped qemu-system@bootstrap-direct.service - QEMU System Virtual Machine bootstrap-direct.

That took 1.7 seconds. The daemon stops with the machine, and its
socket stays listening for the next start.

.. _direct-network:

3. A network
============

Forward a local port to the guest's SSH over QEMU's `user-mode network`_:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

    append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
   +netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10047-:22
   +device=virtio-net-pci-non-transitional,netdev=net0,romfile=

``romfile=`` with no value leaves out the card's iPXE boot ROM, which
the firmware would otherwise load.

:ref:`Regenerate <host-regenerate>`, :ref:`start <host-start>`, and log
in with :cmd:`ssh`:

.. code-block:: console
   :class: cmd-host

   $ ssh -i ~/bootstrap/system/ssh/id_ed25519 -p 10047 root@127.0.0.1 \
       uname --kernel-release
   6.18.57

.. _direct-vsock:

4. vsock
========

A `vsock`_ device gives the guest a context id:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

    netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10047-:22
    device=virtio-net-pci-non-transitional,netdev=net0,romfile=
   +device=vhost-vsock-pci-non-transitional,guest-cid=47

The machine registers with :cmd:`machined <systemd-machined>`
(:ref:`host-machined`), which records the id:

.. code-block:: console
   :class: cmd-host

   $ machinectl --user list
   MACHINE          CLASS SERVICE     OS VERSION ADDRESSES
   bootstrap-direct vm    qemu-system -  -       -

NixOS listens for SSH on vsock by itself, and :ref:`host-ssh` already
names the machine by this context id:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-direct systemctl is-system-running
   Warning: Permanently added 'vsock/47' (ED25519) to the list of known hosts.
   running

.. _direct-poweroff:

5. Poweroff over SSH
====================

With :key:`SSHPrivateKey=` in :key:`[VM]`, the stop powers the guest off
over SSH first (:ref:`SSH poweroff <host-poweroff>`):

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

   +[VM]
   +SSHPrivateKey=%h/bootstrap/system/ssh/id_ed25519
   +
    [QEMU]
    machine=q35
    nodefaults=

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap-direct.service
   ...
   Connection to vsock/47 closed by remote host.
   ...
   [  OK  ] Reached target System Power Off.
   [   16.084116] reboot: Power down

.. _direct-console:

6. A console
============

The serial console goes to the journal, which you read but cannot type
into. Give the guest a second console, a virtio console on a socket in
the machine's runtime directory. ``%i`` is the machine name:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

   -append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
   +append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
    netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10047-:22
    device=virtio-net-pci-non-transitional,netdev=net0,romfile=
    device=vhost-vsock-pci-non-transitional,guest-cid=47
   +device=virtio-serial-pci-non-transitional
   +chardev=socket,id=console0,path=%t/qemu-system/%i/console.sock,server=on,wait=off
   +device=virtconsole,chardev=console0

:ref:`Regenerate <host-regenerate>`, :ref:`start <host-start>`, and
attach with :cmd:`socat`:

.. code-block:: console
   :class: cmd-host

   $ socat -,rawer,escape=0x0f UNIX-CONNECT:$XDG_RUNTIME_DIR/qemu-system/bootstrap-direct/console.sock

Press :kbd:`Enter` for the prompt and log in as root with no password:

.. code-block:: console
   :class: cmd-guest

   bootstrap login: root
   [root@bootstrap:~]# uname --kernel-release
   6.18.57

:kbd:`Ctrl-O` leaves the console.

.. _direct-drop-in:

7. The closure in a drop-in
===========================

``kernel=`` and ``initrd=`` follow the ``result`` link, but ``init=`` names one
build, and every ``nix build`` makes a new one. Move ``append=`` to a
drop-in: the files in ``bootstrap-direct.qemu.d/`` are appended to
``bootstrap-direct.qemu``, so a rebuild changes only the drop-in.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

    initrd=%h/bootstrap/direct/result/initrd
   -append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
    netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10047-:22
    device=virtio-net-pci-non-transitional,netdev=net0,romfile=

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/.config/qemu/systemd/bootstrap-direct.qemu.d

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu.d/10-closure.conf

   [QEMU]
   append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init

To see what the generator makes of the two files without writing
anything:

.. code-block:: console
   :class: cmd-host

   $ qemu-system-generator --user --dryrun
   ---qemu-system@bootstrap-direct.service---
   ...
       -append "console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init" \
   ...
   ---virtiofsd@bootstrap-direct-store.socket---
   ...
   ---virtiofsd@bootstrap-direct-store.service---
   ...

:ref:`Regenerate <host-regenerate>` by hand after editing the drop-in.

.. _direct-settings:

8. systemd settings
===================

:cmd:`[Unit] <systemd.unit(5)>`, :cmd:`[Service] <systemd.service(5)>` and
``[Install]`` are copied into the generated service, so
the machine takes a description, `resource limits`_ and an install target
the way a hand-written unit does:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

    gid-map=:0:%G:1:
   +
   +[Unit]
   +Description=NixOS Bootstrap Guest
   +
   +[Service]
   +MemoryMax=3G
   +CPUWeight=50
   +
   +[Install]
   +WantedBy=machines.target

.. code-block:: console
   :class: cmd-host

   $ systemctl --user status qemu-system@bootstrap-direct.service
   ● qemu-system@bootstrap-direct.service - NixOS Bootstrap Guest
        Loaded: loaded (/home/user/.config/qemu/systemd/bootstrap-direct.qemu; enabled-runtime; preset: enabled)
        Active: active (running) since Thu 2026-10-22 16:46:55 CEST; 13s ago
        Memory: 340.4M (max: 3G, available: 2.6G, peak: 341.5M)

With :cmd:`WantedBy=machines.target <WantedBy=>`, ``systemctl --user start
machines.target`` starts the machine too. Every generated machine is
also part of :cmd:`machines.target`, so ``systemctl --user stop
machines.target`` stops all of them, not only this one.

.. _direct-share:

9. Your home
============

Share your home as :doc:`home` describes. Add its section to the file:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

    gid-map=:0:%G:1:

   +[virtiofsd]
   +shared-dir=%h
   +tag=home
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:
   +
    [Unit]

Save :ref:`home.nix <home-guest>` as ``~/bootstrap/direct/home.nix``, with
your home in place of ``/home/user``, and add it to the guest:

.. code-block:: diff
   :caption: ~/bootstrap/direct/flake.nix

            modules = [
              ./configuration.nix
              ./virtiofs.nix
   +          ./home.nix
            ];

Rebuild and put the path ``readlink result`` prints, followed by
``/init``, in the drop-in's ``init=``. Then
:ref:`regenerate <host-regenerate>` and :ref:`restart <host-start>`.
Root's home is now yours, with the two systemd directories empty, and a
file the guest writes belongs to you on the host:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-direct 'pwd; ls --almost-all ~/.config/systemd ~/.local/share/systemd; touch ~/bootstrap/direct/from-guest'
   Warning: Permanently added 'vsock/47' (ED25519) to the list of known hosts.
   /home/user
   /home/user/.config/systemd:

   /home/user/.local/share/systemd:
   $ ls --format=long ~/bootstrap/direct/from-guest
   -rw-r--r-- 1 user user 0 Oct 27 13:47 /home/user/bootstrap/direct/from-guest

.. _direct-link:

10. The closure through the link
================================

Every rebuild so far meant a new path in ``init=``. The initrd resolves
``init=`` inside the guest's root and follows symbolic links on the way
(``initrd-find-nixos-closure`` in nixpkgs'
``nixos/modules/system/boot/systemd/initrd.nix``), so ``init=`` can name
the ``result`` link in your home, once the initrd mounts the home:

.. code-block:: diff
   :caption: ~/bootstrap/direct/home.nix

      fileSystems.${home} = {
        device = "home";
        fsType = "virtiofs";
   +    neededForBoot = true;
      };

Rebuild, and change the drop-in one last time. ``%h`` is your home on
the host, and the guest mounts it at the same path:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu.d/10-closure.conf

    [QEMU]
   -append=console=ttyS0 console=hvc0 init=/nix/store/...-nixos-system-bootstrap-.../init
   +append=console=ttyS0 console=hvc0 init=%h/bootstrap/direct/result/init

:ref:`Regenerate <host-regenerate>` and :ref:`restart <host-start>`.
From here a change to the guest is a rebuild and a restart, and the
drop-in stays as it is. The boot now depends on the home share: if it
fails to mount, the guest stops in the initrd.

.. _direct-peer:

11. A second machine on one network
===================================

A second machine, ``bootstrap-direct-peer``, joins ``bootstrap-direct`` on a
multicast segment and boots the same closure, which holds both machines'
addresses (:doc:`network`):

.. code-block:: diff
   :caption: ~/bootstrap/direct/configuration.nix

      networking.hostName = "bootstrap";
   +  networking.useNetworkd = true;
   +  systemd.network.networks = {
   +    "10-lan-bootstrap" = {
   +      matchConfig.MACAddress = "52:54:00:00:00:47";
   +      address = [ "10.0.100.47/24" ];
   +    };
   +    "10-lan-peer" = {
   +      matchConfig.MACAddress = "52:54:00:00:00:48";
   +      address = [ "10.0.100.48/24" ];
   +    };
   +  };
      services.openssh.enable = true;

With ``networking.useNetworkd``, NixOS asks for the user-mode card's
address over DHCP through systemd-networkd too, from its
``99-ethernet-default-dhcp`` network, and the two ``10-`` networks come
before it. Rebuild:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/direct
   $ nix build

Give ``bootstrap-direct`` its card:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

    netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10047-:22
    device=virtio-net-pci-non-transitional,netdev=net0,romfile=
   +netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5556,local.type=inet,local.host=127.0.0.1,local.port=0
   +device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:47,romfile=
    device=vhost-vsock-pci-non-transitional,guest-cid=47

``bootstrap-direct-peer`` needs the store and the home, which the closure
mounts, a context id of its own and no forwarded port, since
``bootstrap-direct`` holds 10047:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-direct-peer.qemu

   [VM]
   SSHPrivateKey=%h/bootstrap/system/ssh/id_ed25519

   [QEMU]
   machine=q35
   nodefaults=
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio
   kernel=%h/bootstrap/direct/result/kernel
   initrd=%h/bootstrap/direct/result/initrd
   netdev=user,id=net0
   device=virtio-net-pci-non-transitional,netdev=net0,romfile=
   netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5556,local.type=inet,local.host=127.0.0.1,local.port=0
   device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:48,romfile=
   device=vhost-vsock-pci-non-transitional,guest-cid=48

   [virtiofsd]
   shared-dir=/nix/store
   tag=store
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h
   tag=home
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

Its drop-in names the same link, and gives the machine its own
hostname:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/.config/qemu/systemd/bootstrap-direct-peer.qemu.d

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-direct-peer.qemu.d/10-closure.conf

   [QEMU]
   append=console=ttyS0 systemd.hostname=peer init=%h/bootstrap/direct/result/init

:ref:`Regenerate <host-regenerate>`, :ref:`restart <host-start>`
``bootstrap-direct``, start ``bootstrap-direct-peer``, and each reaches
the other:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap-direct.service
   $ systemctl --user start qemu-system@bootstrap-direct-peer.service
   $ ssh bootstrap-direct-peer 'hostname; ping -c 1 10.0.100.47'
   Warning: Permanently added 'vsock/48' (ED25519) to the list of known hosts.
   peer
   PING 10.0.100.47 (10.0.100.47) 56(84) bytes of data.
   64 bytes from 10.0.100.47: icmp_seq=1 ttl=64 time=0.342 ms
   ...
   $ ssh bootstrap-direct ping -c 1 10.0.100.48
   PING 10.0.100.48 (10.0.100.48) 56(84) bytes of data.
   64 bytes from 10.0.100.48: icmp_seq=1 ttl=64 time=0.261 ms
   ...

.. _direct-fast:

12. A faster boot
=================

Each step is timed as :ref:`fast-measure` shows.

The machine as it stands takes 8.4 to 8.5 seconds.

The console
-----------

:ref:`fast-console` explains these settings.

Add them to the drop-in:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu.d/10-closure.conf

    [QEMU]
   -append=console=ttyS0 console=hvc0 init=%h/bootstrap/direct/result/init
   +append=console=ttyS0 console=hvc0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 init=%h/bootstrap/direct/result/init

:ref:`Regenerate <host-regenerate>` and :ref:`restart <host-start>`: 6.3
seconds, and the initrd's share falls from 2.9 to 2.0.

The store
---------

:ref:`fast-store` explains why the store, and only the store.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

    [virtiofsd]
    shared-dir=/nix/store
    tag=store
   +cache=always
    sandbox=namespace

5.7 seconds.

The closure
-----------

:ref:`fast-closure` explains these four settings.

.. code-block:: diff
   :caption: ~/bootstrap/direct/configuration.nix

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

Rebuild and :ref:`restart <host-start>`:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/direct
   $ nix build

4.9 seconds.

More processors
---------------

systemd starts what it can in parallel:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

   -smp=2
   +smp=4

4.9 seconds, no faster than two processors on this guest.

Quiet
-----

:ref:`fast-quiet` explains what it costs.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu.d/10-closure.conf

    [QEMU]
   -append=console=ttyS0 console=hvc0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 init=%h/bootstrap/direct/result/init
   +append=console=ttyS0 console=hvc0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 quiet init=%h/bootstrap/direct/result/init

4.4 seconds, about half the time this section started from.

The initrd still waits about 0.8 seconds on systemd's mount rate limit,
which this guest cannot lift safely (:ref:`fast-mounts`).

.. _direct-numa:

13. Two NUMA nodes
==================

This step and the next are optional and change ``bootstrap-direct-peer``
only. For a virtiofs share, the generator backs all of a machine's memory
with one shared memory backend of its own, unless the file supplies the
memory itself. A file that splits the memory into NUMA nodes does, one
backend per node, and QEMU refuses the generator's backend beside them.
The backends must add up to ``m=2G``, and must be shared for virtiofsd to
reach the guest's memory. Put the nodes in a drop-in:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-direct-peer.qemu.d/20-numa.conf

   [QEMU]
   object=memory-backend-memfd,id=ram0,size=1G,share=on
   object=memory-backend-memfd,id=ram1,size=1G,share=on
   numa=node,nodeid=0,cpus=0,memdev=ram0
   numa=node,nodeid=1,cpus=1,memdev=ram1

:ref:`Regenerate <host-regenerate>` and :ref:`restart <host-start>`. The
unit holds no memory backend of the generator's, and the guest has two
nodes:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap-direct-peer.service
   $ systemctl --user cat qemu-system@bootstrap-direct-peer.service | grep --count systemd-mem
   0
   $ ssh bootstrap-direct-peer ls /sys/devices/system/node
   has_cpu
   has_generic_initiator
   has_memory
   has_normal_memory
   node0
   node1
   online
   possible
   power
   uevent

.. _direct-json:

14. A device written as JSON
============================

QEMU also takes a ``-device`` value as a JSON object, and some devices
only that way: ``vfio-user-pci`` names its server's socket as an object.
The generator reads a JSON line as it reads the comma-separated form, so
machined still records the context id. Write the peer's vsock device as
JSON:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct-peer.qemu

    device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:48,romfile=
   -device=vhost-vsock-pci-non-transitional,guest-cid=48
   +device={"driver": "vhost-vsock-pci-non-transitional", "guest-cid": 48}

Regenerate, restart, and ask machined:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap-direct-peer.service
   $ machinectl --user show --property=VSockCID bootstrap-direct-peer
   VSockCID=48

.. _direct-microvm:

15. A microvm
=============

This step is optional, and the machine is no faster for it. QEMU's
`microvm`_ machine type has no PCI bus unless asked for one, and its
firmware does little more than hand over to ``kernel=``: its devices
sit on virtio-mmio, and the kernel reaches its ``/init`` sooner. In
exchange, it gives up what the other guides rely on
(``docs/system/i386/microvm.rst`` and ``hw/i386/microvm.c`` in QEMU):

- No PCI device, so no VFIO passthrough and no ``nic=``, unless
  ``pcie=on`` adds a PCI Express bus, which it does only with `ACPI`_.
- No hotplug of any kind: CPUs, disks and cards are fixed at start.
- No live migration across QEMU versions.
- No firmware boots a virtio-mmio disk, so the machine boots
  ``kernel=`` only; the disk and UKI guides cannot use it.
- 24 virtio-mmio slots with ACPI, eight without, and every virtio device
  takes one, the generator's virtiofs devices too.
- Without ACPI, no power button: with ``acpi=off`` the guest cannot
  power off, and every stop waits out :cmd:`TimeoutStopSec=`. Keep ACPI,
  microvm's default.

microvm reserves no room for its ACPI tables when it loads the initrd:
``microvm_memory_init()`` in ``hw/i386/microvm.c`` passes
``x86_load_linux()`` an ``acpi_data_size`` of 0, where the PC machines
pass ``PC_FW_DATA``. An initrd that does not end on a page boundary can
end over the tables, in the top page of RAM, and the guest stops at
``ACPI: Core revision``, depending on the initrd's size and the number of
devices. Boot a copy padded to whole pages, and make it again after every
build:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/direct
   $ cp --dereference --force result/initrd initrd
   $ chmod u+w initrd
   $ truncate --size=%4096 initrd

Then change the machine. On microvm the virtio devices take their
``-device`` names instead of the ``-pci-non-transitional`` ones, and have
no boot ROM to leave out. The user-mode card was already a ``netdev=``
and a ``device=`` that names it; ``nic=`` would still open the forwarded
port, but microvm never creates the card it asks for, since only the PC
machines' PCI setup does (``pci_init_nic_devices()`` in
``hw/i386/pc.c``). The ``global=`` line has QEMU's virtio-mmio transport
offer virtio 1.0, as a non-transitional PCI device does; QEMU defaults
to the legacy transport (``force-legacy`` in ``hw/virtio/virtio-mmio.c``).
Under `KVM`_, microvm leaves the RTC out unless ``rtc=on`` asks for it
(``hw/i386/microvm.c``). Without it the kernel's RTC probe fails, and
``/init`` started at 1.50 seconds instead of 0.30 here:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu

   -machine=q35
   +machine=microvm,rtc=on
    nodefaults=
    accel=kvm
    cpu=host
    m=2G
    smp=4
    nographic=
    serial=mon:stdio
    kernel=%h/bootstrap/direct/result/kernel
   -initrd=%h/bootstrap/direct/result/initrd
   +initrd=%h/bootstrap/direct/initrd
   +global=virtio-mmio.force-legacy=false
    netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10047-:22
   -device=virtio-net-pci-non-transitional,netdev=net0,romfile=
   +device=virtio-net-device,netdev=net0
    netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5556,local.type=inet,local.host=127.0.0.1,local.port=0
   -device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:47,romfile=
   -device=vhost-vsock-pci-non-transitional,guest-cid=47
   -device=virtio-serial-pci-non-transitional
   +device=virtio-net-device,netdev=lan,mac=52:54:00:00:00:47
   +device=vhost-vsock-device,guest-cid=47
   +device=virtio-serial-device

The generator follows the machine type for the virtiofs devices it adds:

.. code-block:: console
   :class: cmd-host

   $ qemu-system-generator --user --dryrun | grep vhost-user-fs
       -device vhost-user-fs-device,queue-size=1024,chardev=systemd-virtiofs-store,tag=store \
       -device vhost-user-fs-device,queue-size=1024,chardev=systemd-virtiofs-home,tag=home \

On microvm this guest's ``systemd-ssh-generator`` opens no vsock
listener by itself, so ask for one on the kernel command line with
``systemd.ssh_listen=vsock::22``:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-direct.qemu.d/10-closure.conf

    [QEMU]
   -append=console=ttyS0 console=hvc0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 quiet init=%h/bootstrap/direct/result/init
   +append=console=ttyS0 console=hvc0 systemd.ssh_listen=vsock::22 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 quiet init=%h/bootstrap/direct/result/init

:ref:`Regenerate <host-regenerate>`, :ref:`restart <host-start>` and log in:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap-direct.service
   $ ssh bootstrap-direct uname --kernel-release
   Warning: Permanently added 'vsock/47' (ED25519) to the list of known hosts.
   6.18.59

The kernel reaches ``/init`` at 0.30 seconds against 0.44 on q35, and
the initrd mounts the store at 1.44 seconds against 1.61, but ``ssh``
answers after 4.9 seconds against 4.4. NixOS's userspace outweighs what
microvm saves before it, so this guide, and the others, keep q35.
``bootstrap-direct-peer`` stays on q35; the LAN does not depend on the
machine type.

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
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _microvm: https://www.qemu.org/docs/master/system/i386/microvm.html
.. _ACPI: https://uefi.org/specifications
.. _Linux: https://www.kernel.org
.. _resource limits: https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _KVM: https://docs.kernel.org/virt/kvm/index.html
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
