.. SPDX-License-Identifier: copyleft-next-0.3.1

=====================================
Bootstrap from a kernel and an initrd
=====================================

This guide boots a `NixOS`_ guest under your user's `systemd`_ manager
with no firmware and no disk. `QEMU`_ loads the guest's kernel and `initrd`_
itself, the root file system is a `tmpfs`_, and ``/nix/store`` is the
host's own, shared over `virtiofs`_. Its :key:`.qemu` file grows by one
feature per step, and no step needs root.

The file format is specified in
:doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` and the generator in
:doc:`qemu-system-generator(8) </man/qemu-system-generator.8>`. The outputs
below are real, trimmed to the lines that matter, with the home directory
shown as ``/home/user``.

.. include:: _prepare.rst

The guest
---------

The guest is an ordinary NixOS configuration. Create
``~/bootstrap/kernel/flake.nix``:

.. code-block:: nix
   :caption: ~/bootstrap/kernel/flake.nix

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

``~/bootstrap/kernel/configuration.nix`` sets the guest up, with the
public key from ``~/bootstrap/ssh/id_ed25519.pub`` pasted in:

.. code-block:: nix
   :caption: ~/bootstrap/kernel/configuration.nix

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
:ref:`step 6 <kernel-console>`.

``~/bootstrap/kernel/virtiofs.nix`` has no boot loader and no disk.
The root is a tmpfs, and the store arrives over virtiofs under the tag
``store``, which :ref:`step 2 <kernel-store>` sets up:

.. code-block:: nix
   :caption: ~/bootstrap/kernel/virtiofs.nix

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

   $ cd ~/bootstrap/kernel
   $ nix build

``result`` now holds the system. Its ``kernel`` and ``initrd`` are what QEMU
boots, and its ``init`` is what the kernel runs once the store is mounted:

.. code-block:: console
   :class: cmd-host

   $ ls -l result/kernel result/initrd
   lrwxrwxrwx 1 root root 71 Jan  1  1970 result/initrd -> /nix/store/3rxnmhxn6xfh2hwiakgircj03j4gvd1g-initrd-linux-6.18.57/initrd
   lrwxrwxrwx 1 root root 65 Jan  1  1970 result/kernel -> /nix/store/maq0fxv0r4p9bac4haqy45c4vhlc33ag-linux-6.18.57/bzImage

.. include:: _generator.rst

.. _kernel-kernel:

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
   kernel=%h/bootstrap/kernel/result/kernel
   initrd=%h/bootstrap/kernel/result/initrd
   append=console=ttyS0

.. include:: _start.rst

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

.. include:: _kill.rst

.. _kernel-store:

2. The store
============

Two things are missing: the store under the tag ``store``, and ``init=``,
which tells the initrd which system in the store to boot:

.. code-block:: console
   :class: cmd-host

   $ readlink ~/bootstrap/kernel/result
   /nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f

Add both, with your own path in ``init=``:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    kernel=%h/bootstrap/kernel/result/kernel
    initrd=%h/bootstrap/kernel/result/initrd
   -append=console=ttyS0
   +append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
   +
   +[virtiofsd]
   +shared-dir=/nix/store
   +tag=store
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:

Each :key:`[virtiofsd]` section runs one `virtiofsd`_ daemon for one shared
directory. Its keys are the long options of ``virtiofsd --help``
without the dashes. The generator runs the daemon from its own socket
unit and adds the QEMU lines that connect to it. ``sandbox=``,
``uid-map=`` and ``gid-map=`` let it run without root: ``%U`` and ``%G``
are your user and group ids, which the guest's root maps to.

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

.. _kernel-network:

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

   $ ssh -i ~/bootstrap/ssh/id_ed25519 -p 10022 root@127.0.0.1 uname -r
   6.18.57

.. _kernel-vsock:

4. vsock
========

A vsock device gives the guest a context id, a socket address the host
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

With the ``Include`` in your ``~/.ssh/config``, the machine's name is an
``ssh`` alias:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap systemctl is-system-running
   Warning: Permanently added 'vsock/42' (ED25519) to the list of known hosts.
   running

.. _kernel-poweroff:

5. Poweroff over SSH
====================

:key:`[VM]` holds what is not a QEMU option. With :key:`SshPrivateKey=`,
stopping the machine first runs ``systemctl poweroff`` in the guest over
vsock, and presses the power button only if that fails:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

   +[VM]
   +SshPrivateKey=%h/bootstrap/ssh/id_ed25519
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

.. _kernel-console:

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
   [root@bootstrap:~]# uname -r
   6.18.57

.. _kernel-drop-in:

7. The closure in a drop-in
===========================

``kernel=`` and ``initrd=`` follow the ``result`` link, but ``init=`` names one
build, and every ``nix build`` makes a new one. Move ``append=`` to a
drop-in: the files in ``bootstrap.qemu.d/`` are appended to
``bootstrap.qemu``, so a command can rewrite that one line without
touching the file you edit by hand.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    initrd=%h/bootstrap/kernel/result/initrd
   -append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
    nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22

.. code-block:: shell
   :class: cmd-host

   mkdir --parents ~/.config/qemu/systemd/bootstrap.qemu.d
   cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
   [QEMU]
   append=console=ttyS0 console=hvc0 init=$(readlink ~/bootstrap/kernel/result)/init
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

.. _kernel-settings:

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

.. _kernel-share:

9. A share of your own
======================

Sharing a directory of your own takes another ``[virtiofsd]`` section.
Make the directory and put a file in it:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/share
   $ echo "hello from the host" > ~/bootstrap/share/hello.txt

Then add the section:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    gid-map=:0:%G:1:

   +[virtiofsd]
   +shared-dir=%h/bootstrap/share
   +tag=host
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:
   +
    [Unit]

The guest mounts it by tag, so ``virtiofs.nix`` changes too:

.. code-block:: diff
   :caption: ~/bootstrap/kernel/virtiofs.nix

      fileSystems."/nix/store" = {
        device = "store";
        fsType = "virtiofs";
      };
   +  fileSystems."/mnt/host" = {
   +    device = "host";
   +    fsType = "virtiofs";
   +  };

Rebuild, rewrite the drop-in as in :ref:`step 7 <kernel-drop-in>`,
regenerate and restart:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap 'cat /mnt/host/hello.txt; touch /mnt/host/from-guest'
   Warning: Permanently added 'vsock/42' (ED25519) to the list of known hosts.
   hello from the host
   $ ls -l ~/bootstrap/share
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
  for :cmd:`TimeoutStopSec=`, two minutes, before killing it;
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
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _Linux: https://www.kernel.org
.. _resource limits: https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html
