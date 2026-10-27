.. SPDX-License-Identifier: copyleft-next-0.3.1

============
Installation
============

The generator is built with `Nix`_ and installs for one user without root,
for the whole host, or through a `NixOS`_ module. Each starts from a
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

The profile puts ``qemu-system-generator`` on ``PATH``. `systemd`_ runs user
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

:cmd:`systemctl` links the units by the path it is given, not by the store
path behind it. After
:cmd:`nix profile upgrade qemu-system-generator <nix profile upgrade>`, a
``systemctl --user daemon-reload`` loads the new generation's units.

The service turns every ``.qemu`` file in ``~/.config/qemu/systemd/`` into
units and reloads the manager; it runs at login, and the
:cmd:`systemd.path(5)` unit runs it again whenever that directory changes.

For the whole host
==================

On a distribution other than NixOS, the same build serves the system
manager and every user manager. systemd runs a generator linked from
``/etc/systemd/system-generators/`` or ``/etc/systemd/user-generators/``
at boot and on each ``daemon-reload`` (:cmd:`systemd.generator(7)`), and
the build ships a link for each. Build it as yourself; then, as root,
point a profile at the build and link both generators from it.

Root needs the experimental features too. Nix reads ``/etc/nix/nix.conf``
and the configuration directory of the user who runs it (`configuration
file`_), and ``sudo`` sets ``HOME`` to root's, so the line in your own
``~/.config/nix/nix.conf`` does not reach root. Enable them for every
user:

.. code-block:: console
   :class: cmd-host

   $ echo 'experimental-features = nix-command flakes' \
       | sudo tee --append /etc/nix/nix.conf

Then build and install:

.. code-block:: console
   :class: cmd-host

   $ out=$(nix build --print-out-paths --no-link)
   $ sudo "$(command -v nix)" build \
       --profile /nix/var/nix/profiles/qemu-system-generator "$out"
   $ sudo mkdir --parents /etc/systemd/system-generators /etc/systemd/user-generators
   $ sudo ln --symbolic --force \
       /nix/var/nix/profiles/qemu-system-generator/lib/systemd/system-generators/qemu-system-generator \
       /etc/systemd/system-generators/
   $ sudo ln --symbolic --force \
       /nix/var/nix/profiles/qemu-system-generator/lib/systemd/user-generators/qemu-system-generator \
       /etc/systemd/user-generators/
   $ sudo systemctl daemon-reload

``sudo`` needs the full path to ``nix`` because its ``PATH`` leaves out
the Nix profile. The profile keeps the build from garbage collection, and
running the first two commands again moves it to a newer build. A user
manager picks the generator up at its next ``daemon-reload`` or login.
Leave out the user units of :ref:`install-user`, or every machine's units
are generated twice.

At boot, systemd runs generators before it mounts most file systems
(:cmd:`systemd.generator(7)`), so ``/nix`` must be on the root file system,
as the Nix installer puts it.

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
.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _configuration file: https://nix.dev/manual/nix/latest/command-ref/conf-file.html#configuration-file
.. _use-xdg-base-directories: https://nix.dev/manual/nix/latest/command-ref/conf-file.html#conf-use-xdg-base-directories
.. _Nix: https://nix.dev/manual/nix/latest/
.. _systemd: https://systemd.io
