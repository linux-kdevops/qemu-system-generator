.. SPDX-License-Identifier: copyleft-next-0.3.1

==============
The home share
==============

Every guide shares your home with its guests at the same path, and the
`NixOS`_ guests make it root's home. Write your own home where the
examples show ``/home/user``.

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

.. _home-guest:

The guest side
==============

The NixOS guests take one more module, ``home.nix``, the same in every
guide:

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
     users.users.root.home = lib.mkForce home;
   }

NixOS sets root's home to ``/root`` itself, so the module overrides it
with ``lib.mkForce``.

.. _home-units:

Your units stay on the host
---------------------------

``home.nix`` hides ``~/.config/systemd`` and ``~/.local/share/systemd``
under an empty, read-only `tmpfs`_, so the guests do not run your user
units (:cmd:`systemd.unit(5)`).

.. _home-ssh:

What else the guest sees
------------------------

NixOS's :cmd:`sshd <sshd(8)>` reads ``~/.ssh/authorized_keys`` as well as
the keys set in the configuration
(`services.openssh.authorizedKeysInHomedir`_ and ``AuthorizedKeysFile`` in
:cmd:`sshd_config(5)`), so a key your account accepts on the host also logs
in to the guests as root. The files root writes, such as its shell history,
land in your home.

.. _home-cloud:

The cloud guests
================

`cloud-init`_ writes root's key to ``~/.ssh/authorized_keys``, which
would put the guide's key in your own file if root lived in the shared
home. The cloud guests therefore mount the home at the same path and keep
root's home in ``/root``. No user manager starts in the shared home there, so
they need no tmpfs over it.

.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _tmpfs: https://docs.kernel.org/filesystems/tmpfs.html
.. _services.openssh.authorizedKeysInHomedir: https://nixos.org/manual/nixos/stable/options#opt-services.openssh.authorizedKeysInHomedir
.. _cloud-init: https://cloudinit.readthedocs.io
