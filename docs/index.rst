.. SPDX-License-Identifier: copyleft-next-0.3.1

=====================
qemu-system-generator
=====================

qemu-system-generator is a `systemd`_ generator
(:cmd:`systemd.generator(7)`) for :key:`.qemu` files: one file per virtual
machine, in systemd unit-file syntax, whose :key:`[QEMU]` section is
`QEMU`_'s own command line passed through line by line. The generator runs
at boot and on :cmd:`systemctl daemon-reload <systemctl>`, writes the units
that run the machine and the `virtiofsd`_ daemons it shares directories
through, then exits.

:doc:`background/qemu-problem` explains why: QEMU leaves its lifecycle to
whoever starts it, every wrapper that takes the job on brings its own
configuration language, and this one maps the file to QEMU's command line
one to one.

The format is specified in :doc:`man/qemu-systemd.unit.5` and the program
in :doc:`man/qemu-system-generator.8`. :doc:`bootstrap/index` boots a
first guest without root, in five ways, one feature per step.

Community
=========

- Source and issues:
  `GitHub <https://github.com/linux-kdevops/qemu-system-generator>`__.
- Chat: `Discord <https://bit.ly/linux-kdevops-chat>`__, or ``#kdevops`` on
  `OFTC IRC <https://webchat.oftc.net/?channels=kdevops>`__.
- Mailing list:
  `kdevops@lists.linux.dev <mailto:kdevops@lists.linux.dev>`__.

These are linked from the icons in the top navigation bar too.

.. toctree::
   :maxdepth: 1
   :caption: Background

   background/qemu-problem
   background/wrappers

.. toctree::
   :maxdepth: 1
   :caption: Walk-throughs

   bootstrap/index

.. toctree::
   :maxdepth: 1
   :caption: Manual pages

   man/qemu-systemd.unit.5
   man/qemu-system-generator.8

.. toctree::
   :maxdepth: 1
   :caption: Contributing

   writing

.. _systemd: https://systemd.io
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
