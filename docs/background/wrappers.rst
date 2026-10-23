.. SPDX-License-Identifier: copyleft-next-0.3.1

=============
QEMU wrappers
=============

Tools that start QEMU for you, each with its own configuration language
and defaults, grouped by what they are for. :doc:`qemu-problem` explains
why this project is not one more. A project's own description is quoted
where it publishes one.

The list began as the research behind `qemu-system-units`_ and its
precursor, and every link was checked in October 2026. It names only
tools that run QEMU; virtual machine monitors of their own, such as
Firecracker, Cloud Hypervisor or crosvm, are out of scope.

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
- `osandov-linux`_: Omar Sandoval's development scripts, which include
  his VM management.
- `linux-kernel-module-cheat`_: a Buildroot, QEMU and GDB environment
  for learning kernel and module development across architectures.
- `kworkflow`_ (``kw``): a kernel developer workflow tool whose
  ``kw vm`` mounts, unmounts and starts a QEMU image.
- `little-vm-helper`_: "tool for building and running VMs for
  development and testing", from Cilium.
- `kernel-development-flake`_: "Nix flake for linux kernel development",
  with commands that boot QEMU and attach GDB.
- `nixos-shell`_: "Spawns lightweight nixos vms in a shell."
- `xfstests-bld`_: its ``kvm-xfstests`` runs a file system test
  appliance under QEMU.
- `mkosi`_: builds OS images and boots them with ``mkosi qemu``;
  `mkosi-kernel`_ adds profiles for kernel development.

Testing and CI
==============

- `ktest.pl`_: Steven Rostedt's in-tree build, boot and bisect tool.
- `KUnit`_: ``kunit.py run --arch`` runs the in-tree unit tests under
  QEMU.
- `TuxRun`_: "a command line tool for testing Linux on the following
  virtual devices, using curated test suites", now part of KernelCI.
- `LAVA`_: Linaro's test lab system, whose QEMU device type is a Jinja2
  template.
- `syzkaller`_: "an unsupervised coverage-guided kernel fuzzer", which
  runs its pools of QEMU machines itself.
- `kAFL`_: "A fuzzer for full VM kernel/driver targets", on a modified
  QEMU.
- `BPF CI`_: the workflows that test every BPF patch in QEMU.
- `danobi/vmtest`_, `hugelgupf/vmtest`_ and `anatol/vmtest`_: three
  unrelated tools named vmtest that run tests in QEMU machines.
- `kirk`_: "the official executor of Linux Test Project", which can drive
  a QEMU guest over a serial channel.
- `kdevops`_: kernel testing infrastructure that provisions its guests
  through libvirt.
- `lkp-tests`_: the 0-Day test robot's suite, whose ``lkp qemu`` runs a
  job in a QEMU virtual machine.
- `cijoe`_: "tools for systems development and testing", with QEMU
  support.
- `Buildroot`_: its QEMU configurations generate a ``start-qemu.sh``.
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
- `virt-manager`_: "Desktop tool for managing virtual machines via
  libvirt."
- `GNOME Boxes`_: GNOME's virtual machine application, on libvirt.
- `Vagrant`_ with `vagrant-libvirt`_ or `vagrant-qemu`_: Vagrantfiles,
  through libvirt or straight to QEMU.
- `Lima`_: YAML-defined Linux machines with file sharing and port
  forwarding.
- `Multipass`_: "orchestrates virtual Ubuntu instances."
- `Incus`_: "Powerful system container and virtual machine manager",
  whose virtual machines are QEMU.
- `virter`_: "simple creation and cloning of virtual machines based on
  libvirt."
- `kvmrun`_: "A qemu-kvm virtual machine manager written in Go."
- `Proxmox VE`_: a virtualization platform that runs QEMU through its
  own ``qm`` tooling.
- `Packer`_: its QEMU plugin builds machine images.
- `terraform-provider-libvirt`_: Terraform resources for libvirt.
- `qemu-vm.nix`_: NixOS's typed options that compile to a QEMU start
  script, used by the NixOS tests.
