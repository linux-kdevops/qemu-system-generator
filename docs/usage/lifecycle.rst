.. SPDX-License-Identifier: copyleft-next-0.3.1

=========
Lifecycle
=========

Every machine is the service ``qemu-system@name.service`` of your user
manager, so :cmd:`systemctl` ``--user`` starts, stops and inspects it as
it does any other service. The unit's lines are in
:key:`qemu-system@.service`.

.. _usage-list:

Listing and inspecting
======================

Two commands list the machines, `systemd`_'s view and
:cmd:`machined <systemd-machined>`'s:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user list-units 'qemu-system@*'
   $ machinectl --user list

The first lists the units, failed ones included; the second the machines
that registered, with their class and service (:doc:`machined`).

``status`` shows a machine's state and the last ten lines of its
journal, which are the end of the serial console:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user status qemu-system@bootstrap-direct.service

``systemctl --user cat`` prints the unit the generator wrote.

.. _usage-start:

Starting and stopping
=====================

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system@bootstrap-direct.service
   $ systemctl --user stop qemu-system@bootstrap-direct.service
   $ systemctl --user restart qemu-system@bootstrap-direct.service

A start returns as soon as `QEMU`_ runs; the guest boots after that. A stop
asks the guest to power off, first over SSH, then with the power button,
and waits for QEMU to exit (:ref:`SSH poweroff <host-poweroff>`). On
a restart, the machine's `virtiofsd`_ daemons exit with the old QEMU and
start again when the new one connects.
Stopping :cmd:`machines.target` stops every machine at once; starting it
starts each one whose file names it in :cmd:`WantedBy=`
(:ref:`direct-settings`).

A guest that cannot answer yet takes the full stop timeout, two minutes.
``kill`` ends it at once (:ref:`kill <host-kill>`). It sends
``SIGTERM``, on which QEMU exits cleanly. A signal QEMU does not handle,
such as ``--signal=SIGKILL``, leaves the service ``failed`` until the
next start, or until you clear it:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user kill qemu-system@bootstrap-direct.service
   $ systemctl --user kill --signal=SIGKILL qemu-system@bootstrap-direct.service
   $ systemctl --user reset-failed qemu-system@bootstrap-direct.service

Stop a machine with ``systemctl``. ``machinectl poweroff`` kills QEMU
outright (:ref:`machined-verbs`).

Rebooting
=========

A reboot inside the guest resets the machine and leaves QEMU running,
so the service stays active with the same process:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-direct systemctl reboot

`QMP`_ resets a hung guest from the host, as the reset button would
(:ref:`usage-qmp`). A machine whose file has ``no-reboot=`` in
:key:`[QEMU]` exits instead of rebooting, and its service stops.

.. _usage-pause:

Pausing
========

``freeze`` suspends every process in the service's cgroup, QEMU
included, until ``thaw``:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user freeze qemu-system@bootstrap-direct.service
   $ systemctl --user thaw qemu-system@bootstrap-direct.service

While frozen, nothing answers: not the guest, not SSH, not the QMP
socket. A stop or restart thaws the unit first. To pause only the guest's
processors and keep QEMU answering, send QMP's ``stop`` and ``cont``
instead (:ref:`usage-qmp`).

.. _usage-boot:

At login and at boot
====================

A machine whose file has ``WantedBy=machines.target`` in ``[Install]``
starts with :cmd:`machines.target`. systemd's presets enable that target
for the system manager but not for yours, so enable it once:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user enable machines.target

A user manager runs only while you are logged in, and stops every
machine when you log out. Lingering keeps it running from boot to
shutdown, so the machines start at boot. The default policy lets you
enable it for yourself without root:

.. code-block:: console
   :class: cmd-host

   $ loginctl enable-linger
   $ loginctl show-user "$USER" --property=Linger
   Linger=yes

The machines then stop when the host shuts down, each as
``systemctl --user stop`` would stop it.

.. _usage-remove:

Removing a machine
==================

Stop the machine, delete its state directory while its unit still
loads, then remove its file and drop-in directory and regenerate:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap-direct.service
   $ systemctl --user clean --what=state qemu-system@bootstrap-direct.service
   $ rm --recursive --force ~/.config/qemu/systemd/bootstrap-direct.qemu ~/.config/qemu/systemd/bootstrap-direct.qemu.d
   $ systemctl --user start qemu-system-generator.service

``clean --what=state`` deletes the state directory with everything in it,
disk images included (:cmd:`systemctl`); leave it out to keep them.
The generator deletes the units it wrote for a file that is gone.

.. _systemd: https://systemd.io
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _QMP: https://www.qemu.org/docs/master/interop/qmp-spec.html
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
