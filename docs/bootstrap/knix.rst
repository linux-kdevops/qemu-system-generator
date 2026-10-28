.. SPDX-License-Identifier: copyleft-next-0.3.1

=======================================
Bootstrap with your own kernel and QEMU
=======================================

This guide boots a `NixOS`_ guest under your user's `systemd`_ manager on a
`Linux`_ kernel and a `QEMU`_ you build yourself. The system stays in the
host's ``/nix/store``, shared over `virtiofs`_, and so do the kernel's
modules.

Why knix
========

A NixOS system from `nixpkgs`_ carries its kernel: the `initrd`_ holds that
kernel's modules, and ``/lib/modules`` in the running system is that
kernel's too. nixpkgs can build a kernel from your tree, but as a `Nix`_
derivation, which rebuilds from scratch after every change.

`knix`_ keeps the kernel out of the closure. Its ``imageless`` backend
builds a NixOS system with no kernel of its own and an initrd with no
modules, which expects the boot-critical drivers built in and mounts
``/lib/modules`` over virtiofs under the tag ``modules``. You build the
kernel in your tree with :cmd:`make` as usual, and the same closure boots
every build. knix also provides the ``build-kernel`` and ``build-qemu``
shells, which bring the toolchains.

Prepare the host as :doc:`host` shows.

The kernel
----------

Configure the release as a `KVM`_ guest with virtiofs built in, build it
and install its modules next to it, inside the ``build-kernel`` shell
(:cmd:`nix develop`):

.. code-block:: console
   :class: cmd-host

   $ git clone --depth 1 --branch v7.3 \
       https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git ~/bootstrap/knix/linux
   $ cd ~/bootstrap/knix/linux
   $ K="nix develop github:linux-kdevops/knix#build-kernel --command"
   $ B=$HOME/bootstrap/knix/linux-build
   $ $K make O=$B defconfig kvm_guest.config
   $ $K scripts/config --file $B/.config \
       --enable FUSE_FS --enable VIRTIO_FS \
       --enable VSOCKETS --enable VIRTIO_VSOCKETS
   $ $K make O=$B olddefconfig
   $ $K make O=$B --jobs=$(nproc)
   $ $K make O=$B INSTALL_MOD_PATH=$HOME/bootstrap/knix/linux-install modules_install

``kvm_guest.config`` turns on the virtio drivers; the
`kernel build documentation`_ covers ``O=`` and the configuration targets.
The kernel is ``~/bootstrap/knix/linux-build/arch/x86/boot/bzImage``, and
the modules are under ``~/bootstrap/knix/linux-install/lib/modules/``.

QEMU
----

Build the release inside the ``build-qemu`` shell and install it in the
workspace (`QEMU build system`_):

.. code-block:: console
   :class: cmd-host

   $ git clone --depth 1 --branch v11.1.2 \
       https://gitlab.com/qemu-project/qemu.git ~/bootstrap/knix/qemu
   $ mkdir --parents ~/bootstrap/knix/qemu-build
   $ cd ~/bootstrap/knix/qemu-build
   $ Q="nix develop github:linux-kdevops/knix#build-qemu --command"
   $ $Q ~/bootstrap/knix/qemu/configure --prefix=$HOME/bootstrap/knix/qemu-install \
       --target-list=x86_64-softmmu --enable-kvm --enable-slirp --disable-docs
   $ $Q make --jobs=$(nproc) install

.. code-block:: console
   :class: cmd-host

   $ ~/bootstrap/knix/qemu-install/bin/qemu-system-x86_64 --version
   QEMU emulator version 11.1.2 (v11.1.2)

The guest
---------

Start from knix's template (:cmd:`nix flake init`) and point its input at
knix upstream:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/knix/guest
   $ cd ~/bootstrap/knix/guest
   $ nix flake init --template github:linux-kdevops/knix

.. code-block:: diff
   :caption: ~/bootstrap/knix/guest/flake.nix

   -    knix.url = "path:/path/to/knix";
   +    knix.url = "github:linux-kdevops/knix";

Save :ref:`home.nix <home-guest>` as ``~/bootstrap/knix/guest/home.nix``,
with your home in place of ``/home/user``, to share your home as
:doc:`home` describes. Import it, name the guest, and paste the public
key from ``~/bootstrap/system/ssh/id_ed25519.pub`` into ``default.nix``:

