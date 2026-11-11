.. SPDX-License-Identifier: copyleft-next-0.3.1

==========================
Bootstrap with libguestfs
==========================

This guide changes `Debian`_'s testing cloud image with `libguestfs`_ and
boots it under your user's `systemd`_ manager, with `SeaBIOS`_ and the
image's `GRUB`_. :cmd:`virt-customize` starts a small appliance VM that
opens the disk and runs commands in it. `xfsprogs`_ and `xfstests`_ build
from their git trees inside the disk, and an upgrade of either rebuilds
only that one.

Prepare the host as :doc:`host` shows. nixpkgs builds libguestfs with
its appliance; run the commands below inside :cmd:`nix shell`:

.. code-block:: console
   :class: cmd-host

   $ nix shell nixpkgs#guestfs-tools

1. The image
============

A relative path in a ``.qemu`` file resolves in the machine's state
directory. Download the image there as :ref:`cloud-images` does, and
keep the download to start over from:

.. code-block:: console
   :class: cmd-host

   $ curl --location --remote-name --create-dirs --output-dir ~/.local/state/qemu-system/bootstrap-libguestfs \
       https://cloud.debian.org/images/cloud/forky/daily/latest/debian-14-genericcloud-amd64-daily.qcow2
   $ cd ~/.local/state/qemu-system/bootstrap-libguestfs
   $ curl --silent --location https://cloud.debian.org/images/cloud/forky/daily/latest/SHA512SUMS | sha512sum --check --ignore-missing
   debian-14-genericcloud-amd64-daily.qcow2: OK
   $ cp debian-14-genericcloud-amd64-daily.qcow2 bootstrap.qcow2

The image leaves its name, root's key and the SSH host keys to
`cloud-init`_ on first boot. Remove cloud-init and set all three in the
disk instead, with the key from :ref:`host-ssh`. ``--format`` applies to
the ``--add`` after it:

.. code-block:: console
   :class: cmd-host

   $ virt-customize --format qcow2 --add bootstrap.qcow2 \
       --uninstall cloud-init --hostname bootstrap-libguestfs \
       --ssh-inject root:file:$HOME/bootstrap/system/ssh/id_ed25519.pub \
       --run-command 'ssh-keygen -A'

2. The machine
==============

The machine boots the disk with ``snapshot=on``, so each boot starts from
the disk as customized and leaves it unchanged:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-libguestfs.qemu

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
   drive=file=bootstrap.qcow2,if=virtio,format=qcow2,snapshot=on
   device=vhost-vsock-pci-non-transitional,guest-cid=60

:ref:`Regenerate <host-regenerate>`, :ref:`start <host-start>`, and log
in over `vsock`_:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-libguestfs

:cmd:`virt-customize` fails to start on a disk that a running machine
has open, so stop the machine before each change.

3. xfsprogs and xfstests
========================

Clone both at a release:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/libguestfs
   $ cd ~/bootstrap/libguestfs
   $ git clone --depth 1 --branch v7.1.1 https://git.kernel.org/pub/scm/fs/xfs/xfsprogs-dev.git src/xfsprogs-dev
   $ git clone --depth 1 --branch v2026.09.22 https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git src/xfstests-dev

One script builds either in the disk: xfsprogs as Debian packages that
:cmd:`dpkg` installs over Debian's own, and xfstests into
``/var/lib/xfstests``:

.. code-block:: bash
   :caption: ~/bootstrap/libguestfs/build

   #!/bin/bash
   # Build xfsprogs or xfstests from /work and install it in the image.
   set -o errexit -o nounset -o pipefail
   build=$(mktemp --directory)
   case $1 in
   xfsprogs)
       cp --recursive /work/xfsprogs-dev "$build/xfsprogs"
       (cd "$build/xfsprogs" && DEB_BUILD_OPTIONS="parallel=$(nproc) nocheck" dpkg-buildpackage --build=binary --no-sign)
       dpkg --install "$build"/xfsprogs_*.deb
       ;;
   xfstests)
       cp --recursive /work/xfstests-dev "$build/xfstests"
       make --directory="$build/xfstests" --jobs="$(nproc)"
       rm --recursive --force /var/lib/xfstests
       make --directory="$build/xfstests" install
       ;;
   esac
   rm --recursive "$build"

