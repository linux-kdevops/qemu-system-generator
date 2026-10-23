.. SPDX-License-Identifier: copyleft-next-0.3.1

The generator
-------------

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
A save takes a moment to land, so every step below regenerates by hand
before starting the machine, which waits until the reload is done:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service

.. _use-xdg-base-directories: https://nix.dev/manual/nix/latest/command-ref/conf-file.html#conf-use-xdg-base-directories
