.. SPDX-License-Identifier: copyleft-next-0.3.1

============================
Bootstrap from a cloud image
============================

This guide boots the cloud images of `Debian`_ testing, `Fedora`_ 44 and
`Ubuntu`_ 26.10, unmodified, each a machine of its own under your user's
systemd manager. On first boot,
`cloud-init`_ reads a seed image that names the machine and lets your key
in for root. Each disk is the guest's own: what it writes stays across
restarts. No step needs root.

The three machines share most of their lines through a drop-in
directory, so each ``.qemu`` file holds only what differs. The file format
is specified in :doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` and
the generator in :doc:`qemu-system-generator(8) </man/qemu-system-generator.8>`.
The outputs below are real, trimmed to the lines that matter, with the
home directory shown as ``/home/user``.

.. include:: _prepare.rst

The images
----------

A relative path in a ``.qemu`` file resolves in the machine's state
directory, ``~/.local/state/qemu-system/<name>/`` under a user manager.
Download each image there with :cmd:`curl`, and check it against the
checksums its distribution publishes. Debian testing, which becomes
Debian 14, has daily builds:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/.local/state/qemu-system/cloud-debian
   $ cd ~/.local/state/qemu-system/cloud-debian
   $ curl --location --remote-name https://cloud.debian.org/images/cloud/forky/daily/latest/debian-14-genericcloud-amd64-daily.qcow2
   $ curl --silent --location https://cloud.debian.org/images/cloud/forky/daily/latest/SHA512SUMS | sha512sum --check --ignore-missing
   debian-14-genericcloud-amd64-daily.qcow2: OK

Fedora's checksum file is signed, so ``sha256sum`` skips and counts the
signature lines it cannot parse:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/.local/state/qemu-system/cloud-fedora
   $ cd ~/.local/state/qemu-system/cloud-fedora
   $ curl --location --remote-name https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2
   $ curl --silent --location https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/x86_64/images/Fedora-Cloud-44-1.7-x86_64-CHECKSUM | sha256sum --check --ignore-missing
   Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2: OK
   sha256sum: WARNING: 17 lines are improperly formatted

Ubuntu's ``current`` directory holds the latest build of 26.10. The image
is a qcow2 file despite its ``.img`` name:

.. code-block:: console
   :class: cmd-host

   $ mkdir --parents ~/.local/state/qemu-system/cloud-ubuntu
   $ cd ~/.local/state/qemu-system/cloud-ubuntu
   $ curl --location --remote-name https://cloud-images.ubuntu.com/stonking/current/stonking-server-cloudimg-amd64.img
   $ curl --silent --location https://cloud-images.ubuntu.com/stonking/current/SHA256SUMS | sha256sum --check --ignore-missing
   stonking-server-cloudimg-amd64.img: OK

The seeds
---------

cloud-init's `NoCloud`_ source reads its configuration from a file system
labelled ``cidata``. ``~/bootstrap/cloud/user-data`` holds what every
machine gets, with the public key from ``~/bootstrap/ssh/id_ed25519.pub``
pasted in:

.. code-block:: yaml
   :caption: ~/bootstrap/cloud/user-data

   #cloud-config
   disable_root: false
   ssh_authorized_keys:
     - ssh-ed25519 AAAA... bootstrap

The images let only their default user in by default, and
``disable_root: false`` gives root the key too. The meta-data names each
machine; cloud-init configures a machine once per ``instance-id``.
:cmd:`cloud-localds`, from nixpkgs' ``cloud-utils``, writes one seed into
each state directory:

.. code-block:: shell
   :class: cmd-host

   cd ~/bootstrap/cloud
   for name in cloud-debian cloud-fedora cloud-ubuntu; do
     printf 'instance-id: %s\nlocal-hostname: %s\n' $name $name > meta-data
     nix shell nixpkgs#cloud-utils --command \
       cloud-localds ~/.local/state/qemu-system/$name/seed.iso user-data meta-data
   done

.. include:: _generator.rst

.. _cloud-debian:

1. Debian testing
=================

The lines every machine shares go in a drop-in. A directory named after
the part of the name up to a dash, ``cloud-.qemu.d/``, applies to every
``cloud-*.qemu`` file, and its files are appended to each. Create
``~/.config/qemu/systemd/cloud-.qemu.d/10-cloud.conf``:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-.qemu.d/10-cloud.conf

   [VM]
   SshPrivateKey=%h/bootstrap/ssh/id_ed25519

   [QEMU]
   machine=q35
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio
   drive=file=seed.iso,if=virtio,format=raw,readonly=on
   nic=user,model=virtio-net-pci-non-transitional

``%h`` is your home directory, one of the `specifiers`_ systemd expands
in a unit. ``nic=`` gives the guest QEMU's `user-mode network`_, which
needs no privileges. Then create
``~/.config/qemu/systemd/cloud-debian.qemu``, with the disk and a `vsock`_
device whose context id is the machine's own:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-debian.qemu

   [QEMU]
   drive=file=debian-14-genericcloud-amd64-daily.qcow2,if=virtio,format=qcow2
   device=vhost-vsock-pci-non-transitional,guest-cid=43

The file's lines come first, so the disk is the first drive and the seed
the second. Each line of ``[QEMU]`` is one QEMU option with its dash
removed, passed on as written: ``m=2G`` is ``-m 2G``, and ``nographic=``
with an empty value is the bare flag ``-nographic``.
:cmd:`qemu-system-x86_64(1)` documents them all. The file is named after
the machine, and the unit after the file:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user start qemu-system@cloud-debian.service
   $ journalctl --user-unit=qemu-system@cloud-debian.service --follow

:cmd:`journalctl` shows the serial console. `SeaBIOS`_, QEMU's default
firmware, starts the image's GRUB, and cloud-init reads the seed:

.. code-block:: console
   :caption: guest
   :class: cmd-guest

   SeaBIOS (version rel-1.17.0-0-gb52ca86e094d-prebuilt.qemu.org)
   Booting from Hard Disk...
   GRUB loading.
   Welcome to GRUB!
     Booting `Debian GNU/Linux'
   ...
   Welcome to Debian GNU/Linux forky/sid!
   ...
   [    9.239455] cloud-init[420]: Cloud-init v. 26.2 finished at Fri, 23 Oct 2026 07:14:43 +0000. Datasource DataSourceNoCloud [seed=/dev/vdb].  Up 9.23 seconds
   ...
   Try contacting this VM's SSH server via 'ssh vsock%43' from host.