.. code-block:: diff
   :caption: ~/bootstrap/knix/guest/default.nix

        #   knix.nixosModules.testSuites.blktests
   +    ./home.nix
      ];

   -  networking.hostName = "vm";
   +  networking.hostName = "bootstrap";

   ...
      # SSH keys for the root and unprivileged accounts.
   -  # users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 ..." ];
   -  # users.users.${config.knix.user.name}.openssh.authorizedKeys.keys = [ "ssh-ed25519 ..." ];
   +  users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ]; # from ~/bootstrap/system/ssh/id_ed25519.pub
   +  users.users.${config.knix.user.name}.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ]; # from ~/bootstrap/system/ssh/id_ed25519.pub

.. code-block:: console
   :class: cmd-host

   $ nix build .#toplevel

This closure has no ``kernel`` or ``initrd`` link. Its ``boot.json``, a NixOS
`bootspec`_ document, names the initrd and ``init``:

.. code-block:: console
   :class: cmd-host

   $ jq '."org.nixos.bootspec.v1" | {initrd, init}' result/boot.json
   {
     "initrd": "/nix/store/pbx7c8f26kng743zyrabx8p9qplpi6zj-initrd-linux-6.18.34/initrd",
     "init": "/nix/store/m907x9z5b9cab8ab57cgs1sjsh8i4ghr-nixos-system-bootstrap-26.05.20260608.bd0ff2d/init"
   }

The initrd's name mentions a nixpkgs kernel, but knix empties its module
lists, so it boots any kernel that has the boot-critical drivers built
in.

.. _knix-kernel:

1. Your kernel
==============

Create ``~/.config/qemu/systemd/bootstrap-knix.qemu``. The kernel comes from
your build, and three shares bring the store, ``/lib/modules`` and your
home:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu

   [VM]
   SshPrivateKey=%h/bootstrap/system/ssh/id_ed25519

   [QEMU]
   machine=q35
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio
   kernel=%h/bootstrap/knix/linux-build/arch/x86/boot/bzImage
   nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10049-:22
   device=vhost-vsock-pci-non-transitional,guest-cid=49

   [virtiofsd]
   shared-dir=/nix/store
   tag=store
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h/bootstrap/knix/linux-install/lib/modules
   tag=modules
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h
   tag=home
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

``nic=`` forwards a local port to the guest's SSH over QEMU's
`user-mode network`_, and the `vsock`_ device gives the guest the context id
:ref:`host-ssh` names. :key:`SshPrivateKey=` has a stop power the guest
off over SSH first (:ref:`SSH poweroff <host-poweroff>`), and each
:key:`[virtiofsd]` section shares one directory without root
(:ref:`host-virtiofsd`).

The initrd and ``init=`` name one build of the closure, and every
:cmd:`nix build` makes a new one, so they go in a drop-in: the files in
``bootstrap-knix.qemu.d/`` are appended to ``bootstrap-knix.qemu``.
Put in the two paths ``boot.json`` names, and ``root=tmpfs``, which knix
wants on the command line:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/.config/qemu/systemd/bootstrap-knix.qemu.d

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu.d/10-closure.conf

   [QEMU]
   initrd=/nix/store/pbx7c8f26kng743zyrabx8p9qplpi6zj-initrd-linux-6.18.34/initrd
   append=root=tmpfs console=ttyS0 init=/nix/store/m907x9z5b9cab8ab57cgs1sjsh8i4ghr-nixos-system-bootstrap-26.05.20260608.bd0ff2d/init

:ref:`Regenerate <host-regenerate>` and :ref:`start <host-start>` it; the
journal has the serial console.

The machine registers with :cmd:`machined <systemd-machined>`
(:ref:`host-machined`), which records the id:

.. code-block:: console
   :class: cmd-host

   $ machinectl --user list
   MACHINE        CLASS SERVICE     OS VERSION ADDRESSES
   bootstrap-knix vm    qemu-system -  -       -

:ref:`host-ssh` already names the machine by its context id. Log in and
look at the shares:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-knix \
       'uname --kernel-release; findmnt --noheadings --output=TARGET,SOURCE,FSTYPE --types=virtiofs'
   Warning: Permanently added 'vsock/49' (ED25519) to the list of known hosts.
   7.3.0
   /nix/store   store   virtiofs
   └─/nix/store store   virtiofs
   /lib/modules modules virtiofs
   /home/user   home    virtiofs

