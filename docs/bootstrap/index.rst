.. SPDX-License-Identifier: copyleft-next-0.3.1

=========
Bootstrap
=========

Each guide below boots a guest under your user's `systemd`_ manager from
:key:`.qemu` files. Each starts from a host prepared once as :doc:`host`
shows. They differ in how the guest boots, which decides what a change to
the guest costs.

.. list-table::
   :header-rows: 1
   :widths: 18 22 32 28

   * - Guide
     - Firmware
     - What the guest boots
     - After a change to the guest
   * - :doc:`disk`
     - `SeaBIOS`_ and `GRUB`_, then `OVMF`_ and `systemd-boot`_
     - A `NixOS`_ disk image that `nixpkgs`_ builds
     - Rebuild the whole image
   * - :doc:`uki`
     - OVMF
     - A NixOS `UKI`_ shared over `virtiofs`_, with the host's store
     - Rebuild the UKI and restart
   * - :doc:`direct`
     - None: `QEMU`_ loads the kernel
     - The kernel and `initrd`_ of a NixOS closure, with the host's store
     - Rebuild and rewrite one drop-in line
   * - :doc:`knix`
     - None: QEMU loads the kernel
     - Your own kernel, a `knix`_ closure and your own QEMU
     - :cmd:`make` the kernel and restart
   * - :doc:`cloud`
     - SeaBIOS and the image's GRUB
     - `Debian`_, `Fedora`_ and `Ubuntu`_ cloud images, set up by `cloud-init`_
     - Change it inside the guest, with its package manager

:doc:`disk` is the closest to a physical machine. :doc:`direct` covers most
of the format: `vsock`_, SSH poweroff, consoles, drop-ins, systemd
settings and shares, which work the same way in the others. It ends on
`QEMU`_'s :ref:`microvm machine type <direct-microvm>`.

Each NixOS guide ends with :doc:`a faster boot <fast>`, from the
defaults to the fastest configuration it can reach, one change at a time.

The file format is specified in
:doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` and the generator in
:doc:`qemu-system-generator(8) </man/qemu-system-generator.8>`.

What a machine writes lives where its
generated unit's :cmd:`StateDirectory=` puts it, which is also where a
relative path in its ``.qemu`` file resolves:
``~/.local/state/qemu-system/<name>/`` under a user manager,
``/var/lib/qemu-system/<name>/`` under the system manager
(:cmd:`systemd.exec(5)`). The `NixOS`_ guests boot read-only store paths and
keep nothing there; the cloud guests keep their disks there.

.. toctree::
   :maxdepth: 1

   host
   disk
   uki
   direct
   knix
   cloud
   home
   nixos
   network
   fast

.. _systemd: https://systemd.io
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _knix: https://github.com/linux-kdevops/knix
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _SeaBIOS: https://www.seabios.org/
.. _GRUB: https://www.gnu.org/software/grub/manual/grub/grub.html
.. _OVMF: https://github.com/tianocore/tianocore.github.io/wiki/OVMF
.. _systemd-boot: https://www.freedesktop.org/software/systemd/man/latest/systemd-boot.html
.. _nixpkgs: https://github.com/NixOS/nixpkgs
.. _UKI: https://uapi-group.org/specifications/specs/unified_kernel_image/
.. _virtiofs: https://virtio-fs.gitlab.io
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _Debian: https://www.debian.org
.. _Fedora: https://fedoraproject.org
.. _Ubuntu: https://ubuntu.com
.. _cloud-init: https://cloudinit.readthedocs.io
