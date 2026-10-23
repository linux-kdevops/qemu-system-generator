.. SPDX-License-Identifier: copyleft-next-0.3.1

Each line of :key:`[QEMU]` is one QEMU option with its dash removed, passed on
as written: ``m=2G`` is ``-m 2G``, and ``nographic=`` with an empty value is
the bare flag ``-nographic``. :cmd:`qemu-system-x86_64(1)` documents them all.
The file is named after the machine, and the unit after the file:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user start qemu-system@bootstrap.service
   $ journalctl --user-unit=qemu-system@bootstrap.service --follow

:cmd:`journalctl` shows the serial console.
