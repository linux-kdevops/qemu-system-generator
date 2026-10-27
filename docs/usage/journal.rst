.. SPDX-License-Identifier: copyleft-next-0.3.1

========
Journal
========

Two journals are involved: the host's, which holds the machine's serial
console and what `systemd`_ and `QEMU`_ say about it, and the guest's own,
which holds everything the guest logs. :cmd:`journalctl` reads both.

.. _usage-console-log:

The serial console
==================

With ``serial=mon:stdio``, as in the guides, the guest's serial console
is QEMU's standard output, which the service sends to the host journal
under ``qemu-system@name``. Follow it live, or read it back:

.. code-block:: console
   :class: cmd-host

   $ journalctl --user-unit=qemu-system@bootstrap-direct.service --follow
   $ journalctl --user-unit=qemu-system@bootstrap-direct.service --boot
   $ journalctl --user-unit=qemu-system@bootstrap-direct.service --invocation=0
   $ journalctl --user-unit=qemu-system@bootstrap-direct.service --list-invocations

Each start of the service is an invocation; ``--invocation=0`` shows
the latest, ``--invocation=-1`` the one before, and
``--list-invocations`` lists them all.
A pattern names several units, such as a machine's `virtiofsd`_ daemons:

.. code-block:: console
   :class: cmd-host

   $ journalctl --user-unit='virtiofsd@bootstrap-direct-*'

Use ``--user-unit=``, not ``--user --unit=``. ``--user`` reads only your
own journal files, and :cmd:`journald <systemd-journald.service(8)>` writes
those only with persistent storage (``SplitMode=`` in
:cmd:`journald.conf(5)`). On a host whose journal is in ``/run``, it finds
nothing. ``--user-unit=`` matches your units' entries in every journal you
can read. Reading entries that landed in the system journal needs the
``systemd-journal`` group.

.. _usage-guest-journal:

The guest's journal
===================

The guest's own journal is on the guest. Read it over SSH as root:

.. code-block:: console
   :class: cmd-host

   $ ssh bootstrap-direct journalctl --boot
   $ ssh bootstrap-direct journalctl --dmesg
   $ ssh bootstrap-direct journalctl --list-boots
   $ ssh bootstrap-direct journalctl --boot --priority=err
   $ ssh bootstrap-direct journalctl --unit=sshd.service --follow
   $ ssh bootstrap-direct systemd-analyze blame

``--list-boots`` lists the boots the guest still has: a guest whose root
file system is new at every boot, as in the direct and `knix`_ guides,
has only the current one.

.. _usage-forward:

The guest's journal on the host
===============================

A guest with a fresh root loses its journal, and SSH cannot reach a
guest that stops answering. journald can send every entry over a socket
as it logs it (``ForwardToSocket=`` in :cmd:`journald.conf(5)`, systemd
256 and later), and :cmd:`systemd-journal-remote.service(8)` on the host
writes what arrives into journal files. The socket can be a `vsock`_ one,
so this needs no network and no root.

On the host, a socket unit and its service of your own listen on port
19532 of the host's vsock address, ``2``, for every guest at once. The
port must be above 1023, since the kernel reserves the lower ports of
vsock for root. Make the directory the files go to, and write the two
units:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/journal

.. code-block:: ini
   :caption: ~/.config/systemd/user/journal-remote.socket

   [Unit]
   Description=Journal entries from the guests

   [Socket]
   ListenStream=vsock:2:19532

   [Install]
   WantedBy=sockets.target

.. code-block:: ini
   :caption: ~/.config/systemd/user/journal-remote.service

   [Unit]
   Description=Journal entries from the guests

   [Service]
   ExecStart=/usr/lib/systemd/systemd-journal-remote --output=%h/bootstrap/journal/guests.journal --split-mode=none

.. code-block:: console
   :class: cmd-host

   $ systemctl --user daemon-reload
   $ systemctl --user enable --now journal-remote.socket

`Debian`_ and `Ubuntu`_ ship ``systemd-journal-remote`` in a package of that
name. Like ``systemd-ssh-proxy`` (:ref:`host-ssh`), it lives in systemd's
private library directory, which
:cmd:`systemd-path system-library-private <systemd-path>` prints.
``--split-mode=none`` writes one file for all the guests;
``--split-mode=host`` would write one per connection, named after the
guest's vsock address and port, not its name.

On the guest, journald takes the address from the
``journal.forward_to_socket`` credential
(:cmd:`systemd.system-credentials(7)`). QEMU passes it in the `SMBIOS`_
tables (:cmd:`smbios-type-11(7)`), one line in :key:`[QEMU]`:

.. code-block:: ini

   [QEMU]
   smbios=type=11,value=io.systemd.credential:journal.forward_to_socket=vsock:2:19532

systemd reads those tables through the kernel's ``dmi-sysfs`` driver,
which it loads early if it is a module. With a kernel that lacks it,
such as the knix guide's trimmed one, pass the credential on the kernel
command line instead, before the rest of ``append=``:

.. code-block:: ini

   [QEMU]
   append=console=ttyS0 systemd.set_credential=journal.forward_to_socket:vsock:2:19532 ...

Restart the machine. The guest's journal then reaches the host from its
first kernel message on, and :cmd:`journalctl` reads it with
``--directory``, picking one guest by its hostname:

.. code-block:: console
   :class: cmd-host

   $ journalctl --directory ~/bootstrap/journal _HOSTNAME=bootstrap --boot
   $ journalctl --directory ~/bootstrap/journal --list-boots
   $ journalctl --directory ~/bootstrap/journal --field=_HOSTNAME

Other remote journals
=====================

Between hosts on a network, systemd has two more tools.
:cmd:`systemd-journal-upload.service(8)` sends a system's journal over
HTTP or HTTPS to a ``systemd-journal-remote`` started with
``--listen-http=`` or ``--listen-https=``, and
:cmd:`systemd-journal-gatewayd.service(8)` serves a system's journal
over HTTP, on port 19531, to a browser or :cmd:`curl`. Both need the guest
to have an address the other side can reach; the vsock socket above
needs none.

.. _systemd: https://systemd.io
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _knix: https://github.com/linux-kdevops/knix
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _Debian: https://www.debian.org
.. _Ubuntu: https://ubuntu.com
.. _SMBIOS: https://www.dmtf.org/standards/smbios