The generated unit registers each machine with your user's
:cmd:`systemd-machined.service(8)`, which records the vsock id, and the
guest's :cmd:`systemd-ssh-generator(8)` listens for SSH on vsock. Give the
machine a ``Host`` block:

.. code-block:: text
   :caption: ~/bootstrap/ssh/config.d/cloud-debian.conf

   Host cloud-debian
       HostName vsock/43
       ProxyCommand /usr/lib/systemd/systemd-ssh-proxy %h %p
       ProxyUseFdpass yes
       IdentityFile ~/bootstrap/ssh/id_ed25519
       IdentitiesOnly yes
       User root
       StrictHostKeyChecking accept-new
       UserKnownHostsFile /dev/null

.. include:: _ssh_host.rst

With the ``Include`` in your ``~/.ssh/config``, the machine's name is an
:cmd:`ssh` alias:

.. code-block:: console
   :class: cmd-host

   $ ssh cloud-debian 'hostname; uname -r'
   Warning: Permanently added 'vsock/43' (ED25519) to the list of known hosts.
   cloud-debian
   7.2.11+deb14-cloud-amd64

``[VM]`` holds what is not a QEMU option. With ``SshPrivateKey=``, stopping
the machine first runs ``systemctl poweroff`` in the guest over vsock, and
presses the power button only if that fails. The stop takes 3.6 seconds:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@cloud-debian.service

.. _cloud-fedora:

2. Fedora
=========

The drop-in already holds everything but the disk and the context id,
and the ``Host`` block differs in the same id:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-fedora.qemu

   [QEMU]
   drive=file=Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2,if=virtio,format=qcow2
   device=vhost-vsock-pci-non-transitional,guest-cid=44

.. code-block:: text
   :caption: ~/bootstrap/ssh/config.d/cloud-fedora.conf

   Host cloud-fedora
       HostName vsock/44
       ProxyCommand /usr/lib/systemd/systemd-ssh-proxy %h %p
       ProxyUseFdpass yes
       IdentityFile ~/bootstrap/ssh/id_ed25519
       IdentitiesOnly yes
       User root
       StrictHostKeyChecking accept-new
       UserKnownHostsFile /dev/null

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user start qemu-system@cloud-fedora.service
   $ ssh cloud-fedora 'hostname; uname -r'
   Warning: Permanently added 'vsock/44' (ED25519) to the list of known hosts.
   cloud-fedora
   6.19.10-300.fc44.x86_64

.. _cloud-ubuntu:

3. Ubuntu
=========

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-ubuntu.qemu

   [QEMU]
   drive=file=stonking-server-cloudimg-amd64.img,if=virtio,format=qcow2
   device=vhost-vsock-pci-non-transitional,guest-cid=45

.. code-block:: text
   :caption: ~/bootstrap/ssh/config.d/cloud-ubuntu.conf

   Host cloud-ubuntu
       HostName vsock/45
       ProxyCommand /usr/lib/systemd/systemd-ssh-proxy %h %p
       ProxyUseFdpass yes
       IdentityFile ~/bootstrap/ssh/id_ed25519
       IdentitiesOnly yes
       User root
       StrictHostKeyChecking accept-new
       UserKnownHostsFile /dev/null

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user start qemu-system@cloud-ubuntu.service
   $ ssh cloud-ubuntu 'hostname; uname -r'
   Warning: Permanently added 'vsock/45' (ED25519) to the list of known hosts.
   cloud-ubuntu
   7.3.0-9-generic

.. _cloud-all:

4. All three
============

Start them together and :cmd:`machinectl` lists them. Past the first
boot, each reaches SSH in 9 to 12 seconds:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system@cloud-debian.service qemu-system@cloud-fedora.service qemu-system@cloud-ubuntu.service
   $ machinectl --user list
   MACHINE      CLASS SERVICE     OS VERSION ADDRESSES
   cloud-debian vm    qemu-system -  -       -
   cloud-fedora vm    qemu-system -  -       -
   cloud-ubuntu vm    qemu-system -  -       -

The context ids must differ, since a host has one vsock address space. A
line in a machine's own file or drop-in directory, such as
``cloud-debian.qemu.d/``, applies to that machine only; a drop-in file
there with the name of one in ``cloud-.qemu.d/`` replaces it.

To start a guest over, stop it and download its image again; cloud-init
reads the seed on the fresh image's first boot.

.. _Debian: https://cloud.debian.org/images/cloud/
.. _Fedora: https://fedoraproject.org/cloud/download
.. _Ubuntu: https://cloud-images.ubuntu.com
.. _cloud-init: https://cloudinit.readthedocs.io
.. _NoCloud: https://cloudinit.readthedocs.io/en/latest/reference/datasources/nocloud.html
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _SeaBIOS: https://www.seabios.org/
