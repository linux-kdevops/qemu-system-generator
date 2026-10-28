.. SPDX-License-Identifier: copyleft-next-0.3.1

=============
A faster boot
=============

.. list-table::
   :header-rows: 1

   * - Guide
     - As built
     - Fastest
   * - :ref:`disk <disk-fast>`
     - 12.4 s
     - 4.5 s
   * - :ref:`uki <uki-fast>`
     - 8.7 s
     - 5.3 s
   * - :ref:`direct <direct-fast>`
     - 8.5 s
     - 4.5 s
   * - :ref:`knix <knix-fast>`
     - 7.7 s
     - 1.5 s

.. _fast-measure:

Measuring
=========

The time that matters is from starting the unit to the first command that
runs over :cmd:`ssh`. :cmd:`systemd-analyze(1)` in the guest splits the
guest's part into kernel, `initrd`_ and userspace, but it cannot see
`QEMU`_ or the firmware, so time the whole of it from the host. Stop the
machine first, ``bootstrap-disk`` here. The ``timeout`` covers a `vsock`_
connection that does not answer yet:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@bootstrap-disk.service
   $ time sh -c 'systemctl --user start qemu-system@bootstrap-disk.service; until timeout 2 ssh bootstrap-disk true; do sleep 0.1; done' 2>/dev/null

Each time in the guides is the middle of three boots on the host they
were written on, an AMD Ryzen 7 PRO 8700GE. Each step keeps the ones
before it.

.. _fast-console:

The console
===========

Before it prints anything, `systemd`_ asks the console which terminal it is
and how large, with ANSI queries, and waits 333 milliseconds for each
answer (``CONSOLE_ANSI_SEQUENCE_TIMEOUT_USEC`` in systemd's
``src/basic/terminal-util.c``). A serial console that goes to the journal
never answers. The initrd's systemd waits out a query for the terminal's
name and two for its size, and the system's systemd waits again after the
switch to the real root. Three settings on the kernel command line answer
them in advance (:cmd:`kernel-command-line(7)`): ``systemd.tty.term.console=``
names the terminal, and ``systemd.tty.rows.console=`` and
``systemd.tty.columns.console=`` give its size:

.. code-block:: text

   systemd.tty.term.console=vt220 systemd.tty.rows.console=24 systemd.tty.columns.console=80

.. _fast-store:

The store
=========

By default `virtiofsd`_ lets the guest cache a file only until it opens it
again, so every lookup in the store goes back to the host. The store
never changes under a path once written, which is the case ``cache=always``
is for: the guest keeps what it read (``CachePolicy`` in virtiofsd's
``src/passthrough/mod.rs``). Set it on the store's share only. A
directory you change from the host while the guest runs, such as your
home, keeps the default.

.. _fast-closure:

The closure
===========

The guests forget everything when they stop, so every boot is a first
boot, and four things on the way to a login are first-boot work or not
needed here:

- `NixOS`_ generates an RSA key of 4096 bits and an Ed25519 key for
  :cmd:`sshd <sshd(8)>` (``services.openssh.hostKeys``), and the first
  login waits for ``sshd-keygen.service``. The RSA key takes most of the
  time, and the configuration from :ref:`host-ssh` accepts any host key.
  Keep Ed25519 only.
- ``systemd-journal-catalog-update.service`` and
  ``systemd-update-done.service`` run when ``/var`` is newer than its
  last update (``ConditionNeedsUpdate=``), which a fresh ``/var`` always
  is, and ``sysinit.target`` waits for them. ``systemd.suppressedSystemUnits``
  leaves them out.
- sshd's PAM stack registers each login with
  :cmd:`logind <systemd-logind.service(8)>`, and the first registration
  starts root's user manager, ``user@0.service``, before the command runs.
  ``startSession = false`` skips that; logins then have no logind session,
  so :cmd:`loginctl` does not list them. The sshd module sets it, hence
  ``lib.mkForce``.
- ``systemd-vconsole-setup.service`` sets the keyboard map and font of
  virtual terminals, which these guests do not show. ``console.enable =
  false`` leaves it out.

All four, in a NixOS module:

.. code-block:: nix

   services.openssh.hostKeys = [
     { type = "ed25519"; path = "/etc/ssh/ssh_host_ed25519_key"; }
   ];
   security.pam.services.sshd.startSession = lib.mkForce false;
   console.enable = false;
   systemd.suppressedSystemUnits = [
     "systemd-journal-catalog-update.service"
     "systemd-update-done.service"
   ];

.. _fast-processors:

More processors
===============

systemd starts what it can in parallel, so ``smp=4`` instead of two
shortens the boot, by 0.05 to 0.12 seconds in the guides. Eight gained
nothing more in the `knix`_ guide.

.. _fast-quiet:

Quiet
=====

Every line on the serial console costs time. ``quiet`` turns off the
kernel's log output and systemd's status output (:cmd:`systemd(1)`). The
journal then shows the boot only when something fails, and
``journalctl --boot`` in the guest still has every line. Leave it out
while a boot needs watching.

.. _fast-mounts:

The mount rate limit
====================

systemd holds back starting a mount while more than five mount events a
second arrive (``mount_rate_limit_burst`` in systemd's
``src/core/mount.c``), and early boot mounts more than five file systems;
the initrd waits about 0.8 seconds before it mounts ``/sysroot``.
``SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST=`` raises the limit. The kernel
passes a ``key=value`` parameter it does not know to init as an
environment variable (``unknown_bootoption()`` in the kernel's
``init/main.c``), and systemd keeps it across the switch to the real root.

It is safe only with `virtiofs`_ built into the kernel. With the `nixpkgs`_
kernel, whose virtiofs is a module, the store's mount then runs before
the ``virtiofs`` module has found the ``store`` tag, and the guest stops
in emergency mode. :ref:`knix-fast` builds a kernel with virtiofs built
in and lifts the limit.

.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _systemd: https://systemd.io
.. _virtiofsd: https://gitlab.com/virtio-fs/virtiofsd
.. _knix: https://github.com/linux-kdevops/knix
.. _virtiofs: https://virtio-fs.gitlab.io
.. _nixpkgs: https://github.com/NixOS/nixpkgs