After a kernel rebuild, ``make modules_install`` again and
:ref:`restart <host-start>`. After a closure rebuild, put the new paths
from ``boot.json`` in the drop-in, :ref:`regenerate <host-regenerate>`
and :ref:`restart <host-start>`; the path unit does not watch the
drop-in directory.

.. _knix-qemu:

2. Your QEMU
============

:key:`QemuBinary=` replaces the QEMU the generator was built with. It becomes
the first word of :cmd:`ExecStart=`:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu

    [VM]
   +QemuBinary=%h/bootstrap/knix/qemu-install/bin/qemu-system-x86_64
    SshPrivateKey=%h/bootstrap/system/ssh/id_ed25519

:ref:`Regenerate <host-regenerate>` and :ref:`restart <host-start>`, then
check which binary runs:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap-knix.service
   $ readlink /proc/$(systemctl --user show --property=MainPID --value qemu-system@bootstrap-knix.service)/exe
   /home/user/bootstrap/knix/qemu-install/bin/qemu-system-x86_64

The guest boots in about 9 seconds and stops in 2.

.. _knix-peer:

3. A second machine on one network
==================================

A second machine, ``bootstrap-knix-peer``, joins ``bootstrap-knix`` on a
multicast segment and boots the same closure, which holds both machines'
addresses (:doc:`network`):

.. code-block:: diff
   :caption: ~/bootstrap/knix/guest/default.nix

      networking.hostName = "bootstrap";
   +  systemd.network.networks = {
   +    "10-lan-bootstrap" = {
   +      matchConfig.MACAddress = "52:54:00:00:00:49";
   +      address = [ "10.0.100.49/24" ];
   +    };
   +    "10-lan-peer" = {
   +      matchConfig.MACAddress = "52:54:00:00:00:50";
   +      address = [ "10.0.100.50/24" ];
   +    };
   +  };

The two ``10-`` networks come before knix's ``80-ethernet``, which asks
for every other card's address over DHCP. Rebuild, and put the new
paths in the drop-in:

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/knix/guest
   $ nix build .#toplevel
   $ jq '."org.nixos.bootspec.v1" | {initrd, init}' result/boot.json

Give ``bootstrap-knix`` its card:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu

    nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10049-:22
   +netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5557,local.type=inet,local.host=127.0.0.1,local.port=0
   +device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:49
    device=vhost-vsock-pci-non-transitional,guest-cid=49

``bootstrap-knix-peer`` needs the three shares the closure mounts, a
context id of its own and no forwarded port, since ``bootstrap-knix`` holds
10049:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-knix-peer.qemu

   [VM]
   QemuBinary=%h/bootstrap/knix/qemu-install/bin/qemu-system-x86_64
   SshPrivateKey=%h/bootstrap/system/ssh/id_ed25519

   [QEMU]
   machine=q35
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio
   kernel=%h/bootstrap/knix/linux-build/arch/x86/boot/bzImage
   nic=user,model=virtio-net-pci-non-transitional
   netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5557,local.type=inet,local.host=127.0.0.1,local.port=0
   device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:50
   device=vhost-vsock-pci-non-transitional,guest-cid=50

   [virtiofsd]
   shared-dir=/nix/store
   tag=store
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h/bootstrap/knix/linux-install/lib/modules
   tag=modules
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h
   tag=home
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

Its drop-in names the same closure, and gives the machine its own
hostname:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/.config/qemu/systemd/bootstrap-knix-peer.qemu.d

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-knix-peer.qemu.d/10-closure.conf

   [QEMU]
   initrd=/nix/store/...-initrd-linux-.../initrd
   append=root=tmpfs console=ttyS0 systemd.hostname=peer init=/nix/store/...-nixos-system-bootstrap-.../init

