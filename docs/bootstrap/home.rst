.. SPDX-License-Identifier: copyleft-next-0.3.1

==============
The home share
==============

Every guide shares your home directory with its guests and mounts it at
the same path, so a path names the same file on the host and in the
guest. In the `NixOS`_ guests it is also root's home, and ``ssh bootstrap``
lands in your files. The examples show the home as ``/home/user``; write
your own wherever it appears in a guest's configuration.

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

``uid-map=`` and ``gid-map=`` map the guest's root to your user and group:
your files belong to root in the guest, and what root writes there
belongs to you on the host.

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

Your user manager's units, and the links that enable them, are files in
your home: ``~/.config/systemd/user/`` and ``~/.local/share/systemd/user/``
are both on its search path (:cmd:`systemd.unit(5)`). A guest whose root
lives in that home would start them as soon as root's own user manager
runs: a host timer would fire inside every guest, and a service that
keeps its data in your home would start a second time on the same files.

``home.nix`` mounts an empty, read-only `tmpfs`_ over
``~/.config/systemd`` and ``~/.local/share/systemd``, which hides them
from the guest and leaves the rest of the home shared. `systemd`_ mounts
each after the home, since its path lies below it
(:cmd:`systemd.mount(5)`). Being read-only, they also keep the guest from
writing units your own manager would then load.

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
.. _systemd: https://systemd.io
.. _services.openssh.authorizedKeysInHomedir: https://nixos.org/manual/nixos/stable/options#opt-services.openssh.authorizedKeysInHomedir
.. _cloud-init: https://cloudinit.readthedocs.io