- `microvm.nix`_: "NixOS MicroVMs", with QEMU among its hypervisors.
- `Kata Containers`_: containers in lightweight virtual machines, QEMU
  among its hypervisors.
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
- The ArchWiki's `QEMU`_ page, which shows QEMU run as a systemd service.

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
.. _linux-kernel-module-cheat: https://github.com/cirosantilli/linux-kernel-module-cheat
.. _kworkflow: https://github.com/kworkflow/kworkflow
.. _little-vm-helper: https://github.com/cilium/little-vm-helper
.. _kernel-development-flake: https://github.com/jordanisaacs/kernel-development-flake
.. _nixos-shell: https://github.com/Mic92/nixos-shell
.. _xfstests-bld: https://github.com/tytso/xfstests-bld
.. _mkosi: https://github.com/systemd/mkosi
.. _mkosi-kernel: https://github.com/DaanDeMeyer/mkosi-kernel
.. _ktest.pl: https://github.com/torvalds/linux/blob/master/tools/testing/ktest/ktest.pl
.. _KUnit: https://docs.kernel.org/dev-tools/kunit/run_wrapper.html
.. _TuxRun: https://github.com/kernelci/tuxrun
.. _LAVA: https://gitlab.com/lava/lava
.. _syzkaller: https://github.com/google/syzkaller
.. _kAFL: https://github.com/IntelLabs/kAFL
.. _BPF CI: https://github.com/kernel-patches/vmtest
.. _danobi/vmtest: https://github.com/danobi/vmtest
.. _hugelgupf/vmtest: https://github.com/hugelgupf/vmtest
.. _anatol/vmtest: https://github.com/anatol/vmtest
.. _kirk: https://github.com/linux-test-project/kirk
.. _kdevops: https://github.com/linux-kdevops/kdevops
.. _lkp-tests: https://github.com/intel/lkp-tests
.. _cijoe: https://github.com/refenv/cijoe
.. _Buildroot: https://buildroot.org
.. _runqemu: https://git.openembedded.org/openembedded-core/tree/scripts/runqemu
.. _qemu-runner: https://github.com/Novakov/qemu-runner
.. _qemu-launcher: https://github.com/software-artificer/qemu-launcher
.. _qemuconf: https://github.com/Gottox/qemuconf
.. _pemu: https://github.com/haishanh/pemu
.. _pyqvvm: https://github.com/irtimmer/pyqvvm
.. _quickemu: https://github.com/quickemu-project/quickemu
.. _libvirt: https://libvirt.org
.. _virt-manager: https://github.com/virt-manager/virt-manager
.. _GNOME Boxes: https://gitlab.gnome.org/GNOME/gnome-boxes
.. _Vagrant: https://developer.hashicorp.com/vagrant
.. _vagrant-libvirt: https://github.com/vagrant-libvirt/vagrant-libvirt
.. _vagrant-qemu: https://github.com/ppggff/vagrant-qemu
.. _Lima: https://lima-vm.io
.. _Multipass: https://github.com/canonical/multipass
.. _Incus: https://github.com/lxc/incus
.. _virter: https://github.com/LINBIT/virter
.. _kvmrun: https://github.com/kvmrun/kvmrun
.. _Proxmox VE: https://www.proxmox.com/en/products/proxmox-virtual-environment/overview
.. _Packer: https://developer.hashicorp.com/packer/integrations/hashicorp/qemu
.. _terraform-provider-libvirt: https://github.com/dmacvicar/terraform-provider-libvirt
.. _qemu-vm.nix: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/virtualisation/qemu-vm.nix
.. _microvm.nix: https://github.com/microvm-nix/microvm.nix
.. _Kata Containers: https://github.com/kata-containers/kata-containers
.. _systemd-vmspawn: https://www.freedesktop.org/software/systemd/man/latest/systemd-vmspawn.html
.. _kvm-systemd: https://github.com/rafaelmartins/kvm-systemd
.. _qemu-kvm-systemd-service: https://codeberg.org/eaon/qemu-kvm-systemd-service
.. _virtctl: https://github.com/dehesselle/virtctl
.. _QEMU: https://wiki.archlinux.org/title/QEMU
