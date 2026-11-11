.. SPDX-License-Identifier: copyleft-next-0.3.1

=====================
Bootstrap with mkosi
=====================

This guide builds a `Debian`_ testing disk image with `mkosi`_ and boots it
under your user's `systemd`_ manager, with `OVMF`_, `systemd-boot`_ and
Debian's kernel. `xfsprogs`_ and `xfstests`_ build from their git trees
into the image, and an upgrade of either rebuilds the image.

Prepare the host as :doc:`host` shows.

The tools
=========

mkosi runs as your user. Before the image, it builds a tools tree: a
Debian root with Debian's own :cmd:`apt`, :cmd:`bootctl` and
:cmd:`systemd-repart`, in which every later step runs. It bootstraps that
tree with the host's :cmd:`apt`, :cmd:`dpkg` and :cmd:`gpgv`, which Nix
provides, and checks Debian's archive against Debian's keyring. Link the
keyring into the directory the image is built from:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/mkosi
   $ cd ~/bootstrap/mkosi
   $ nix build nixpkgs#debian-archive-keyring --out-link keyring

Run the commands below inside :cmd:`nix shell`, which puts mkosi and
those three tools on the ``PATH``:

.. code-block:: console
   :class: cmd-host

   $ nix shell nixpkgs#mkosi nixpkgs#apt nixpkgs#dpkg nixpkgs#gnupg

1. The image
============

``mkosi.conf`` describes the image (:cmd:`mkosi(1)`). ``Bootable=yes``
installs systemd-boot and a `UKI`_ of Debian's kernel in the EFI system
partition. The four firstboot settings answer what
:cmd:`systemd-firstboot(1)` would otherwise ask on the console at every
boot. ``SandboxTrees=`` puts the keyring where Debian's tools look for
it:

.. code-block:: ini
   :caption: ~/bootstrap/mkosi/mkosi.conf

   [Distribution]
   Distribution=debian
   Release=testing

   [Output]
   Output=bootstrap
   OutputDirectory=mkosi.output

   [Content]
   Bootable=yes
   KernelCommandLine=console=ttyS0
   Locale=C.UTF-8
   Keymap=us
   Timezone=UTC
   Hostname=bootstrap-mkosi
   Packages=
           linux-image-amd64
           systemd
           systemd-boot
           systemd-boot-efi
           udev
           dbus
           openssh-server

   [Build]
   ToolsTree=default
   ToolsTreePackages=udev
   SandboxTrees=keyring/share/keyrings:/usr/share/keyrings

Debian ships systemd-boot's EFI binary apart from the tool, in
``systemd-boot-efi``. The default tools tree leaves out ``udev``, and
without it nixpkgs' mkosi stops at ``systemd-hwdb not found``.

A file under ``mkosi.extra/`` is copied into the image at the same path.
Root takes the key from :ref:`host-ssh`:

.. code-block:: text
   :caption: ~/bootstrap/mkosi/mkosi.extra/root/.ssh/authorized_keys

   ssh-ed25519 AAAA... bootstrap

Build it:

.. code-block:: console
   :class: cmd-host

   $ mkosi build
   ...
   ‣ /home/user/bootstrap/mkosi/mkosi.output/bootstrap.raw size is 1003.3M, consumes 483.7M.

mkosi keeps the tools tree in ``mkosi.tools/`` for the builds after the
first.

2. The machine
==============

Link OVMF from nixpkgs, and give the machine the image with
``snapshot=on``, so each boot starts from the image as built and leaves
it unchanged:

.. code-block:: console
   :class: cmd-host

   $ nix build nixpkgs#OVMF.fd --out-link ovmf

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-mkosi.qemu

   [VM]
   SSHPrivateKey=%h/bootstrap/system/ssh/id_ed25519

   [QEMU]
   machine=q35
   accel=kvm
   cpu=host
   m=2G
   smp=4
   nographic=
   serial=mon:stdio
   drive=if=pflash,format=raw,readonly=on,file=%h/bootstrap/mkosi/ovmf-fd/FV/OVMF_CODE.fd
   drive=if=pflash,format=raw,snapshot=on,file=%h/bootstrap/mkosi/ovmf-fd/FV/OVMF_VARS.fd
   drive=file=%h/bootstrap/mkosi/mkosi.output/bootstrap.raw,if=virtio,format=raw,snapshot=on
   device=vhost-vsock-pci-non-transitional,guest-cid=59

In the guest, :cmd:`systemd-ssh-generator(8)` adds a socket for SSH on
`vsock`_, and :ref:`host-ssh` names the machine by its context id.
:ref:`Regenerate <host-regenerate>`, :ref:`start <host-start>`, and log
in:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-mkosi

3. xfsprogs and xfstests
========================

Clone both at a release, beside the configuration:

.. code-block:: console
   :class: cmd-host

   $ git clone --depth 1 --branch v7.1.1 https://git.kernel.org/pub/scm/fs/xfs/xfsprogs-dev.git src/xfsprogs-dev
   $ git clone --depth 1 --branch v2026.09.22 https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git src/xfstests-dev

