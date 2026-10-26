.. SPDX-License-Identifier: copyleft-next-0.3.1

================
Prepare the host
================

Every guide starts from a host prepared once, as below: the device
access, the SSH key and configuration, and the generator.

What you need
=============

- A host whose kernel has `KVM`_, where your user can open ``/dev/kvm``.
- systemd with a user manager, which is what
  :cmd:`systemctl` ``--user`` talks to.
- `Nix`_ with flakes enabled.
- A checkout of `this repository`_.

Two of these need root, once. systemd's udev rules give ``/dev/kvm`` and
``/dev/vhost-vsock`` to the ``kvm`` group, and the vsock device needs the
``vhost_vsock`` module, loaded at every boot from then on:

.. code-block:: console
   :caption: host
   :class: cmd-host

   $ sudo usermod --append --groups kvm "$(whoami)"
   $ echo vhost_vsock | sudo tee /etc/modules-load.d/vhost_vsock.conf
   $ sudo modprobe vhost_vsock

Log out and back in for the group to take effect. Nothing else in these
guides runs as root.

The workspace
=============

Everything you write or build lives in ``~/bootstrap``: a directory per
guide, and in ``system/`` what every guide shares, the SSH key and
configuration in ``system/ssh/`` and the directory the guests share in
``system/share/``:

.. code-block:: console
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

Every guest gets a `vsock`_ device with a context id of its own, a socket
address the host reaches without any network. One configuration names
every guide's machine by its context id and lets them all in the same
way:

.. code-block:: text
   :caption: ~/bootstrap/system/ssh/config

   Include ~/bootstrap/system/ssh/config.d/*.conf

   Host bootstrap
       HostName vsock/42
   Host cloud-debian
       HostName vsock/43
   Host cloud-fedora
       HostName vsock/44
   Host cloud-ubuntu
       HostName vsock/45
   Host peer
       HostName vsock/46

   Host bootstrap cloud-debian cloud-fedora cloud-ubuntu peer
       ProxyCommand /usr/lib/systemd/systemd-ssh-proxy %h %p
       ProxyUseFdpass yes
       IdentityFile ~/bootstrap/system/ssh/id_ed25519
       IdentitiesOnly yes
       User root
       StrictHostKeyChecking accept-new
       UserKnownHostsFile /dev/null

ssh takes the first value it finds for each keyword, so the two blocks
for one name add up (:cmd:`ssh_config(5)`). A machine of your own gets a
file of its own in ``config.d/``. The last block dials the context id
through :cmd:`systemd-ssh-proxy(1)`, which systemd installs in its private
library directory, ``/usr/lib/systemd/`` on most distributions;
:cmd:`systemd-path system-library-private <systemd-path>` prints the
prefix. ``ProxyUseFdpass`` has the proxy hand ssh the connected socket,
and ``IdentitiesOnly`` keeps an agent's other keys from being offered
first. A guest makes new host keys on a fresh root file system, so
``UserKnownHostsFile /dev/null`` keeps them out of your ``known_hosts``,
and each connection prints that it added one.

ssh refuses a configuration file that anyone but you can write, and names
it as having "Bad owner or permissions". With a ``umask`` of ``002``, run
``chmod go-w`` on each file you create here.

These guides never edit your ``~/.ssh/config``. Add one ``Include`` of
theirs near its top yourself, before any ``Host`` or ``Match`` line, which
would make the ``Include`` conditional (:cmd:`ssh_config(5)`). Use the
``~`` path, since ssh resolves a relative ``Include`` against ``~/.ssh``:

.. code-block:: text
   :caption: ~/.ssh/config

   Include ~/bootstrap/system/ssh/config

Then ``ssh bootstrap`` reaches the guest named ``bootstrap`` once it runs.
A guest listens for SSH on vsock by itself through
:cmd:`systemd-ssh-generator(8)`, which NixOS and the cloud images have.

.. _host-generator:

The generator
=============

Add it to your Nix profile with :cmd:`nix profile add`, from the checkout
of this repository:

.. code-block:: console
   :class: cmd-host

   $ nix profile add .

The profile puts ``qemu-system-generator`` on ``PATH``. systemd runs user
generators only from root-owned directories
(:cmd:`systemd.generator(7)`), so a user manager never runs this one on its
own. Two user units do it instead. Enable them through the profile's
link, ``~/.nix-profile`` (``~/.local/state/nix/profile`` with
`use-xdg-base-directories`_):

.. code-block:: console
   :class: cmd-host

   $ systemctl --user enable --now \
       ~/.nix-profile/lib/systemd/user/qemu-system-generator.service \
       ~/.nix-profile/lib/systemd/user/qemu-system-generator.path

``systemctl`` links the units by the path it is given, not by the store
path behind it. After
:cmd:`nix profile upgrade qemu-system-generator <nix profile upgrade>`, a
``systemctl --user daemon-reload`` loads the new generation's units.

The service turns every ``.qemu`` file in ``~/.config/qemu/systemd/`` into
units and reloads the manager; it runs at login, and the
:cmd:`systemd.path(5)` unit runs it again whenever that directory changes.

.. _host-start:

Starting and stopping a machine
===============================

A ``.qemu`` file is named after the machine, and the unit after the file.
Each line of :key:`[QEMU]` is one QEMU option with its dash removed, passed
on as written: ``m=2G`` is ``-m 2G``, and ``nographic=`` with an empty value
is the bare flag ``-nographic``. :cmd:`qemu-system-x86_64(1)` documents them
all, and :doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` the rest of
the file.

A save takes a moment to reach the path unit, so the guides regenerate by
hand before starting a machine; the start waits until the reload is done.
:cmd:`journalctl` shows the serial console:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user start qemu-system@bootstrap.service
   $ journalctl --user-unit=qemu-system@bootstrap.service --follow

"Regenerate and restart" in a guide means the first line, then
``systemctl --user restart``.

:doc:`/usage/index` shows what else ``systemctl``, ``journalctl``,
``machinectl`` and QMP do with a running machine.

.. _host-poweroff:

:key:`[VM]` holds what is not a QEMU option. With
:key:`SshPrivateKey=` there, stopping the machine first runs ``systemctl
poweroff`` in the guest over vsock, and presses the power button only if
that fails.

.. _host-kill:

``systemctl --user stop`` presses the power button and waits two minutes
for a guest that cannot answer it yet. Kill such a guest instead; QEMU
exits on ``SIGTERM``:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user kill qemu-system@bootstrap.service

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
the dashes. The generator runs the daemon from its own socket unit and
adds the QEMU lines that connect to it. ``sandbox=``, ``uid-map=`` and
``gid-map=`` let it run without root: ``%U`` and ``%G`` are your user and
group ids, which the guest's root maps to, and ``%h`` is your home
directory, among the `specifiers`_ systemd expands in a unit. The guides
share the host's ``/nix/store`` and ``~/bootstrap/system/share`` this way.

.. _KVM: https://docs.kernel.org/virt/kvm/index.html
.. _Nix: https://nix.dev/manual/nix/latest/
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _this repository: https://github.com/linux-kdevops/qemu-system-generator
.. _use-xdg-base-directories: https://nix.dev/manual/nix/latest/command-ref/conf-file.html#conf-use-xdg-base-directories
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
