.. SPDX-License-Identifier: copyleft-next-0.3.1

==========
Motivation
==========

Running a virtual machine under `QEMU`_ is two jobs. One is the command
line: the machine type, memory, disks, devices, network and kernel. The
other is the lifecycle: starting and stopping the machine, ordering it
against what it needs, collecting its output, limiting its resources and
running the daemons that serve it. QEMU does the first, as a command line
only, and leaves the second to whoever starts it.

Dozens of tools fill the gap, each filling both halves in its own
way; :doc:`wrappers` lists them. This project fills it without another
vocabulary: the command line stays QEMU's own, one option per line, and
`systemd`_ runs the lifecycle, through a generator modelled on `Podman`_'s
`Quadlet`_ (:ref:`below <from-quadlet>`).

What QEMU does not do
=====================

Configuration files
-------------------

``-readconfig`` reads QEMU's option groups only. A ``[machine]`` section
can carry ``kernel``, ``initrd`` and ``append``, since QEMU parses those
three options into machine properties, but an option QEMU keeps in a
variable of its own has no section at all:

.. code-block:: ini
   :caption: cpu.cfg

   [cpu]
     model = "host"

.. code-block:: console
   :class: cmd-host

   $ qemu-system-x86_64 -readconfig cpu.cfg
   qemu-system-x86_64: -readconfig cpu.cfg: There is no option group 'cpu'

QEMU 11.1.2 answers the same for ``[serial]`` and ``[nographic]``. So
``-cpu``, ``-serial`` and ``-nographic``, among others, stay on the command
line, and no file alone describes a machine.

Stopping the guest
------------------

QEMU treats ``SIGTERM``, the signal systemd stops a service with, as a
host-side kill: ``qemu_system_killed()`` in
:qemu:`system/runstate.c <blob/master/system/runstate.c>` sets
``force_shutdown`` and QEMU exits without pressing the guest's `ACPI`_ power
button. The guest never shuts down. QEMU issue `#148`_, "Please solve
graceful (ACPI) poweroff issue, using signals, most importantly SIGTERM",
has been open since May 2021.

Readiness
---------

QEMU does not tell systemd when it is ready, so a QEMU service is
:cmd:`Type=simple <Type=>` rather than ``Type=notify``. Michael Tokarev
posted `[PATCH] implement systemd startup notification for qemu-system`_ on
17 December 2025, and QEMU's master branch, as of commit
:qemu:`bddb785e79 <commit/bddb785e79>` of 16 October 2026, has no
:cmd:`sd_notify <sd_notify(3)>` call.

Integration with the host
-------------------------

systemd documents what a VM manager does on the host:
`Writing VM and Container Managers`_ asks it to register each machine with
:cmd:`systemd-machined` and place it in :cmd:`machine.slice`, and
`The VM Interface`_ describes what the guest side can expect, such as
credentials over `SMBIOS`_ and readiness over :cmd:`AF_VSOCK <vsock(7)>`.
QEMU does none of this itself, and ``systemd-machined`` cannot report a
virtual machine's OS or addresses: in systemd v262,
``machine_get_os_release()`` and ``machine_get_addresses()`` in
:gh:`src/machine/machine.c <systemd/systemd/blob/v262/src/machine/machine.c>`
return ``-EOPNOTSUPP`` for any machine that is neither the host nor a
container.

Existing wrappers
=================

A wrapper answers both halves at once, so its configuration language
carries its author's choices. Two examples, from their sources:

- `virtme-ng`_ boots the `microvm`_ machine type whenever it can, as
  ``-M microvm,accel=kvm,pcie=on,rtc=on``
  (:gh:`virtme/architectures.py
  <arighi/virtme-ng/blob/main/virtme/architectures.py>`),
  unless told ``--disable-microvm``.
- `vmctl`_'s q35 base configuration defaults to 4 CPUs, 8 GiB of memory,
  ``-nodefaults``, ``kernel-irqchip=split`` and a virtual Intel IOMMU,
  each a shell variable with its own name (``GUEST_SMP``,
  ``GUEST_MEMORY``, ``GUEST_VIOMMU``).

