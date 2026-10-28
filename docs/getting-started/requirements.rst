.. SPDX-License-Identifier: copyleft-next-0.3.1

============
Requirements
============

The host needs two things installed: `systemd`_ and `Nix`_. systemd runs
the machines, as services of your user's manager or of the system's. Nix
builds the generator together with the `QEMU`_, `virtiofsd`_, :cmd:`socat`
and :cmd:`varlinkctl` its units call, pinned to store paths, so the host
needs no QEMU package from its distribution.

systemd
=======

systemd must be the init system. :cmd:`systemctl is-system-running
<systemctl>` prints the manager's state, ``running``, or ``degraded`` if
a unit has failed, and ``offline`` where another program is PID 1.
Either of the first two will do:

.. code-block:: console
   :caption: host
   :class: cmd-host

   $ systemctl is-system-running
   degraded

Two parts of a machine's lifecycle need a recent systemd on the host.
:cmd:`ssh` reaches a guest over `vsock`_ through :cmd:`systemd-ssh-proxy(1)`,
added in systemd 256. A machine registers with your user's
:cmd:`systemd-machined`, which runs per user since systemd 259; the
registration is best effort, and an older systemd runs the machine
without it.

Nix
===

Install Nix with the multi-user (daemon) installer:

.. code-block:: console
   :class: cmd-host

   $ curl --proto '=https' --tlsv1.2 -L https://nixos.org/nix/install \
       | sh -s -- --daemon

The `Nix install guide`_ has the other installers and notes per platform.
The flake needs the ``nix`` command and flakes, enabled once:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/.config/nix
   $ echo 'experimental-features = nix-command flakes' \
       | tee --append ~/.config/nix/nix.conf

Device access
=============

QEMU's ``accel=kvm`` opens ``/dev/kvm``, and a ``vhost-vsock`` device
opens ``/dev/vhost-vsock``. systemd's `udev`_ rules give both to the ``kvm``
group, and the vsock device needs the ``vhost_vsock`` module, loaded at
every boot from then on. Both need root, once, through :cmd:`usermod` and
:cmd:`modprobe`:

.. code-block:: console
   :class: cmd-host

   $ sudo usermod --append --groups kvm "$(whoami)"
   $ echo vhost_vsock | sudo tee /etc/modules-load.d/vhost_vsock.conf
   $ sudo modprobe vhost_vsock

Log out and back in for the group to take effect.

PCI passthrough
===============

Skip this section unless a machine takes a host PCI device through VFIO.
The generator grants the service the VFIO device nodes and sets
:cmd:`LimitMEMLOCK=infinity <LimitMEMLOCK=>`, since an assigned device
pins guest memory. The rest is host configuration, done once as root.

Bind each device to the ``vfio-pci`` driver, as the kernel's `VFIO`_
documentation shows. Its ``/dev/vfio/<group>`` node belongs to root; a
udev rule gives the nodes to the ``kvm`` group, and :cmd:`udevadm` applies
it:

.. code-block:: text
   :caption: /etc/udev/rules.d/10-vfio-kvm.rules

   SUBSYSTEM=="vfio", GROUP="kvm", MODE="0660"

.. code-block:: console
   :class: cmd-host

   $ sudo udevadm control --reload-rules
   $ sudo udevadm trigger --subsystem-match=vfio

A user unit cannot raise a limit above the hard limit of the manager
that runs it, and a user manager inherits the system's default of 8 MiB
for locked memory (``DefaultLimitMEMLOCK=`` in ``systemd-system.conf``).
Raise it for every user manager with a drop-in:

.. code-block:: console
   :class: cmd-host

   $ sudo mkdir --parents /etc/systemd/system/user@.service.d
   $ printf '[Service]\nLimitMEMLOCK=infinity\n' \
       | sudo tee /etc/systemd/system/user@.service.d/memlock.conf
   $ sudo systemctl daemon-reload

Your manager gets the new limit when it next starts. Restarting it stops
every unit it runs, so do it from a session you can afford to lose, or
reboot:

.. code-block:: console
   :class: cmd-host

   $ sudo systemctl restart "user@$(id --user).service"
   $ systemctl show --property=LimitMEMLOCK "user@$(id --user).service"

.. _systemd: https://systemd.io
.. _Nix: https://nix.dev/manual/nix/latest/
.. _Nix install guide: https://nixos.org/download/#nix-install-linux
.. _VFIO: https://docs.kernel.org/driver-api/vfio.html
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _udev: https://www.freedesktop.org/software/systemd/man/latest/udev.html