:ref:`Regenerate <host-regenerate>`, :ref:`restart <host-start>`
``bootstrap-knix``, start ``bootstrap-knix-peer``, and each reaches the
other:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap-knix.service
   $ systemctl --user start qemu-system@bootstrap-knix-peer.service
   $ ssh bootstrap-knix-peer 'hostname; ping -c 1 10.0.100.49'
   Warning: Permanently added 'vsock/50' (ED25519) to the list of known hosts.
   peer
   PING 10.0.100.49 (10.0.100.49) 56(84) bytes of data.
   64 bytes from 10.0.100.49: icmp_seq=1 ttl=64 time=0.322 ms
   ...
   $ ssh bootstrap-knix ping -c 1 10.0.100.50
   PING 10.0.100.50 (10.0.100.50) 56(84) bytes of data.
   64 bytes from 10.0.100.50: icmp_seq=1 ttl=64 time=0.185 ms
   ...

.. _knix-fast:

4. A faster boot
================

Your own kernel and QEMU allow three more changes: a kernel with only
what this machine uses, a smaller firmware, and no initrd of its own.

Each step is timed as :ref:`fast-measure` shows.

The machine as it stands takes 7.7 seconds.

A smaller kernel
----------------

``defconfig`` builds drivers for hardware this machine does not have and
makes a 15 MB image for the firmware to load. `kfragments`_ holds
fragments to merge onto an empty configuration, with a ``builtin/`` set
that turns a feature from a module into part of the kernel. Fetch them:

.. code-block:: console
   :class: cmd-host

   $ git clone --depth 1 https://github.com/linux-kdevops/kfragments.git \
       ~/bootstrap/knix/kfragments

What this machine needs beyond them goes in a fragment of your own:

.. code-block:: text
   :caption: ~/bootstrap/knix/fast.config

   # The machine: q35's PCI bus and ACPI, the RTC, a hardware RNG
   CONFIG_PCI=y
   CONFIG_ACPI=y
   CONFIG_RTC_CLASS=y
   CONFIG_RTC_DRV_CMOS=y
   CONFIG_HW_RANDOM_VIRTIO=y
   CONFIG_X86_X2APIC=y
   CONFIG_NR_CPUS=64
   # Devices for the other machine types and boot paths
   CONFIG_VIRTIO_MMIO=y
   CONFIG_VIRTIO_CONSOLE=y
   CONFIG_PVH=y
   # What systemd and the network stack expect
   CONFIG_NET=y
   CONFIG_INET=y
   CONFIG_PACKET=y
   CONFIG_UNIX=y
   CONFIG_BPF_SYSCALL=y
   CONFIG_CGROUP_BPF=y
   CONFIG_DEVTMPFS_MOUNT=y
   CONFIG_NO_HZ_IDLE=y
   CONFIG_PRINTK_TIME=y
   CONFIG_KERNEL_ZSTD=y

Merge it with the fragments for a systemd guest with virtiofs, virtio-net
and vsock built in, into a build directory of its own so that
``bootstrap-knix-peer`` keeps the first kernel (`kfragments`_ has the merge
rules):

.. code-block:: console
   :class: cmd-host

   $ cd ~/bootstrap/knix/linux
   $ K="nix develop github:linux-kdevops/knix#build-kernel --command"
   $ B=$HOME/bootstrap/knix/linux-fast-build
   $ F=$HOME/bootstrap/knix/kfragments/kernel/configs
   $ $K make O=$B allnoconfig
   $ $K scripts/kconfig/merge_config.sh -y -n -O $B $B/.config \
       $F/core/64bit.config $F/core/core.config $F/core/modules.config \
       $F/core/systemd.config $F/core/initrd.config $F/arch/x86_64.config \
       $F/core/acpi-poweroff.config $F/virt/virtio-fs.config \
       $F/virt/virtio-net.config $F/virt/vsock.config $F/core/localversion.config \
       $F/builtin/core/systemd.config $F/builtin/virt/virtio-fs.config \
       $F/builtin/virt/virtio-net.config $F/builtin/virt/vsock.config \
       $HOME/bootstrap/knix/fast.config
   $ $K make O=$B olddefconfig
   $ $K make O=$B --jobs=$(nproc)
   $ $K make O=$B INSTALL_MOD_PATH=$HOME/bootstrap/knix/linux-fast-install modules_install

It builds in about a minute and a half, into a 4.4 MB image. Point the
machine at it and at its modules:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu

   -kernel=%h/bootstrap/knix/linux-build/arch/x86/boot/bzImage
   +kernel=%h/bootstrap/knix/linux-fast-build/arch/x86/boot/bzImage
   ...
    [virtiofsd]
   -shared-dir=%h/bootstrap/knix/linux-install/lib/modules
   +shared-dir=%h/bootstrap/knix/linux-fast-install/lib/modules

