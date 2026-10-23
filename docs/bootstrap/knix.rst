.. SPDX-License-Identifier: copyleft-next-0.3.1

=======================================
Bootstrap with your own kernel and QEMU
=======================================

This guide boots a `NixOS`_ guest under your user's systemd manager on a
`Linux`_ kernel and a `QEMU`_ you build yourself. The system stays in the
host's ``/nix/store``, shared over `virtiofs`_, and so do the kernel's
modules. No step needs root.

The file format is specified in
:doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` and the generator in
:doc:`qemu-system-generator(8) </man/qemu-system-generator.8>`. The outputs
below are real, trimmed to the lines that matter, with the home directory
shown as ``/home/user``.

Why knix
========

A `NixOS`_ system from `nixpkgs`_ carries its kernel: the initrd holds that
kernel's modules, and ``/lib/modules`` in the running system is that
kernel's too. nixpkgs can build a kernel from your tree, but as a `Nix`_
derivation, which rebuilds from scratch after every change.

`knix`_ keeps the kernel out of the closure. Its ``imageless`` backend
builds a NixOS system with no kernel of its own and an `initrd`_ with no
modules, which expects the boot-critical drivers built in and mounts
``/lib/modules`` over virtiofs under the tag ``modules``. You build the
kernel in your tree with ``make`` as usual, and the same closure boots
every build. knix also provides the ``build-kernel`` and ``build-qemu``
shells, which bring the toolchains.

.. include:: _prepare.rst

The kernel
----------

Configure the release as a KVM guest with virtiofs built in, build it
and install its modules next to it, inside the ``build-kernel`` shell
(:cmd:`nix develop`):

.. code-block:: console
   :class: cmd-host

   $ git clone --depth 1 --branch v7.3 \
       https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git ~/nixos-bootstrap/linux
   $ cd ~/nixos-bootstrap/linux
   $ K="nix develop github:linux-kdevops/knix#build-kernel --command"
   $ B=$HOME/nixos-bootstrap/linux-build
   $ $K make O=$B defconfig kvm_guest.config
   $ $K scripts/config --file $B/.config \
       --enable FUSE_FS --enable VIRTIO_FS \
       --enable VSOCKETS --enable VIRTIO_VSOCKETS
   $ $K make O=$B olddefconfig
   $ $K make O=$B --jobs=$(nproc)
   $ $K make O=$B INSTALL_MOD_PATH=$HOME/nixos-bootstrap/linux-install modules_install

``kvm_guest.config`` turns on the virtio drivers; the
`kernel build documentation`_ covers ``O=`` and the configuration targets.
The kernel is ``~/nixos-bootstrap/linux-build/arch/x86/boot/bzImage``, and
the modules are under ``~/nixos-bootstrap/linux-install/lib/modules/``.

QEMU
----

Build the release inside the ``build-qemu`` shell and install it in the
workspace (`QEMU build system`_):

.. code-block:: console
   :class: cmd-host

   $ git clone --depth 1 --branch v11.1.2 \
       https://gitlab.com/qemu-project/qemu.git ~/nixos-bootstrap/qemu
   $ mkdir --parents ~/nixos-bootstrap/qemu-build
   $ cd ~/nixos-bootstrap/qemu-build
   $ Q="nix develop github:linux-kdevops/knix#build-qemu --command"
   $ $Q ~/nixos-bootstrap/qemu/configure --prefix=$HOME/nixos-bootstrap/qemu-install \
       --target-list=x86_64-softmmu --enable-kvm --enable-slirp --disable-docs
   $ $Q make --jobs=$(nproc) install

.. code-block:: console
   :class: cmd-host

   $ ~/nixos-bootstrap/qemu-install/bin/qemu-system-x86_64 --version
   QEMU emulator version 11.1.2 (v11.1.2)

The guest
---------

Start from knix's template (:cmd:`nix flake init`) and point its input at
knix upstream:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/nixos-bootstrap/knix
   $ cd ~/nixos-bootstrap/knix
   $ nix flake init --template github:linux-kdevops/knix

.. code-block:: diff
   :caption: ~/nixos-bootstrap/knix/flake.nix

   -    knix.url = "path:/path/to/knix";
   +    knix.url = "github:linux-kdevops/knix";

Name the guest, mount a share of your own at ``/mnt/host``, and paste the
public key from ``~/nixos-bootstrap/id_ed25519.pub`` into ``default.nix``:

.. code-block:: diff
   :caption: ~/nixos-bootstrap/knix/default.nix

        #   knix.nixosModules.testSuites.blktests
   +    knix.nixosModules.mounts.shares
      ];

   -  networking.hostName = "vm";
   +  knix.shares."/mnt/host".tag = "host";
   +
   +  networking.hostName = "bootstrap";

   ...
      # SSH keys for the root and unprivileged accounts.
   -  # users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 ..." ];
   -  # users.users.${config.knix.user.name}.openssh.authorizedKeys.keys = [ "ssh-ed25519 ..." ];
   +  users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
   +  users.users.${config.knix.user.name}.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];

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

.. include:: _generator.rst

.. _knix-kernel:

1. Your kernel
==============

Make a directory for the guest to share, and put a file in it:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/nixos-bootstrap/share
   $ echo "hello from the host" > ~/nixos-bootstrap/share/hello.txt

Create ``~/.config/qemu/systemd/bootstrap.qemu``. The kernel comes from
your build, and three shares bring the store, ``/lib/modules`` and the
directory above:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

   [VM]
   SshPrivateKey=%h/nixos-bootstrap/id_ed25519

   [QEMU]
   machine=q35
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio
   kernel=%h/nixos-bootstrap/linux-build/arch/x86/boot/bzImage
   nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
   device=vhost-vsock-pci-non-transitional,guest-cid=42

   [virtiofsd]
   shared-dir=/nix/store
   tag=store
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h/nixos-bootstrap/linux-install/lib/modules
   tag=modules
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h/nixos-bootstrap/share
   tag=host
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

``%h`` is your home directory, and ``%U`` and ``%G`` your user and group
ids, among the `specifiers`_ systemd expands in a unit. ``nic=`` forwards
a local port to the guest's SSH over QEMU's `user-mode network`_. The
`vsock`_ device gives the guest a context id, which the generated unit
registers with your user's :cmd:`systemd-machined.service(8)`; with
``SshPrivateKey=`` in ``[VM]``, stopping the machine first runs
``systemctl poweroff`` in the guest over vsock, and presses the power
button only if that fails.

Each ``[virtiofsd]`` section runs one `virtiofsd`_ daemon for one shared
directory. Its keys are the long options of ``virtiofsd --help`` without
the dashes. The generator runs the daemon from its own socket unit and
adds the QEMU lines that connect to it. ``sandbox=``, ``uid-map=`` and
``gid-map=`` let it run without root, with the guest's root mapped to
you.

The initrd and ``init=`` name one build of the closure, and every
``nix build`` makes a new one, so they go in a drop-in: the files in
``bootstrap.qemu.d/`` are appended to ``bootstrap.qemu``. :cmd:`jq` writes
it from ``boot.json``, and knix wants ``root=tmpfs`` on the command line:

.. code-block:: shell
   :class: cmd-host

   mkdir --parents ~/.config/qemu/systemd/bootstrap.qemu.d
   jq --raw-output '."org.nixos.bootspec.v1" |
       "[QEMU]\ninitrd=\(.initrd)\nappend=root=tmpfs console=ttyS0 init=\(.init)"' \
       ~/nixos-bootstrap/knix/result/boot.json \
       > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf

.. include:: _start.rst

Log in over vsock. :cmd:`systemd-ssh-proxy(1)` lets :cmd:`ssh` on the host
reach a registered machine by name, through the ``ssh`` configuration
that systemd ships as ``20-systemd-ssh-proxy.conf``; most distributions
link it into ``/etc/ssh/ssh_config.d/``. Without it, use
``-p 10022 root@127.0.0.1``:

.. code-block:: console
   :class: cmd-host

   $ ssh -i ~/nixos-bootstrap/id_ed25519 root@machine/bootstrap \
       'uname -r; findmnt --noheadings --output=TARGET,SOURCE,FSTYPE --types=virtiofs'
   7.3.0
   /nix/store   store   virtiofs
   └─/nix/store store   virtiofs
   /lib/modules modules virtiofs
   /mnt/host    host    virtiofs

After a kernel rebuild, ``make modules_install`` again and restart. After
a closure rebuild, rerun the ``jq`` command, regenerate and restart; the
path unit does not watch the drop-in directory.

.. _knix-qemu:

2. Your QEMU
============

``QemuBinary=`` replaces the QEMU the generator was built with. It becomes
the first word of ``ExecStart=``, which must be a literal absolute path,
so write your home directory out:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/bootstrap.qemu

    [VM]
   +QemuBinary=/home/user/nixos-bootstrap/qemu-install/bin/qemu-system-x86_64
    SshPrivateKey=%h/nixos-bootstrap/id_ed25519

Regenerate and restart, then check which binary runs:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@bootstrap.service
   $ readlink /proc/$(systemctl --user show --property=MainPID --value qemu-system@bootstrap.service)/exe
   /home/user/nixos-bootstrap/qemu-install/bin/qemu-system-x86_64

The guest boots in about 9 seconds and stops in 2.

.. _Linux: https://www.kernel.org
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _nixpkgs: https://github.com/NixOS/nixpkgs
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _bootspec: https://nixos.org/manual/nixos/stable/#sec-bootspec
.. _knix: https://github.com/linux-kdevops/knix
.. _kernel build documentation: https://docs.kernel.org/admin-guide/README.html
.. _QEMU build system: https://www.qemu.org/docs/master/devel/build-system.html
.. _virtiofs: https://virtio-fs.gitlab.io
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
