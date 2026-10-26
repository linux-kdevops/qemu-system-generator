.. SPDX-License-Identifier: copyleft-next-0.3.1

=============================
machined, machinectl, varlink
=============================

:cmd:`systemd-machined` keeps a list of the virtual machines and
containers that run on a host. Each user manager has one of its own,
since systemd 259, and every machine the generator writes registers with
yours as it starts (:ref:`host-machined`): its name, class ``vm``,
service ``qemu-system``, QEMU's process as its leader, the vsock context
id from a ``vhost-vsock`` device, and the UUID from ``uuid=``.
Registration is best effort: a machine runs without machined. It leaves
the list when QEMU exits.

machined serves the list over `Varlink`_, a socket protocol that
:cmd:`machinectl` and :cmd:`varlinkctl` both speak. ``--user`` selects
your user's machined; without it, both ask the system's, which does not
know your machines.

.. _machined-verbs:

machinectl
==========

.. code-block:: console
   :class: cmd-host

   $ machinectl --user list
   MACHINE   CLASS SERVICE     OS VERSION ADDRESSES
   bootstrap vm    qemu-system -  -       -
   $ machinectl --user status bootstrap
   $ machinectl --user show bootstrap

``show`` prints the registration, ``VSockCID=42`` among it. The OS,
version and address columns stay empty for a virtual machine, which
machined cannot look into.

``status`` and ``show`` name the unit ``user@1000.service``, your whole
user manager, with the machine's service as a subgroup of it. That is a
bug in systemd 259 to 262: a user's machined looks up a machine
registered over Varlink in the system manager's view of the process,
where everything you run sits under ``user@1000.service``. A
registration over D-Bus gets the user manager's view and the machine's
own service. The journal lines ``status`` prints therefore come from
every one of your user services. Read the machine's own with
``journalctl --user-unit=`` (:ref:`usage-console-log`).

The other verbs were written for containers and for machines that
systemd-vmspawn starts. Few of them fit a machine the generator writes:

``login``, ``shell``, ``bind``, ``copy-to``, ``copy-from``
   Containers only. machined refuses them for a virtual machine; use
   ``ssh`` and ``scp`` instead.
``poweroff``, ``reboot``, ``pause``, ``resume``
   Since systemd 261 these go to the machine's control socket, a
   Varlink service (``io.systemd.MachineInstance``) that systemd-vmspawn
   runs in front of QEMU's QMP. QEMU speaks only QMP, so a machine the
   generator writes registers no control socket. ``pause`` and
   ``resume`` then fail with "does not expose a varlink control socket".
   ``poweroff`` and ``reboot`` fall back to the signals that shut a
   container down, ``SIGRTMIN+4`` and ``SIGINT``, sent to QEMU. QEMU dies
   on the first and exits on the second, and the guest never shuts down
   either way. Use ``systemctl --user stop`` (:ref:`usage-start`), and
   QMP's ``stop`` and ``cont`` to pause (:ref:`usage-qmp`).
``terminate``, ``kill``
   Both act on the registered unit, which the bug above makes your user
   manager. ``terminate`` fails with "No such process" and ``kill`` with
   "No such file or directory". ``kill --kill-whom=leader --signal=...``
   still signals QEMU itself.
``start``, ``enable``, and the image verbs
   They work on systemd-nspawn and systemd-vmspawn images, not on a
   ``.qemu`` file.

:cmd:`systemctl` ``--machine=`` and :cmd:`journalctl` ``--machine=``
reach into containers only as well.

.. _usage-ssh:

ssh by machine name
===================

:cmd:`systemd-ssh-proxy(1)` connects ssh to a host name of the form
``machine/name``: it asks your user's machined for the machine's vsock
context id, then the system's if yours does not know the name, and
dials the guest's port 22 over vsock. systemd's ``ssh_config`` snippet,
``/etc/ssh/ssh_config.d/20-systemd-ssh-proxy.conf`` on most
distributions, routes ``machine/*`` through it as root, so the key is
all ssh still needs:

.. code-block:: console
   :class: cmd-host

   $ ssh -i ~/bootstrap/system/ssh/id_ed25519 machine/bootstrap

The same snippet routes ``vsock/42``, by context id. The bootstrap guides
name each machine in ``~/bootstrap/system/ssh/config`` instead
(:ref:`host-ssh`), which works without the snippet and keeps
``ssh bootstrap`` short.

.. _usage-varlink:

varlinkctl
==========

Your machined listens on ``io.systemd.Machine`` in
``$XDG_RUNTIME_DIR/systemd/machine``. :cmd:`varlinkctl` lists what it
offers, prints an interface with the documentation of each field, and
calls a method with JSON parameters:

.. code-block:: console
   :class: cmd-host

   $ varlinkctl list-methods $XDG_RUNTIME_DIR/systemd/machine/io.systemd.Machine
   $ varlinkctl introspect $XDG_RUNTIME_DIR/systemd/machine/io.systemd.Machine io.systemd.Machine
   $ varlinkctl call $XDG_RUNTIME_DIR/systemd/machine/io.systemd.Machine io.systemd.Machine.List '{"name":"bootstrap"}'
   {"name":"bootstrap","class":"vm","service":"qemu-system",...,"vSockCid":42,"UID":1000}

``List`` with ``'{}'`` returns every machine, one JSON object each, but
only with ``--more``; ``--json=pretty`` indents them:

.. code-block:: console
   :class: cmd-host

   $ varlinkctl call --more --json=pretty $XDG_RUNTIME_DIR/systemd/machine/io.systemd.Machine io.systemd.Machine.List '{}'

``Register``, called the same way, is what the generated unit runs as
it starts (``ExecStartPost=`` in
:doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>`).

.. _Varlink: https://varlink.org