7.6 seconds. The kernel's share falls from 0.8 to 0.3 seconds, but
userspace takes longer and eats most of that back.

The console
-----------

:ref:`fast-console` explains these settings.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu.d/10-closure.conf

    [QEMU]
    initrd=/nix/store/...-initrd-linux-.../initrd
   -append=root=tmpfs console=ttyS0 init=/nix/store/...-nixos-system-bootstrap-.../init
   +append=root=tmpfs console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 init=/nix/store/...-nixos-system-bootstrap-.../init

:ref:`Regenerate <host-regenerate>` and :ref:`restart <host-start>`: 5.6
seconds.

The store
---------

:ref:`fast-store` explains why the store, and only the store.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu

    [virtiofsd]
    shared-dir=/nix/store
    tag=store
   +cache=always
    sandbox=namespace

5.0 seconds.

The firmware
------------

QEMU ships `qboot`_, "a simple x86 firmware that can boot Linux" from
``kernel=``, as ``qboot.rom``. It can replace `SeaBIOS`_ on q35:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu

    machine=q35
   +bios=qboot.rom

4.7 seconds; the time before the kernel starts falls from 0.24 to 0.15
seconds.

The mount rate limit
--------------------

This kernel has virtiofs built in, so the limit can go
(:ref:`fast-mounts`):

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu.d/10-closure.conf

    [QEMU]
    initrd=/nix/store/...-initrd-linux-.../initrd
   -append=root=tmpfs console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 init=/nix/store/...-nixos-system-bootstrap-.../init
   +append=root=tmpfs console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST=1000 init=/nix/store/...-nixos-system-bootstrap-.../init

3.3 seconds, and the initrd's share falls from 1.6 to 0.7.

The closure
-----------

:ref:`fast-closure` explains these four settings.

A fifth one is on the kernel command line: systemd applies the preset
policy on a first boot, and ``systemd.condition_first_boot=false``
tells it this is not one (:cmd:`kernel-command-line(7)`).

.. code-block:: diff
   :caption: ~/bootstrap/knix/guest/default.nix

      networking.hostName = "bootstrap";
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

   $ cd ~/bootstrap/knix/guest
   $ nix build .#toplevel
   $ jq '."org.nixos.bootspec.v1" | {initrd, init}' result/boot.json

Put the new paths in the drop-in, and the option:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu.d/10-closure.conf

    [QEMU]
    initrd=/nix/store/...-initrd-linux-.../initrd
   -append=root=tmpfs console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST=1000 init=/nix/store/...-nixos-system-bootstrap-.../init
   +append=root=tmpfs console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST=1000 systemd.condition_first_boot=false init=/nix/store/...-nixos-system-bootstrap-.../init

2.6 seconds.

Quiet
-----

:ref:`fast-quiet` explains what it costs.

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu.d/10-closure.conf

    [QEMU]
    initrd=/nix/store/...-initrd-linux-.../initrd
   -append=root=tmpfs console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST=1000 systemd.condition_first_boot=false init=/nix/store/...-nixos-system-bootstrap-.../init
   +append=root=tmpfs console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST=1000 systemd.condition_first_boot=false quiet init=/nix/store/...-nixos-system-bootstrap-.../init

2.0 seconds.

No initrd
---------

The initrd's systemd starts, mounts the root and the store, runs the
NixOS activation and hands over to the system's systemd, which starts
again and loads every unit a second time. Without a systemd initrd, the
system's systemd starts once. The closure already has what that takes:
with the systemd initrd, NixOS copies its stage 2 script to
``prepare-root`` (``nixos/modules/system/activation/top-level.nix`` in
nixpkgs), and that script, run as PID 1, mounts what is missing, runs
the activation and starts systemd
(``nixos/modules/system/boot/stage-2-init.sh``). It only needs the store
mounted first. A static `BusyBox`_ does that from an initramfs of 1.4 MB:

.. code-block:: shell
   :class: cmd-host

   mkdir --parents ~/bootstrap/knix/tiny/bin
   install --mode=0755 "$(nix build --no-link --print-out-paths nixpkgs#pkgsStatic.busybox)/bin/busybox" \
       ~/bootstrap/knix/tiny/bin/busybox

