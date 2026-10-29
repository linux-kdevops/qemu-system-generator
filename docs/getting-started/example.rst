.. SPDX-License-Identifier: copyleft-next-0.3.1

=======
Example
=======

Boot a `NixOS`_ guest of your user's `systemd`_ manager on your own
`Linux`_ kernel, with the host's ``/nix/store``, the kernel's modules and
your home shared over `virtiofs`_. :doc:`/bootstrap/knix` builds a
machine like this one step at a time, explains each line, and in
:ref:`knix-fast` cuts its boot from 7.7 to 1.5 seconds.

Prepare the host as :doc:`/bootstrap/host` shows.

The kernel
==========

Merge the `kfragments`_ with everything a module but virtiofs: the guest
boots NixOS's own systemd `initrd`_, which `knix`_ builds with no modules
and which mounts the store over virtiofs. Without ``-y`` the last value
wins, so the ``builtin/`` fragment goes last. The toolchain comes from
knix's ``build-kernel`` shell (:cmd:`nix develop`):

.. code-block:: console
   :class: cmd-host

   $ git clone --depth 1 https://github.com/linux-kdevops/kfragments.git \
       ~/bootstrap/knix/kfragments
   $ git clone --depth 1 --branch v7.3 \
       https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git ~/bootstrap/knix/linux
   $ cd ~/bootstrap/knix/linux
   $ nix develop github:linux-kdevops/knix#build-kernel
   $ mkdir build
   $ scripts/kconfig/merge_config.sh -n -O build \
       ../kfragments/kernel/configs/core/64bit.config \
       ../kfragments/kernel/configs/core/core.config \
       ../kfragments/kernel/configs/core/modules.config \
       ../kfragments/kernel/configs/core/systemd.config \
       ../kfragments/kernel/configs/core/initrd.config \
       ../kfragments/kernel/configs/arch/x86_64.config \
       ../kfragments/kernel/configs/core/acpi-poweroff.config \
       ../kfragments/kernel/configs/virt/virtio-fs.config \
       ../kfragments/kernel/configs/virt/virtio-net.config \
       ../kfragments/kernel/configs/virt/vsock.config \
       ../kfragments/kernel/configs/core/localversion.config \
       ../kfragments/kernel/configs/builtin/virt/virtio-fs.config
   $ make O=build --jobs=$(nproc)
   $ make O=build INSTALL_MOD_PATH=../destdir modules_install
   $ exit

The guest
=========

Start from knix's template (:cmd:`nix flake init`) and point its input
at knix upstream:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/bootstrap/knix/guest
   $ cd ~/bootstrap/knix/guest
   $ nix flake init --template github:linux-kdevops/knix

.. code-block:: diff
   :caption: ~/bootstrap/knix/guest/flake.nix

   -    knix.url = "path:/path/to/knix";
   +    knix.url = "github:linux-kdevops/knix";

Share your home and mount it in the initrd, so that ``init=`` can reach
the closure through it. Put its path in place of ``/home/user``
(:doc:`/bootstrap/home` explains the masks):

.. code-block:: nix
   :caption: ~/bootstrap/knix/guest/home.nix

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
       neededForBoot = true;
     };
     fileSystems."${home}/.config/systemd" = mask;
     fileSystems."${home}/.local/share/systemd" = mask;
     fileSystems."${home}/.ssh" = mask;
     users.users.root.home = lib.mkForce home;
   }

Replace the template's ``default.nix`` and paste in the public key from
``~/bootstrap/system/ssh/id_ed25519.pub``:

.. code-block:: nix
   :caption: ~/bootstrap/knix/guest/default.nix

   { config, knix, ... }:
   {
     imports = [
       knix.nixosModules.profiles.devel
       ./home.nix
     ];

     networking.hostName = "bootstrap";

     users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
     users.users.${config.knix.user.name}.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
   }

Build the closure; ``result`` links to it, and its ``initrd`` and
``init`` with it:

.. code-block:: console
   :class: cmd-host

   $ nix build .#toplevel

The machine
===========

`QEMU`_ boots your kernel from ``build/`` and the initrd through
``result``, with the ``root=tmpfs`` knix wants. ``init=`` names
``result`` too, in your home, which the guest mounts at the same path,
so a rebuild of the guest changes nothing in this file:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/bootstrap-knix.qemu

   [VM]
   SSHPrivateKey=%h/bootstrap/system/ssh/id_ed25519

   [QEMU]
   machine=q35
   nodefaults=
   accel=kvm
   cpu=host
   m=2G
   smp=4
   nographic=
   serial=mon:stdio
   kernel=%h/bootstrap/knix/linux/build/arch/x86/boot/bzImage
   initrd=%h/bootstrap/knix/guest/result/initrd
   append=root=tmpfs console=ttyS0 init=%h/bootstrap/knix/guest/result/init
   netdev=user,id=net0
   device=virtio-net-pci-non-transitional,netdev=net0,romfile=
   device=vhost-vsock-pci-non-transitional,guest-cid=49

   [virtiofsd]
   shared-dir=/nix/store
   tag=store
   cache=always
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h/bootstrap/knix/linux/destdir/lib/modules
   tag=modules
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

   [virtiofsd]
   shared-dir=%h
   tag=home
   sandbox=namespace
   uid-map=:0:%U:1:
   gid-map=:0:%G:1:

Run it
======

:ref:`Regenerate <host-regenerate>`, start the machine and log in:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user start qemu-system@bootstrap-knix.service
   $ ssh bootstrap-knix \
       'uname --kernel-release; findmnt --noheadings --output=TARGET,SOURCE,FSTYPE --types=virtiofs'

After a kernel rebuild, install the modules again and
:ref:`restart <host-start>`; after a change to the guest, rebuild the
closure and restart.
:ref:`Kill <host-kill>` a guest that does not answer the stop, and
:ref:`remove <usage-remove>` the machine when you are done with it.

.. _NixOS: https://nixos.org/manual/nixos/stable/
.. _systemd: https://systemd.io
.. _Linux: https://www.kernel.org
.. _virtiofs: https://virtio-fs.gitlab.io
.. _kfragments: https://github.com/linux-kdevops/kfragments
.. _knix: https://github.com/linux-kdevops/knix
.. _initrd: https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
