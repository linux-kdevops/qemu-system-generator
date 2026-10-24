.. SPDX-License-Identifier: copyleft-next-0.3.1

============================
Bootstrap from a cloud image
============================

This guide boots the cloud images of `Debian`_ testing, `Fedora`_ 44 and
`Ubuntu`_ 26.10, unmodified, each a machine of its own under your user's
`systemd`_ manager. On first boot,
`cloud-init`_ reads a seed image that names the machine and lets your key
in for root. Each disk is the guest's own: what it writes stays across
restarts. No step needs root.

The three machines share most of their lines through a drop-in
directory, so each :key:`.qemu` file holds only what differs. The file format
is specified in :doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` and
the generator in :doc:`qemu-system-generator(8) </man/qemu-system-generator.8>`.
The outputs below are real, trimmed to the lines that matter, with the
home directory shown as ``/home/user``.

Prepare the host as :doc:`host` shows, once for every guide.

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

Fedora's checksum file is signed, so :cmd:`sha256sum` skips and counts the
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
:cmd:`cloud-localds`, from `nixpkgs`_' ``cloud-utils``, writes one seed into
each state directory:

.. code-block:: shell
   :class: cmd-host

   cd ~/bootstrap/cloud
   for name in cloud-debian cloud-fedora cloud-ubuntu; do
     printf 'instance-id: %s\nlocal-hostname: %s\n' $name $name > meta-data
     nix shell nixpkgs#cloud-utils --command \
       cloud-localds ~/.local/state/qemu-system/$name/seed.iso user-data meta-data
   done

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
in a unit. ``nic=`` gives the guest `QEMU`_'s `user-mode network`_, which
needs no privileges. Then create
``~/.config/qemu/systemd/cloud-debian.qemu``, with the disk and a vsock
device whose context id is the machine's own:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-debian.qemu

   [QEMU]
   drive=file=debian-14-genericcloud-amd64-daily.qcow2,if=virtio,format=qcow2
   device=vhost-vsock-pci-non-transitional,guest-cid=43

The file's lines come first, so the disk is the first drive and the seed
the second. Each line of :key:`[QEMU]` is one QEMU option with its dash
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
firmware, starts the image's `GRUB`_, and cloud-init reads the seed:

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

The guest's :cmd:`systemd-ssh-generator(8)` listens for SSH on vsock.
:ref:`host-ssh` already names the machine by its context id, so its name
is an :cmd:`ssh` alias:

.. code-block:: console
   :class: cmd-host

   $ ssh cloud-debian 'hostname; uname -r'
   Warning: Permanently added 'vsock/43' (ED25519) to the list of known hosts.
   cloud-debian
   7.2.11+deb14-cloud-amd64

With :key:`SshPrivateKey=` from the drop-in, the stop powers the guest off
over SSH first (:ref:`SSH poweroff <host-poweroff>`), in 3.6 seconds:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user stop qemu-system@cloud-debian.service

.. _cloud-fedora:

2. Fedora
=========

The drop-in already holds everything but the disk and the context id,
which :ref:`host-ssh` already names:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-fedora.qemu

   [QEMU]
   drive=file=Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2,if=virtio,format=qcow2
   device=vhost-vsock-pci-non-transitional,guest-cid=44

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

.. _cloud-network:

5. A network between two machines
=================================

A second card on a multicast segment joins cloud-debian and cloud-fedora,
as :doc:`network` explains. Give cloud-debian its card in a drop-in of
its own:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-debian.qemu.d/20-lan.conf

   [QEMU]
   netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
   device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:43

cloud-fedora's drop-in differs only in the ``mac=``:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-fedora.qemu.d/20-lan.conf

   [QEMU]
   netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
   device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:44

cloud-init sets the addresses (:ref:`network-addresses`). Each
configuration names both cards by their MAC addresses; the user-mode
card keeps QEMU's first default address and asks for one over DHCP:

.. code-block:: yaml
   :caption: ~/bootstrap/cloud/cloud-debian.network

   version: 2
   ethernets:
     user:
       match:
         macaddress: "52:54:00:12:34:56"
       dhcp4: true
     lan:
       match:
         macaddress: "52:54:00:00:00:43"
       addresses: [10.0.100.43/24]

.. code-block:: yaml
   :caption: ~/bootstrap/cloud/cloud-fedora.network

   version: 2
   ethernets:
     user:
       match:
         macaddress: "52:54:00:12:34:56"
       dhcp4: true
     lan:
       match:
         macaddress: "52:54:00:00:00:44"
       addresses: [10.0.100.44/24]

cloud-init reads the network configuration for a new ``instance-id``
only, so write both seeds again with one:

.. code-block:: shell
   :class: cmd-host

   cd ~/bootstrap/cloud
   for name in cloud-debian cloud-fedora; do
     printf 'instance-id: %s-lan\nlocal-hostname: %s\n' $name $name > meta-data
     nix shell nixpkgs#cloud-utils --command \
       cloud-localds --network-config=$name.network \
       ~/.local/state/qemu-system/$name/seed.iso user-data meta-data
   done

Regenerate and restart both, and each reaches the other:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user start qemu-system-generator.service
   $ systemctl --user restart qemu-system@cloud-debian.service qemu-system@cloud-fedora.service
   $ ssh cloud-debian ping -c 1 10.0.100.44
   PING 10.0.100.44 (10.0.100.44) 56(84) bytes of data.
   64 bytes from 10.0.100.44: icmp_seq=1 ttl=64 time=0.337 ms
   ...
   $ ssh cloud-fedora ping -c 1 10.0.100.43
   PING 10.0.100.43 (10.0.100.43) 56(84) bytes of data.
   64 bytes from 10.0.100.43: icmp_seq=1 ttl=64 time=0.161 ms
   ...

.. _Debian: https://cloud.debian.org/images/cloud/
.. _Fedora: https://fedoraproject.org/cloud/download
.. _Ubuntu: https://cloud-images.ubuntu.com
.. _cloud-init: https://cloudinit.readthedocs.io
.. _NoCloud: https://cloudinit.readthedocs.io/en/latest/reference/datasources/nocloud.html
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _systemd: https://systemd.io
.. _nixpkgs: https://github.com/NixOS/nixpkgs
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _GRUB: https://www.gnu.org/software/grub/manual/grub/grub.html
.. _SeaBIOS: https://www.seabios.org/
