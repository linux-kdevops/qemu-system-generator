.. SPDX-License-Identifier: copyleft-next-0.3.1

What you need
=============

- A Linux host with `KVM`_, where your user can open ``/dev/kvm``.
- systemd with a user manager, which is what
  :cmd:`systemctl` ``--user`` talks to.
- `Nix`_ with flakes enabled.
- A checkout of `this repository`_.

Preparation
===========

Everything you write or build lives in ``~/bootstrap``, with the SSH key
and configuration in ``ssh/``:

.. code-block:: console
   :caption: host
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/ssh/config.d

SSH
---

The guests accept one key for root, so keep it for these guides only.
Create it with :cmd:`ssh-keygen`:

.. code-block:: console
   :class: cmd-host

   $ ssh-keygen -t ed25519 -N "" -C bootstrap -f ~/bootstrap/ssh/id_ed25519

Once a machine has a vsock device, its guide adds a ``Host`` block for it
to ``~/bootstrap/ssh/config.d/``, and ``~/bootstrap/ssh/config`` collects
them:

.. code-block:: text
   :caption: ~/bootstrap/ssh/config

   Include ~/bootstrap/ssh/config.d/*.conf

These guides never edit your ``~/.ssh/config``. Add one ``Include`` of
theirs near its top yourself, before any ``Host`` or ``Match`` line, which
would make the ``Include`` conditional (:cmd:`ssh_config(5)`). Use the
``~`` path, since ssh resolves a relative ``Include`` against ``~/.ssh``:

.. code-block:: text
   :caption: ~/.ssh/config

   Include ~/bootstrap/ssh/config

Then ``ssh <machine>`` reaches a guest by its name.

.. _KVM: https://docs.kernel.org/virt/kvm/index.html
.. _Nix: https://nix.dev/manual/nix/latest/
.. _this repository: https://github.com/linux-kdevops/qemu-system-generator