The choices are reasonable. Each is still a name to learn and a default
to find, and a machine described for one wrapper does not move to
another.

qemu-system-units
-----------------

`qemu-system-units`_, the template set this project grew out of, set out
with "no new daemon, no new wrapper": `Jinja2`_ templates render systemd
units that run QEMU. It still has a vocabulary. Its variables are named
after concepts rather than options (``image`` for ``-drive``, ``cpus`` for
``-smp``, ``ssh_port`` for a ``hostfwd=`` rule, ``autostart: false`` for
``-S``), and its templates fix part of the command line: every virtio device
is non-transitional, a ``virtio-rng`` device is always present,
``-nographic`` and ``-serial mon:stdio`` are not configurable, networking is
``-nic user`` only, and `virtiofsd`_ always gets ``--xattr`` and
``--no-announce-submounts``. Its
:gh:`docs/design-decisions.md
<linux-kdevops/qemu-system-units/blob/main/docs/design-decisions.md>`
lists every such choice with the reason for it. The choices are documented,
but the templates still turned the command line into another dialect.

This project
============

A :key:`.qemu` file is in systemd's unit-file syntax, and its :key:`[QEMU]`
section maps to QEMU's command line one to one: the key is the option
name without its dash, the value is the option's argument, verbatim, and
the order is the file's. :key:`[virtiofsd]` does the same for virtiofsd.
The generator holds no list of options and translates, validates or
reorders none. QEMU is the validator and its manual the documentation.
:doc:`/man/qemu-systemd.unit.5` specifies the format.

The lifecycle is systemd's. The generator writes the units that start,
stop, order and confine the machine. The next section lists everything
it adds, so every exception to the one-to-one mapping is stated.

.. _from-quadlet:

Comparison with Quadlet
-----------------------

Quadlet does for Podman's containers what this project does for QEMU: its
generator, ``podman-system-generator``, turns ``.container`` files and the
other types it reads into services at every ``daemon-reload``
(:cmd:`podman-systemd.unit(5)`). This project delivers its units the same
way. The search path is Quadlet's with ``qemu/systemd/`` in place of
``containers/systemd/``. Drop-ins work as Quadlet's do, including the
type directory and the dash-prefix directories that let a family of
machines share lines. ``[Install]`` becomes symlinks at
generation time, since a generated unit cannot be enabled afterwards.
``--dryrun`` prints each file after a ``---path---`` line, and an
environment variable replaces the search path: ``QUADLET_UNIT_DIRS``
there, ``QEMU_SYSTEMD_UNIT_DIRS`` here. The manual is named after
Quadlet's too: ``qemu-systemd.unit(5)`` for ``podman-systemd.unit(5)``.

The vocabulary is not copied. A ``[Container]`` key is Quadlet's own and
a table translates it: ``PublishPort=`` becomes ``--publish`` and
``Timezone=`` becomes ``--tz`` (:gh:`pkg/systemd/quadlet/quadlet.go
<podman-container-tools/podman/blob/main/pkg/systemd/quadlet/quadlet.go>`).
``PodmanArgs=`` passes what the table lacks, and its manual recommends
against it. Here that escape hatch is the whole ``[QEMU]`` section.
Templates such as ``foo@.container`` have no counterpart either: a
``.qemu`` file's unit is already an instance of ``qemu-system@.service``,
so a family shares its lines through drop-ins.

Generated settings
------------------

None of the following touches a device, a disk, the network, the display
or the firmware; those are the file's. Each is a lifecycle need, and the
generated lines are in :doc:`/man/qemu-systemd.unit.5`'s GENERATED UNITS.

