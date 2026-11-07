.. SPDX-License-Identifier: copyleft-next-0.3.1

==========
Principles
==========

Six rules hold everywhere in qemu-system-generator, and a change that breaks
one is reworked rather than merged.

The backend section is a decoder
================================

A ``[QEMU]`` line is a QEMU option: its key is the option's name without the
dash, its value is passed on verbatim, and the lines keep the file's order.
The generator never translates, validates or reorders them, and holds no list
of QEMU options; QEMU is the validator and :cmd:`qemu-system-x86_64(1)` the
documentation. ``[virtiofsd]`` follows the same rule for `virtiofsd`_. A line
the generator reads to size or allow what the machine needs, such as ``m=``,
``machine=`` or ``device=``, is still passed on as written.

Zero tool at run time
=====================

The generator runs at :cmd:`systemctl` ``daemon-reload`` and exits. The units
it writes call only what the system ships: QEMU, virtiofsd, :cmd:`socat`,
:cmd:`varlinkctl` and :cmd:`ssh`. Nothing of the project stays between systemd
and the machine.

Names follow Podman's generator
===============================

The project's name stands where `Quadlet`_'s does: in
``QEMU_SYSTEM_GENERATOR_UNIT_DIRS``, the binary and the banner of the files it
writes. ``qemu-system`` stands where ``podman`` does, in the generator links
and in :doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>`. The units are
named after what they run, ``qemu-system@<name>.service`` and
``virtiofsd@<name>-<tag>``, and their directories after those prefixes.

The lifecycle is generated
==========================

What running a machine needs is generated, never asked of the file: its
directories, device access, control socket, registration with
:cmd:`systemd-machined.service(8)`, virtiofs wiring and locked memory.

The door stays open
===================

``src/plan.rs`` holds what a backend decodes a file into, in terms no launcher
owns; ``src/backend/`` decodes and ``src/launcher/`` emits. A second backend or
launcher is a new module, not a rewrite.

Simple steps
============

The commands of ``steps/`` do what the walk-throughs teach, and a step runs
the commands a person would type, plainly, with no code for a case that has
not happened. Nix provides and pins every tool, systemd runs and waits, and
`Dagu`_ orders, repeats and chooses; a step holds only what none of them can.
So a step does one thing to one object: it does not loop over work, retry,
wait, choose how to work or run another step, and it copies no logic another
step holds. KISS, DRY, YAGNI and SRP are the measure, and ``designs/steps.md``
in the source tree turns them into the rules a change to a step or a DAG is
reviewed against.

.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _Quadlet: https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html
.. _Dagu: https://github.com/dagucloud/dagu
