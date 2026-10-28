.. SPDX-License-Identifier: copyleft-next-0.3.1

================
Prepare the host
================

Every guide starts from a host that meets the
:doc:`/getting-started/requirements`, with the generator installed
:ref:`for your user <install-user>`, and from the workspace and SSH
configuration below. Nothing in these guides runs as root.

The workspace
=============

Everything you write or build lives in ``~/bootstrap``, a directory per
guide. What every guide shares lives in ``system/``, and the SSH key and
configuration in ``system/ssh/``:

.. code-block:: console
   :caption: host
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/system/ssh/config.d

.. _host-ssh:

SSH
===

The guests accept one key for root, so keep it for these guides only.
Create it with :cmd:`ssh-keygen`:

.. code-block:: console
   :class: cmd-host

   $ ssh-keygen -t ed25519 -N "" -C bootstrap -f ~/bootstrap/system/ssh/id_ed25519

The guides paste the public key,
``~/bootstrap/system/ssh/id_ed25519.pub``, into each guest's
configuration.

Every guest gets a `vsock`_ device with a context id of its own. One
configuration names every guide's machine by its context id and reaches
it through :cmd:`systemd-ssh-proxy(1)`:

.. code-block:: text
   :caption: ~/bootstrap/system/ssh/config

   Include ~/bootstrap/system/ssh/config.d/*.conf

   Host bootstrap-disk
       HostName vsock/42
   Host cloud-debian
       HostName vsock/43
   Host cloud-fedora
       HostName vsock/44
   Host cloud-ubuntu
       HostName vsock/45
   Host bootstrap-uki
       HostName vsock/46
   Host bootstrap-direct
       HostName vsock/47
   Host bootstrap-direct-peer
       HostName vsock/48
   Host bootstrap-knix
       HostName vsock/49
   Host bootstrap-knix-peer
       HostName vsock/50
   Host cloud-debian-peer
       HostName vsock/51
   Host cloud-fedora-peer
       HostName vsock/52
   Host cloud-ubuntu-peer
       HostName vsock/53

   Host bootstrap-* cloud-*
       ProxyCommand /usr/lib/systemd/systemd-ssh-proxy %h %p
       ProxyUseFdpass yes
       IdentityFile ~/bootstrap/system/ssh/id_ed25519
       IdentitiesOnly yes
       User root
       StrictHostKeyChecking accept-new
       UserKnownHostsFile /dev/null

Add this to your ``~/.ssh/config``:

.. code-block:: text
   :caption: ~/.ssh/config

   Include ~/bootstrap/system/ssh/config

Then ``ssh bootstrap-disk`` reaches the disk guide's machine once it runs.
The guests listen for SSH on vsock through
:cmd:`systemd-ssh-generator(8)`.

.. _host-file:

Running a machine
=================

A ``.qemu`` file is named after the machine, and the unit after the file.
Each line of :key:`[QEMU]` is one QEMU option with its dash removed, passed
on as written: ``m=2G`` is ``-m 2G``, and ``nographic=`` with an empty value
is the bare flag ``-nographic``. :cmd:`qemu-system-x86_64(1)` documents them
all, and :doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` the rest of
the file. :doc:`/usage/index` shows what else :cmd:`systemctl`,
:cmd:`journalctl`, :cmd:`machinectl` and `QMP`_ do with a running machine.

.. _host-regenerate:

Regenerating
------------

The generator turns the ``.qemu`` files into units. Its path unit runs it
a moment after a ``.qemu`` file changes, but not after a change in a
drop-in directory, so the guides regenerate by hand after every change:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service

The start returns once the user manager has loaded the new units.

.. _host-start:

Starting
--------

Start the machine and follow its serial console in the journal:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system@bootstrap-disk.service
   $ journalctl --user-unit=qemu-system@bootstrap-disk.service --follow

A running machine keeps the command line it started with. After
regenerating, restart it to boot with the new one:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user restart qemu-system@bootstrap-disk.service

.. _host-stop:

Stopping
--------

``systemctl --user stop`` presses the guest's power button and waits up
to two minutes for QEMU to exit. A guest whose operating system runs
answers it:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap-disk.service

.. _host-poweroff:

With :key:`SshPrivateKey=` in :key:`[VM]`, which holds what is not a QEMU
option, the stop first runs ``systemctl poweroff`` in the guest over
vsock, and presses the power button only if that fails.

.. _host-kill:

Firmware with nothing to boot, or a guest still booting, does not answer
the power button, and the stop would wait the full two minutes. Kill such
a guest instead; QEMU exits on ``SIGTERM``:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user kill qemu-system@bootstrap-disk.service

.. _host-machined:

Registration
============

The generated unit registers each machine with your user's
:cmd:`systemd-machined.service(8)`. With a vsock device it records the
context id, and ``machinectl --user list`` lists the machine;
:doc:`/usage/machined` has the rest.

.. _host-virtiofsd:

Shares
======

Each :key:`[virtiofsd]` section runs one `virtiofsd`_ daemon for one shared
directory. Its keys are the long options of ``virtiofsd --help`` without
the dashes. The generator runs the daemon from its own socket unit and adds
the QEMU lines that connect to it. ``%U``, ``%G`` and ``%h`` are `systemd`_
`specifiers`_. The guides share the host's ``/nix/store`` and
your home this way; :doc:`home` has what the guest does with the home.

.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _QMP: https://www.qemu.org/docs/master/interop/qmp-spec.html
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _systemd: https://systemd.io