mkosi builds them in the image (:cmd:`mkosi(1)`, "SCRIPTS"): it mounts
each ``BuildSources=`` tree under ``$SRCDIR`` and runs
``mkosi.build.chroot`` in an overlay of the image that adds
``BuildPackages=``, then drops the overlay. The build packages are those
of xfsprogs' ``debian/control`` and xfstests' ``README``. ``Packages=``
gains five of the tools the ``README`` lists for running xfstests; a full
run needs the rest. ``Incremental=yes`` keeps the image with its packages
installed in ``CacheDirectory=``, and a later build starts from there:

.. code-block:: diff
   :caption: ~/bootstrap/mkosi/mkosi.conf

   --- a/bootstrap/mkosi/mkosi.conf
   +++ b/bootstrap/mkosi/mkosi.conf
   @@ -21,8 +21,40 @@ Packages=
            udev
            dbus
            openssh-server
   +        acl
   +        attr
   +        bc
   +        gawk
   +        psmisc
   +BuildPackages=
   +        build-essential
   +        debhelper
   +        gettext
   +        libacl1-dev
   +        libaio-dev
   +        libblkid-dev
   +        libcap-dev
   +        libdevmapper-dev
   +        libedit-dev
   +        libgdbm-compat-dev
   +        libgdbm-dev
   +        libicu-dev
   +        libinih-dev
   +        libtool-bin
   +        liburcu-dev
   +        liburing-dev
   +        pkgconf
   +        systemd-dev
   +        uuid-dev
   +        xfslibs-dev
   +VolatilePackages=xfsprogs

    [Build]
   +BuildSources=
   +        src/xfsprogs-dev:xfsprogs-dev
   +        src/xfstests-dev:xfstests-dev
    ToolsTree=default
    ToolsTreePackages=udev
    SandboxTrees=keyring/share/keyrings:/usr/share/keyrings
   +Incremental=yes
   +CacheDirectory=mkosi.cache

The script builds xfsprogs as Debian packages, which it leaves in
``$PACKAGEDIR``; ``VolatilePackages=xfsprogs`` installs them from there,
over Debian's own. xfstests has no Debian package, and with ``DESTDIR``
set its install fails in its own libtool. The script installs it into
the overlay without ``DESTDIR`` and copies ``/var/lib/xfstests`` into
``$DESTDIR``, which mkosi merges into the image:

.. code-block:: bash
   :caption: ~/bootstrap/mkosi/mkosi.build.chroot

   #!/bin/bash
   set -o errexit -o nounset -o pipefail
   build=$(mktemp --directory)

   cp --recursive "$SRCDIR/xfsprogs-dev" "$build/xfsprogs"
   (cd "$build/xfsprogs" && DEB_BUILD_OPTIONS="parallel=$(nproc) nocheck" dpkg-buildpackage --build=binary --no-sign)
   cp "$build"/xfsprogs_*.deb "$PACKAGEDIR"

   cp --recursive "$SRCDIR/xfstests-dev" "$build/xfstests"
   make --directory="$build/xfstests" --jobs="$(nproc)"
   env --unset=DESTDIR make --directory="$build/xfstests" install
   mkdir --parents "$DESTDIR/var/lib"
   cp --archive /var/lib/xfstests "$DESTDIR/var/lib/"

Make it executable, and build again. ``mkosi build`` refuses to
overwrite an image, and ``--force`` rebuilds it:

.. code-block:: console
   :class: cmd-host

   $ chmod +x mkosi.build.chroot
   $ mkosi --force build

From nothing, with no tools tree and no packages downloaded yet, this
image takes 171 seconds to build, and 142 seconds with the tools tree
kept. ``--force`` given twice also drops the cache ``Incremental=``
keeps, and three times the downloaded packages.

:ref:`Restart <host-start>` the machine:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-mkosi xfs_repair -V
   xfs_repair version 7.1.1
   $ ssh bootstrap-mkosi ls /var/lib/xfstests
   check
   common
   ...

xfsprogs' ``xfs_healer@-.service`` fails on the ext4 root, so
:cmd:`systemctl is-system-running <systemctl>` reports ``degraded``.

.. _mkosi-upgrade:

4. An upgrade
=============

Move a tree to the next release, and build again:

.. code-block:: console
   :class: cmd-host

   $ git -C src/xfsprogs-dev fetch --depth 1 origin tag v7.2.0
   $ git -C src/xfsprogs-dev checkout v7.2.0
   $ mkosi --force build

The build starts from the cached image, runs the whole build script
again, xfstests included, and writes a new disk. :ref:`Restart
<host-start>`, and the guest has xfsprogs 7.2.0. Each time is the middle
of three, timed as :ref:`fast-measure` does:

.. list-table::
   :header-rows: 1

   * - Upgrade
     - Build
     - Start to SSH
   * - xfsprogs v7.1.1 to v7.2.0
     - 108.9 s
     - 7.7 s
   * - xfstests v2026.09.22 to v2026.10.15
     - 126.7 s
     - 8.5 s

.. _Debian: https://www.debian.org
.. _mkosi: https://github.com/systemd/mkosi
.. _systemd: https://systemd.io
.. _OVMF: https://github.com/tianocore/tianocore.github.io/wiki/OVMF
.. _systemd-boot: https://www.freedesktop.org/software/systemd/man/latest/systemd-boot.html
.. _xfsprogs: https://git.kernel.org/pub/scm/fs/xfs/xfsprogs-dev.git
.. _xfstests: https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git
.. _UKI: https://uapi-group.org/specifications/specs/unified_kernel_image/
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