.. code-block:: sh
   :caption: ~/bootstrap/knix/tiny/init

   #!/bin/busybox sh
   /bin/busybox mkdir -p /dev /proc /sys /nix/store
   /bin/busybox mount -t devtmpfs devtmpfs /dev
   /bin/busybox mount -t proc proc /proc
   /bin/busybox ln -s /proc/self/fd /dev/fd
   exec </dev/console >/dev/console 2>&1
   /bin/busybox mount -t virtiofs store /nix/store || exec /bin/busybox sh
   exec "$CLOSURE/prepare-root"

``prepare-root`` reads ``/proc/cmdline`` and writes through ``/dev/fd``,
hence the two lines before the store. The archive's root becomes the
guest's ``/``, and :cmd:`sshd <sshd(8)>` refuses an authorized keys file
when it or any directory above it is writable by group or others
(``safe_path()`` in `OpenSSH`_'s ``misc.c``), so set the modes before
packing it:

.. code-block:: shell
   :class: cmd-host

   chmod 0755 ~/bootstrap/knix/tiny ~/bootstrap/knix/tiny/bin ~/bootstrap/knix/tiny/init
   cd ~/bootstrap/knix/tiny
   find . | cpio --quiet --create --format=newc --owner=0:0 > ~/bootstrap/knix/tiny.cpio

The drop-in boots it with the closure's path in ``CLOSURE=``, which
reaches the script as an environment variable like the rate limit. It
leaves out ``root=``: with no root device named, the kernel makes its
initial root a `tmpfs`_ (``init/do_mounts.c``), and NixOS's ``/`` is a
tmpfs anyway. It leaves out ``init=`` too, so the kernel runs ``/init``.
``CLOSURE=`` is the closure's path:

.. code-block:: console
   :class: cmd-host

   $ readlink ~/bootstrap/knix/guest/result

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu.d/10-closure.conf

    [QEMU]
   -initrd=/nix/store/...-initrd-linux-.../initrd
   -append=root=tmpfs console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST=1000 systemd.condition_first_boot=false quiet init=/nix/store/...-nixos-system-bootstrap-.../init
   +initrd=%h/bootstrap/knix/tiny.cpio
   +append=console=ttyS0 systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80 SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST=1000 systemd.condition_first_boot=false quiet CLOSURE=/nix/store/...-nixos-system-bootstrap-...

1.5 seconds. ``systemd-analyze`` in the guest now reports no initrd, and
counts the activation as the kernel's. ``prepare-root`` exists while
``system.nixos-init.enable`` is off, as it is in knix's closure; its
replacement does not run without an initrd yet.

More processors
---------------

The kernel above allows 64 processors, so more vCPUs reach systemd's
parallel start:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu

   -smp=2
   +smp=4

1.5 seconds again, a little less: the middle of ten boots is 1.49. That
is a fifth of where this section started. Eight vCPUs gain nothing more,
and ``mitigations=off`` and booting the uncompressed ``vmlinux`` gain
nothing measurable. `microvm`_, with this kernel and the store's cache,
took 5.7 to 6.0 seconds where q35 took 4.9 to 5.0, all of it in the
guest's userspace.

.. _Linux: https://www.kernel.org
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _systemd: https://systemd.io
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _nixpkgs: https://github.com/NixOS/nixpkgs
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _bootspec: https://nixos.org/manual/nixos/stable/#sec-bootspec
.. _knix: https://github.com/linux-kdevops/knix
.. _QEMU build system: https://www.qemu.org/docs/master/devel/build-system.html
.. _virtiofs: https://virtio-fs.gitlab.io
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _kfragments: https://github.com/linux-kdevops/kfragments
.. _qboot: https://github.com/bonzini/qboot
.. _Nix: https://nix.dev/manual/nix/latest/
.. _KVM: https://docs.kernel.org/virt/kvm/index.html
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _SeaBIOS: https://www.seabios.org/
.. _BusyBox: https://busybox.net
.. _OpenSSH: https://www.openssh.com
.. _tmpfs: https://docs.kernel.org/filesystems/tmpfs.html
.. _microvm: https://www.qemu.org/docs/master/system/i386/microvm.html
.. _kernel build documentation: https://docs.kernel.org/admin-guide/README.html
