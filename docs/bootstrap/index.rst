.. SPDX-License-Identifier: copyleft-next-0.3.1

=========
Bootstrap
=========

Each guide below boots a guest under your user's systemd manager from
``.qemu`` files, without root, and stands on its own: it starts from an
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

:doc:`disk` is the closest to a physical machine. :doc:`kernel` covers
the most of the format: vsock, SSH poweroff, consoles, drop-ins, systemd
settings and shares, which work the same way in the others.

All four share ``~/nixos-bootstrap`` and its SSH key, and write the
machine ``bootstrap``, in ``~/.config/qemu/systemd/bootstrap.qemu``; before
following a second one, remove that file and ``bootstrap.qemu.d/``.

.. toctree::
   :maxdepth: 1

   disk
   uki
   kernel
   knix
