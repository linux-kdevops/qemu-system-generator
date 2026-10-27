.. SPDX-License-Identifier: copyleft-next-0.3.1

============
Installation
============

The generator installs for one user without root, for the whole host
from a build of its own, or through a NixOS module. Each starts from a
checkout of `this repository`_:

.. code-block:: console
   :caption: host
   :class: cmd-host

   $ git clone https://github.com/linux-kdevops/qemu-system-generator
   $ cd qemu-system-generator

.. _install-user:

For your user
=============

Add it to your Nix profile with :cmd:`nix profile add`, from the checkout:

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

For the whole host
==================

A build installed under ``/usr/lib/systemd/`` runs as the generator of
both the system manager and every user manager, at boot and on each
``daemon-reload``, with no user units. Without Nix, it needs `Cargo`_ to
build and the distribution's QEMU, virtiofsd and ``socat`` to run: the
units call ``/usr/bin/qemu-system-<arch>``, ``/usr/libexec/virtiofsd``
and ``socat`` unless the build sets other paths (``build.rs``).

.. code-block:: console
   :class: cmd-host

   $ cargo build --release
   $ sudo install --mode=0755 target/release/qemu-system-generator /usr/libexec/
   $ sudo ln --symbolic /usr/libexec/qemu-system-generator /usr/lib/systemd/system-generators/
   $ sudo ln --symbolic /usr/libexec/qemu-system-generator /usr/lib/systemd/user-generators/
   $ sudo mkdir --parents /usr/share/qemu/systemd
   $ sudo install --mode=0644 data/qmp-powerdown /usr/share/qemu/systemd/

The system manager reads ``.qemu`` files from ``/etc/qemu/systemd/``, and a
user's manager from ``~/.config/qemu/systemd/``;
:doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` has the full search
path.

On NixOS
========

The flake's `NixOS`_ module installs the generator for the system manager
and every user manager. Add the flake as an input of your system's flake,
add ``qemu-system-generator.nixosModules.default`` to its modules, and
enable it:

.. code-block:: nix
   :caption: configuration.nix

   {
     virtualisation.qemu-system-generator.enable = true;
   }

``units`` installs ``.qemu`` files under ``/etc/qemu/systemd/`` from the
configuration, by name, as text or as a path.

.. _this repository: https://github.com/linux-kdevops/qemu-system-generator
.. _Cargo: https://doc.rust-lang.org/cargo/
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _use-xdg-base-directories: https://nix.dev/manual/nix/latest/command-ref/conf-file.html#conf-use-xdg-base-directories
