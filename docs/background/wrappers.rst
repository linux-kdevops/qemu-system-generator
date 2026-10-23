.. SPDX-License-Identifier: copyleft-next-0.3.1

=============
QEMU wrappers
=============

On this page, a QEMU wrapper is a tool that writes the QEMU command line
itself, from a configuration language or defaults of its own, so that you
can start a virtual machine without writing that command line. A tool
that reaches QEMU through a wrapper, or starts QEMU only as a step of
another job, is not one and is not listed. :doc:`qemu-problem` explains
why this project is not one more.

Entries are grouped by what the tool is for, and a project's own
description is quoted where it publishes one.

The list began as the research behind `qemu-system-units`_ and its
precursor, and every link was checked in October 2026.

Kernel development
==================

- `virtme-ng`_ (``vng``): "Quickly build and run kernels inside a
  virtualized snapshot of your live system." Boots ``microvm`` when it
  can and shares the host's file system over virtiofs.
- `virtme`_: "An easy way to virtualize the running system." The
  original, by Andy Lutomirski, which virtme-ng continues.
- `vmctl`_: "QEMU NVMe Testing Galore!" Shell configuration files for
  QEMU machines, aimed at NVMe testing.
- `run-kernel`_: boots a Linux kernel into a NixOS userland in QEMU,
  with the Nix store shared over virtiofs and no disk image.
- `eudyptula-boot`_: "Boot a Linux kernel in a VM without a dedicated
  root filesystem." One shell script, 9p and overlayfs.
- `Rapido`_: "Quickly test Linux kernel changes." Dracut-built
  initramfs images for storage and file-system testing.
- `vido`_: "wrap commands in throwaway virtual machines."
- `run_qemu`_: "A script to create bootable OS images, and run qemu with
  a locally built kernel." NUMA, NVDIMM and CXL topologies.
- `osandov-linux`_: Omar Sandoval's ``vm.py``, which creates and runs
  his development machines.
- `kworkflow`_ (``kw``): a kernel developer workflow tool whose
  ``kw vm`` starts a QEMU image with ``-smp 2 -m 1024`` unless its
  ``qemu_hw_options`` says otherwise.
- `little-vm-helper`_: "tool for building and running VMs for
  development and testing", from Cilium.
- `kernel-development-flake`_: "Nix flake for linux kernel development",
  with commands that boot QEMU and attach GDB.

Tests in a virtual machine
==========================

- `danobi/vmtest`_: runs a command in a QEMU machine described by a
  ``vmtest.toml``.
- `hugelgupf/vmtest`_: "a Go API for launching QEMU VMs."
- `anatol/vmtest`_: a Go library that "provides an easy way to setup a
  Virtual Machine for your integration tests".

Built images
============

- `mkosi`_: builds OS images, and ``mkosi vm`` boots them in QEMU from
  the settings of its ``[Runtime]`` section.
- `runqemu`_: the OpenEmbedded and Yocto script that boots a built image
  under QEMU.

Configuration file to command line
==================================

- `qemu-runner`_: "Create self-contained wrappers for QEMU command
  lines", from layered INI files.
- `qemu-launcher`_: runs QEMU machines "based on definitions stored in
  configuration files with support for vCPU pinning". Archived.
- `qemuconf`_: "simple qemu launcher with config file support".
- `pemu`_: "A QEMU/KVM command line wrapper", with INI sections that
  inherit a global one.
- `pyqvvm`_: "Python Qemu Virtual Machine Manager", YAML that mirrors the
  command line.
- `quickemu`_: "Quickly create and run optimised Windows, macOS and Linux
  virtual machines", from shell configuration files.

Virtual machine managers and platforms
======================================

- `libvirt`_: the daemon and API behind ``virsh`` and ``virt-install``,
  which translates its domain XML into QEMU command lines.
- `vagrant-qemu`_: a Vagrant provider that runs QEMU directly.
- `Lima`_: YAML-defined Linux machines with file sharing and port
  forwarding.
- `Multipass`_: "orchestrates virtual Ubuntu instances."
- `Incus`_: "Powerful system container and virtual machine manager",
  whose virtual machines are QEMU.
- `kvmrun`_: "A qemu-kvm virtual machine manager written in Go."
- `Proxmox VE`_: a virtualization platform that runs QEMU through its
  own ``qm`` tooling.
- `qemu-vm.nix`_: NixOS's typed options that compile to a QEMU start
  script, used by the NixOS tests.
- `microvm.nix`_: "NixOS MicroVMs", with QEMU among its hypervisors.
- `systemd-vmspawn`_: systemd's own, which starts a QEMU machine from an
  OS image or directory.

systemd units for QEMU
======================

- `qemu-system-units`_: Jinja2 templates rendered into systemd units, the
  predecessor of this project.
- `kvm-systemd`_: a ``kvm@.service`` with one configuration file per
  machine.
- `qemu-kvm-systemd-service`_: a ``qemu@.service`` with one configuration
  file per machine.
- `virtctl`_: machine services wanted by a ``hypervisor.target`` of its
  own.

.. _qemu-system-units: https://github.com/linux-kdevops/qemu-system-units
.. _virtme-ng: https://github.com/arighi/virtme-ng
.. _virtme: https://github.com/amluto/virtme
.. _vmctl: https://github.com/SamsungDS/vmctl
.. _run-kernel: https://github.com/metaspace/run-kernel
.. _eudyptula-boot: https://github.com/vincentbernat/eudyptula-boot
.. _Rapido: https://github.com/rapido-linux/rapido
.. _vido: https://github.com/brenns10/vido
.. _run_qemu: https://github.com/pmem/run_qemu
.. _osandov-linux: https://github.com/osandov/osandov-linux
.. _kworkflow: https://github.com/kworkflow/kworkflow
.. _little-vm-helper: https://github.com/cilium/little-vm-helper
.. _kernel-development-flake: https://github.com/jordanisaacs/kernel-development-flake
.. _mkosi: https://github.com/systemd/mkosi
.. _danobi/vmtest: https://github.com/danobi/vmtest
.. _hugelgupf/vmtest: https://github.com/hugelgupf/vmtest
.. _anatol/vmtest: https://github.com/anatol/vmtest
.. _runqemu: https://git.openembedded.org/openembedded-core/tree/scripts/runqemu
.. _qemu-runner: https://github.com/Novakov/qemu-runner
.. _qemu-launcher: https://github.com/software-artificer/qemu-launcher
.. _qemuconf: https://github.com/Gottox/qemuconf
.. _pemu: https://github.com/haishanh/pemu
.. _pyqvvm: https://github.com/irtimmer/pyqvvm
.. _quickemu: https://github.com/quickemu-project/quickemu
.. _libvirt: https://libvirt.org
.. _vagrant-qemu: https://github.com/ppggff/vagrant-qemu
.. _Lima: https://lima-vm.io
.. _Multipass: https://github.com/canonical/multipass
.. _Incus: https://github.com/lxc/incus
.. _kvmrun: https://github.com/kvmrun/kvmrun
.. _Proxmox VE: https://www.proxmox.com/en/products/proxmox-virtual-environment/overview
.. _qemu-vm.nix: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/virtualisation/qemu-vm.nix
.. _microvm.nix: https://github.com/microvm-nix/microvm.nix
.. _systemd-vmspawn: https://www.freedesktop.org/software/systemd/man/latest/systemd-vmspawn.html
.. _kvm-systemd: https://github.com/rafaelmartins/kvm-systemd
.. _qemu-kvm-systemd-service: https://codeberg.org/eaon/qemu-kvm-systemd-service
.. _virtctl: https://github.com/dehesselle/virtctl
