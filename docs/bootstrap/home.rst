.. SPDX-License-Identifier: copyleft-next-0.3.1

==============
The home share
==============

Every guide shares your home with its guests at the same path, as root's
home. Write your own home where the examples show ``/home/user``.

.. _home-host:

The host side
=============

One :key:`[virtiofsd]` section serves the home (:ref:`host-virtiofsd`):

.. code-block:: ini

   [virtiofsd]
   shared-dir=%h
   tag=home
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

The keys are virtiofsd's long options, documented in its `README`_.

.. _home-guest:

The guest side
==============

Each `NixOS`_ guide saves ``home.nix`` next to its ``flake.nix``, in
``~/bootstrap/<guide>/`` (``~/bootstrap/`knix`_/guest/`` for knix), and adds
it to the guest's modules:

.. code-block:: nix
   :caption: home.nix

   { lib, ... }:
   let
     home = "/home/user";
     mask = {
       device = "none";
       fsType = "tmpfs";
       options = [ "ro" "nosuid" "nodev" "mode=0555" ];
     };
   in
   {
     fileSystems.${home} = {
       device = "home";
       fsType = "virtiofs";
     };
     fileSystems."${home}/.config/systemd" = mask;
     fileSystems."${home}/.local/share/systemd" = mask;
     fileSystems."${home}/.ssh" = mask;
     users.users.root.home = lib.mkForce home;
   }

NixOS sets root's home to ``/root`` itself, so the module overrides it
with ``lib.mkForce``.

.. _home-units:

Your units and keys stay on the host
------------------------------------

``home.nix`` hides ``~/.config/systemd``, ``~/.local/share/systemd`` and
``~/.ssh`` under an empty, read-only `tmpfs`_, so the guests neither run
your user units (:cmd:`systemd.unit(5)`) nor read your keys.

.. _home-cloud:

The cloud guests
================

`cloud-init`_ writes root's key to ``/root/.ssh``, and the sshd that
:cmd:`systemd-ssh-generator(8)` starts on `vsock`_ reads keys from
``~/.ssh`` only. The cloud seed makes the shared home root's home with the
same two systemd masks, and covers ``~/.ssh`` with the guest's own
``/root/.ssh`` instead of an empty tmpfs (:ref:`cloud-home`).

.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _tmpfs: https://docs.kernel.org/filesystems/tmpfs.html
.. _cloud-init: https://cloudinit.readthedocs.io
.. _README: https://gitlab.com/virtio-fs/virtiofsd/-/blob/main/README.md#options
.. _knix: https://github.com/linux-kdevops/knix
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
