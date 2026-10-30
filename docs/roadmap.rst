.. SPDX-License-Identifier: copyleft-next-0.3.1

=======
Roadmap
=======

The intended work, grouped by theme and undated. Each card carries a
status (:bdg-primary:`planned`, :bdg-warning:`research` or
:bdg-info:`staging`), and the plan behind it lives under ``notes/plans/``
in the source tree.

Machines
========

.. grid:: 1 1 2 2
   :gutter: 3

   .. grid-item-card:: PCIe root ports on q35

      :bdg-warning:`research`

      Plug each virtio device into a ``pcie-root-port``, as QEMU's
      `PCI Express guidelines`_ recommend, instead of the root bus, where
      it is an integrated endpoint that cannot be hot-plugged. The guides
      would declare a port per device, and the generator one per
      virtiofs share; which chassis numbers the generator may take is
      open. It costs no measurable boot time, and it is what hotplug over
      the machine's QMP socket needs. Plan: ``notes/plans/pcie-root-ports.md``.

Guides
======

.. grid:: 1 1 2 2
   :gutter: 3

   .. grid-item-card:: The kernel as a Nix derivation

      :bdg-warning:`research`

      Measure the knix guide's ``make`` build against nixpkgs'
      ``linuxManualConfig`` on the same tree after a one-line change, and
      try a plain NixOS closure that carries that kernel, with no
      ``modules`` share. The derivation is cached and shareable; ``make``
      is incremental and keeps the tree a kernel developer works in. The
      result decides whether the guide teaches both. Plan:
      ``notes/plans/kernel-as-derivation.md``.

Generator
=========

.. grid:: 1 1 2 2
   :gutter: 3

   .. grid-item-card:: Watch the drop-in directories

      :bdg-warning:`research`

      The path unit watches only the directory that holds the ``.qemu``
      files, and inotify does not reach into its ``.qemu.d``
      subdirectories, so a drop-in change needs a manual regenerate
      (:ref:`host-regenerate`). The generator, which reads every drop-in
      directory anyway, would write a drop-in for its own path unit with
      a ``PathChanged=`` per directory. Plan:
      ``notes/plans/watch-drop-in-directories.md``.

Upstream
========

.. grid:: 1 1 2 2
   :gutter: 3

   .. grid-item-card:: microvm's initrd over the ACPI tables

      :bdg-info:`staging`

      Report to QEMU that microvm loads an initrd over its own ACPI tables
      when the initrd's size modulo 4096 passes the tables' offset in the
      top page of RAM: ``microvm_memory_init()`` reserves no room for
      them, where the PC machines reserve ``PC_FW_DATA``. The report is
      drafted; once filed, :ref:`direct-microvm` cites it beside its
      workaround. Plan: ``notes/plans/qemu-microvm-initrd-acpi.md``.

Steps and DAGs
==============

.. grid:: 1 1 2 2
   :gutter: 3

   .. grid-item-card:: knix steps per machine

      :bdg-primary:`planned`

      Give ``steps/knix/install`` and ``steps/knix/build`` a ``--name``
      and write one machine each, as the direct steps do, with the builds
      both machines share in a step of their own. Plan:
      ``notes/plans/knix-steps-per-machine.md``.

   .. grid-item-card:: Cover more in CI

      :bdg-primary:`planned`

      Add DAGs for what ``qemu-systemd.unit(5)`` specifies and no DAG
      reaches: the stop over SSH and its timeout, ``[Install]`` under the
      user manager, VFIO behind a vIOMMU, and two machines ordered by
      ``[Unit]`` lines. Then cover what kdevops' guests use and the DAGs
      do not: NVMe behind PCIe root ports, large blocks, virtio-blk and
      virtio-scsi, a bridge, CXL and a crashing guest. Until then, a
      change that breaks one of these shows first on a user's machine.
      Plan: ``notes/plans/ci-coverage.md``.

.. _PCI Express guidelines: https://gitlab.com/qemu-project/qemu/-/blob/master/docs/pcie.txt
