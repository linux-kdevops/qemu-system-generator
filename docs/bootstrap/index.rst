.. SPDX-License-Identifier: copyleft-next-0.3.1

=========
Bootstrap
=========

Each guide below boots a guest under your user's `systemd`_ manager from
:key:`.qemu` files, without root, and stands on its own: it starts from an
empty host and builds its file up one feature per step. They differ in
how the guest boots, which decides what a change to the guest costs.

.. list-table::
   :header-rows: 1
   :widths: 18 22 32 28

   * - Guide
     - Firmware
     - What the guest boots
     - After a change to the guest
   * - :doc:`disk`
     - SeaBIOS and GRUB, then OVMF and systemd-boot
     - A NixOS disk image that nixpkgs builds
     - Rebuild the whole image
   * - :doc:`uki`
     - OVMF
     - A NixOS UKI shared over virtiofs, with the host's store
     - Rebuild the UKI and restart
   * - :doc:`kernel`
     - None: QEMU loads the kernel
     - The kernel and initrd of a NixOS closure, with the host's store
     - Rebuild and rewrite one drop-in line
   * - :doc:`knix`
     - None: QEMU loads the kernel
     - Your own kernel, a knix closure and your own QEMU
     - ``make`` the kernel and restart
   * - :doc:`cloud`
     - SeaBIOS and the image's GRUB
     - Debian, Fedora and Ubuntu cloud images, set up by cloud-init
     - Change it inside the guest, with its package manager

:doc:`disk` is the closest to a physical machine. :doc:`kernel` covers
the most of the format: `vsock`_, SSH poweroff, consoles, drop-ins, systemd
settings and shares, which work the same way in the others, and ends on
`QEMU`_'s :ref:`microvm machine type <kernel-microvm>`.

Everything you write or build lives in ``~/bootstrap``: a directory per
guide, the SSH key and configuration in ``ssh/``, and the directory the
guests share in ``share/``. What a machine writes lives where its
generated unit's :cmd:`StateDirectory=` puts it, which is also where a
relative path in its ``.qemu`` file resolves:
``~/.local/state/qemu-system/<name>/`` under a user manager,
``/var/lib/qemu-system/<name>/`` under the system manager
(:cmd:`systemd.exec(5)`). The `NixOS`_ guests boot read-only store paths and
keep nothing there; the cloud guests keep their disks there.

The NixOS guides and the `knix`_ guide all write the machine ``bootstrap``,
in ``~/.config/qemu/systemd/bootstrap.qemu``; before following a second
one, remove that file and ``bootstrap.qemu.d/``. The cloud guide writes
``cloud-debian``, ``cloud-fedora`` and ``cloud-ubuntu`` beside it.

.. toctree::
   :maxdepth: 1

   disk
   uki
   kernel
   knix
   cloud

.. _systemd: https://systemd.io
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _knix: https://github.com/linux-kdevops/knix
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
