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
in ``ssh/``:

.. code-block:: console
   :caption: host
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/ssh

A throwaway SSH key
-------------------

The guest accepts this key for root, so keep it for these guides only.
Create it with :cmd:`ssh-keygen`:

.. code-block:: console
   :class: cmd-host

   $ ssh-keygen -t ed25519 -N "" -C bootstrap -f ~/bootstrap/ssh/id_ed25519

.. _KVM: https://docs.kernel.org/virt/kvm/index.html
.. _Nix: https://nix.dev/manual/nix/latest/
.. _this repository: https://github.com/linux-kdevops/qemu-system-generator