- ``Type=simple``, since QEMU does not notify readiness (above).
- :cmd:`KillSignal=SIGCONT <KillSignal=>`,
  :cmd:`KillMode=mixed <KillMode=>`,
  :cmd:`TimeoutStopSec=2min <TimeoutStopSec=>` and an :cmd:`ExecStop=` that
  sends ``system_powerdown`` over `QMP`_, preceded by a guest-side
  :cmd:`systemctl poweroff <systemctl>` over SSH when :key:`SshPrivateKey=`
  is set. ``SIGTERM`` would skip the guest's shutdown (above); ``SIGCONT``
  is a signal QEMU ignores, so systemd waits for the guest to power off.
  ``KillMode=mixed`` and the two-minute timeout are what systemd's own
  :gh:`systemd-vmspawn@.service
  <systemd/systemd/blob/v262/units/systemd-vmspawn%40.service.in>`
  uses.
- ``-qmp`` on a socket in the runtime directory, which the stop needs. A
  file's own ``-qmp`` is still passed on; QEMU serves several.
- :cmd:`Slice=machine.slice <Slice=>`, :cmd:`PartOf=` and
  :cmd:`Before=machines.target <Before=>`, the placement systemd's VM
  manager guide asks for, and an :cmd:`ExecStartPost=` that registers the
  machine with ``systemd-machined``, best effort.
- :cmd:`DevicePolicy=closed <DevicePolicy=>` with a :cmd:`DeviceAllow=`
  list, as ``systemd-vmspawn@.service`` does. A VFIO device adds its device
  nodes to the list and lifts :cmd:`LimitMEMLOCK=`, since it pins guest
  memory.
- :cmd:`StateDirectory=`, :cmd:`RuntimeDirectory=` and
  :cmd:`WorkingDirectory=`, so a relative path resolves in the machine's own
  state directory.
- For each ``[virtiofsd]`` section, the `vhost-user`_ wiring: a
  ``-chardev``, a ``vhost-user-fs`` device and, once, a shared
  ``memory-backend-memfd`` set as the machine's memory, which vhost-user
  requires, unless the file names its own backend. The daemon runs
  socket-activated with ``--fd=3`` and stops when its machine no longer
  needs it.

Every scalar :cmd:`[Service] <systemd.service(5)>` setting in the file
overrides the generated one, and an empty ``ExecStop=`` clears the stop
commands. The file states everything else a wrapper would decide.

Limitations
-----------

A decoder that passes QEMU's options through cannot add what QEMU lacks:
the stop still goes through QMP until ``SIGTERM`` powers the guest down,
the service stays ``Type=simple`` until QEMU notifies readiness, and
:cmd:`machinectl` shows no OS or address for a virtual machine until
``systemd-machined`` can learn them. Those are upstream work, in QEMU and
in systemd.

.. toctree::
   :hidden:

   wrappers

.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _systemd: https://systemd.io
.. _Podman: https://podman.io
.. _Quadlet: https://github.com/podman-container-tools/podman/tree/main/pkg/systemd/quadlet
.. _ACPI: https://uefi.org/specifications
.. _SMBIOS: https://www.dmtf.org/standards/smbios
.. _#148: https://gitlab.com/qemu-project/qemu/-/issues/148
.. _[PATCH] implement systemd startup notification for qemu-system: https://lore.kernel.org/all/20251217131909.3713900-1-mjt@tls.msk.ru/
.. _Writing VM and Container Managers: https://systemd.io/WRITING_VM_AND_CONTAINER_MANAGERS/
.. _The VM Interface: https://systemd.io/VM_INTERFACE/
.. _virtme-ng: https://github.com/arighi/virtme-ng
.. _vmctl: https://github.com/SamsungDS/vmctl
.. _microvm: https://www.qemu.org/docs/master/system/i386/microvm.html
.. _Jinja2: https://jinja.palletsprojects.com/
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _QMP: https://www.qemu.org/docs/master/interop/qmp-spec.html
.. _vhost-user: https://www.qemu.org/docs/master/interop/vhost-user.html
.. _qemu-system-units: https://github.com/linux-kdevops/qemu-system-units
