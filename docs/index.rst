.. SPDX-License-Identifier: copyleft-next-0.3.1

=====================
qemu-system-generator
=====================

qemu-system-generator is a systemd generator
(:cmd:`systemd.generator(7)`) for ``.qemu`` files: one file
per virtual machine, in systemd unit-file syntax, whose ``[QEMU]`` section
is QEMU's own command line passed through line by line. The generator runs
at boot and on ``systemctl daemon-reload``, writes the units that run the
machine and the `virtiofsd`_ daemons it shares directories through, then
exits.

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

.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
