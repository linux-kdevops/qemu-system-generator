.. SPDX-License-Identifier: copyleft-next-0.3.1

========
Machines
========

Each machine the generator writes is an ordinary `systemd`_ service,
registered with :cmd:`machined <systemd-machined>` and given its own `QMP`_
socket. The tools that ship with systemd and `QEMU`_ drive it, and this
project adds none. The examples use the bootstrap guides' machine,
``bootstrap``, under your user manager.

.. list-table::
   :header-rows: 1
   :widths: 30 45 25

   * - To
     - Run
     - See
   * - list the running machines
     - ``machinectl --user list``
     - :ref:`usage-list`
   * - see one's state, memory and last lines
     - ``systemctl --user status qemu-system@bootstrap.service``
     - :ref:`usage-list`
   * - start, stop, restart
     - ``systemctl --user start qemu-system@bootstrap.service``
     - :ref:`usage-start`
   * - stop a guest that does not answer
     - ``systemctl --user kill qemu-system@bootstrap.service``
     - :ref:`usage-start`
   * - start machines at boot
     - ``loginctl enable-linger``
     - :ref:`usage-boot`
   * - pause and resume
     - ``systemctl --user freeze qemu-system@bootstrap.service``
     - :ref:`usage-pause`
   * - read the serial console
     - ``journalctl --user-unit=qemu-system@bootstrap.service``
     - :ref:`usage-console-log`
   * - read the guest's journal
     - ``ssh bootstrap journalctl --boot``
     - :ref:`usage-guest-journal`
   * - keep every guest's journal on the host
     - ``journalctl --directory ~/bootstrap/journal``
     - :ref:`usage-forward`
   * - log in
     - ``ssh machine/bootstrap``
     - :ref:`usage-ssh`
   * - ask machined about a machine
     - ``varlinkctl call ... io.systemd.Machine.List``
     - :ref:`usage-varlink`
   * - talk to QEMU itself
     - ``socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/qemu-system/bootstrap/qmp.sock``
     - :ref:`usage-qmp`

.. toctree::
   :maxdepth: 1

   lifecycle
   journal
   machined
   qmp

.. _systemd: https://systemd.io
.. _QMP: https://www.qemu.org/docs/master/interop/qmp-spec.html
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