Install the build packages of xfsprogs' ``debian/control`` and
xfstests' ``README``, and five of the tools the ``README`` lists for
running xfstests; a full run needs the rest. Copy in the trees and the
script, and run it once for each. ``--run`` would hand the script to
:cmd:`sh`, so ``--run-command`` calls :cmd:`bash`. The build runs in the
appliance, which ``--memsize`` and ``--smp`` size:

.. code-block:: console
   :class: cmd-host

   $ virt-customize --format qcow2 --add ~/.local/state/qemu-system/bootstrap-libguestfs/bootstrap.qcow2 \
       --memsize 4096 --smp "$(nproc)" \
       --install acl,attr,bc,gawk,psmisc,build-essential,debhelper,gettext,libacl1-dev,libaio-dev,libblkid-dev,libcap-dev,libdevmapper-dev,libedit-dev,libgdbm-compat-dev,libgdbm-dev,libicu-dev,libinih-dev,libtool-bin,liburcu-dev,liburing-dev,pkgconf,systemd-dev,uuid-dev,xfslibs-dev \
       --mkdir /work --copy-in src/xfsprogs-dev:/work --copy-in src/xfstests-dev:/work --copy-in build:/work \
       --run-command 'bash /work/build xfsprogs' --run-command 'bash /work/build xfstests' \
       --delete /work

From the copy of the download to here, the image takes 141 seconds. The
build packages stay in the disk, where the next build needs them.
:ref:`Start <host-start>` the machine:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-libguestfs xfs_repair -V
   xfs_repair version 7.1.1

It answers 7.2 seconds after the start, timed as :ref:`fast-measure`
does.

.. _libguestfs-upgrade:

4. An upgrade
=============

A copy with ``--reflink=always`` keeps the disk as it is, to boot or
start over from; on `XFS`_ or Btrfs it shares the data rather than
copying it. Move the tree to the next release, and rebuild only that
one:

.. code-block:: console
   :class: cmd-host

   $ cp --reflink=always ~/.local/state/qemu-system/bootstrap-libguestfs/bootstrap.qcow2 \
       ~/.local/state/qemu-system/bootstrap-libguestfs/xfsprogs-7.1.1.qcow2
   $ git -C src/xfsprogs-dev fetch --depth 1 origin tag v7.2.0
   $ git -C src/xfsprogs-dev checkout v7.2.0
   $ virt-customize --format qcow2 --add ~/.local/state/qemu-system/bootstrap-libguestfs/bootstrap.qcow2 \
       --memsize 4096 --smp "$(nproc)" \
       --mkdir /work --copy-in src/xfsprogs-dev:/work --copy-in build:/work \
       --run-command 'bash /work/build xfsprogs' --delete /work

:ref:`Start <host-start>` the machine, and it has xfsprogs 7.2.0. Each
time is the middle of three, timed as :ref:`fast-measure` does:

.. list-table::
   :header-rows: 1

   * - Upgrade
     - Build
     - Start to SSH
   * - xfsprogs v7.1.1 to v7.2.0
     - 75.7 s
     - 7.2 s
   * - xfstests v2026.09.22 to v2026.10.15
     - 30.4 s
     - 7.2 s

.. _Debian: https://www.debian.org
.. _libguestfs: https://libguestfs.org
.. _systemd: https://systemd.io
.. _SeaBIOS: https://www.seabios.org/
.. _GRUB: https://www.gnu.org/software/grub/manual/grub/grub.html
.. _xfsprogs: https://git.kernel.org/pub/scm/fs/xfs/xfsprogs-dev.git
.. _xfstests: https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git
.. _cloud-init: https://cloudinit.readthedocs.io
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _XFS: https://docs.kernel.org/admin-guide/xfs.html
