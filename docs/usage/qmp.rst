.. SPDX-License-Identifier: copyleft-next-0.3.1

===============
QMP and console
===============

`systemd`_ sees a machine as a process; `QEMU`_ itself knows whether the guest
runs, is paused or is reset. `QMP`_, the QEMU Machine Protocol, lets you
ask it, and the generated unit gives every machine a QMP socket in its
runtime directory, ``$XDG_RUNTIME_DIR/qemu-system/name/qmp.sock``, which
the stop also uses (:doc:`qemu-systemd.unit(5)
</man/qemu-systemd.unit.5>`).

.. _usage-qmp:

QMP
===

QMP is one JSON object per line. QEMU greets each client, and answers
nothing until the client sends ``qmp_capabilities``. :cmd:`socat` sends
the commands and prints the replies:

.. code-block:: console
   :class: cmd-host

   $ printf '%s\n' '{"execute":"qmp_capabilities"}' '{"execute":"query-status"}' |
       socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/qemu-system/bootstrap/qmp.sock
   {"QMP": {"version": {"qemu": {"micro": 2, "minor": 1, "major": 11}, "package": "v11.1.2"}, "capabilities": ["oob"]}}
   {"return": {}}
   {"return": {"status": "running", "running": true}}

Replace ``query-status`` with another command of the `QMP reference`_:

``stop``, ``cont``
   Pause the guest's processors and resume them. QEMU keeps running and
   answering, unlike after ``systemctl --user freeze``
   (:ref:`usage-pause`).
``system_reset``
   Reset the machine, as the reset button would. The guest reboots
   without shutting down; the service and its process stay.
``system_powerdown``
   Press the power button, which is what a stop does after the SSH
   poweroff.
``human-monitor-command``
   Run a command of the human monitor, the ``(qemu)`` prompt, such as
   ``info status``:

   .. code-block:: json

      {"execute":"human-monitor-command","arguments":{"command-line":"info status"}}

``socat - UNIX-CONNECT:...`` alone opens an interactive session: type
``{"execute":"qmp_capabilities"}``, then one command per line, and
Ctrl-D to leave. QEMU serves one client at a time and makes the others
wait, the stop's power button included, so do not keep a session open
while you stop the machine.

The ``mon:`` in ``serial=mon:stdio`` multiplexes the human monitor onto
the serial console. The service's standard input is ``/dev/null``,
though, so nothing can be typed into it there. Use QMP.

Console
========

The serial console goes to the journal (:ref:`usage-console-log`),
which you read but cannot type into. A virtio console on a socket of its
own gives a login prompt to attach to with ``socat``, and the kernel
guide adds one (:ref:`direct-console`).

.. _QMP: https://www.qemu.org/docs/master/interop/qmp-spec.html
.. _QMP reference: https://www.qemu.org/docs/master/interop/qemu-qmp-ref.html
.. _systemd: https://systemd.io
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
