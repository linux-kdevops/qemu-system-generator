.. SPDX-License-Identifier: copyleft-next-0.3.1

============================
Bootstrap from a cloud image
============================

This guide boots the cloud image of `Debian`_ testing, `Fedora`_ 44 or
`Ubuntu`_ 26.10, unmodified, as a machine under your user's `systemd`_
manager; the tabs follow the distribution you pick. On first boot,
`cloud-init`_ reads a seed image that names the machine and lets your key
in for root. Each disk is the guest's own: what it writes stays across
restarts.

Every ``cloud-*`` machine shares most of its lines through a drop-in
directory, so its :key:`.qemu` file holds only what differs. The file format
is specified in :doc:`qemu-systemd.unit(5) </man/qemu-systemd.unit.5>` and
the generator in :doc:`qemu-system-generator(8) </man/qemu-system-generator.8>`.
The :doc:`/getting-started/quickstart` boots the same machine without a
seed; carry on from it at :ref:`cloud-seeds`.

Prepare the host as :doc:`host` shows.

.. _cloud-images:

The images
==========

A relative path in a ``.qemu`` file resolves in the machine's state
directory, ``~/.local/state/qemu-system/<name>/`` under a user manager.
Download the image there with :cmd:`curl`, and check it against the
checksums its distribution publishes:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      Debian testing, which becomes Debian 14, has daily builds:

      .. code-block:: console
         :class: cmd-host

         $ curl --location --remote-name --create-dirs --output-dir ~/.local/state/qemu-system/cloud-debian \
             https://cloud.debian.org/images/cloud/forky/daily/latest/debian-14-genericcloud-amd64-daily.qcow2
         $ cd ~/.local/state/qemu-system/cloud-debian
         $ curl --silent --location https://cloud.debian.org/images/cloud/forky/daily/latest/SHA512SUMS | sha512sum --check --ignore-missing
         debian-14-genericcloud-amd64-daily.qcow2: OK

   .. tab-item:: Fedora
      :sync: fedora

      Fedora's checksum file is signed:

      .. code-block:: console
         :class: cmd-host

         $ curl --location --remote-name --create-dirs --output-dir ~/.local/state/qemu-system/cloud-fedora \
             https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2
         $ cd ~/.local/state/qemu-system/cloud-fedora
         $ curl --silent --location https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/x86_64/images/Fedora-Cloud-44-1.7-x86_64-CHECKSUM | sha256sum --check --ignore-missing
         Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2: OK
         sha256sum: WARNING: 17 lines are improperly formatted

   .. tab-item:: Ubuntu
      :sync: ubuntu

      Ubuntu's ``current`` directory holds the latest build of 26.10. The
      image is a qcow2 file despite its ``.img`` name:

      .. code-block:: console
         :class: cmd-host

         $ curl --location --remote-name --create-dirs --output-dir ~/.local/state/qemu-system/cloud-ubuntu \
             https://cloud-images.ubuntu.com/stonking/current/stonking-server-cloudimg-amd64.img
         $ cd ~/.local/state/qemu-system/cloud-ubuntu
         $ curl --silent --location https://cloud-images.ubuntu.com/stonking/current/SHA256SUMS | sha256sum --check --ignore-missing
         stonking-server-cloudimg-amd64.img: OK

.. _cloud-seeds:

The seeds
=========

Each machine reads its seed as a `NoCloud`_ source.
``~/bootstrap/cloud/user-data`` holds what every machine gets, with the
public key from ``~/bootstrap/system/ssh/id_ed25519.pub`` pasted in:

.. code-block:: yaml
   :caption: ~/bootstrap/cloud/user-data

   #cloud-config
   disable_root: false
   ssh_authorized_keys:
     - ssh-ed25519 AAAA... bootstrap # from ~/bootstrap/system/ssh/id_ed25519.pub

``disable_root: false`` gives root the key, and cloud-init's `module
reference`_ documents each key. Each machine's meta-data names it, and
:cmd:`cloud-localds` writes its seed into its state directory:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-debian.meta-data

         instance-id: cloud-debian
         local-hostname: cloud-debian

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds ~/.local/state/qemu-system/cloud-debian/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-debian.meta-data

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-fedora.meta-data

         instance-id: cloud-fedora
         local-hostname: cloud-fedora

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds ~/.local/state/qemu-system/cloud-fedora/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-fedora.meta-data

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-ubuntu.meta-data

         instance-id: cloud-ubuntu
         local-hostname: cloud-ubuntu

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds ~/.local/state/qemu-system/cloud-ubuntu/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-ubuntu.meta-data

.. _cloud-first:

1. The machine
==============

The lines every machine shares go in a drop-in. A directory named after
the part of the name up to a dash, ``cloud-.qemu.d/``, applies to every
``cloud-*.qemu`` file, and its files are appended to each. Create
``~/.config/qemu/systemd/cloud-.qemu.d/10-cloud.conf``:

.. code-block:: ini
   :caption: ~/.config/qemu/systemd/cloud-.qemu.d/10-cloud.conf

   [VM]
   SshPrivateKey=%h/bootstrap/system/ssh/id_ed25519

   [QEMU]
   machine=q35
   accel=kvm
   cpu=host
   m=2G
   smp=2
   nographic=
   serial=mon:stdio
   drive=file=seed.iso,if=virtio,format=raw,readonly=on
   netdev=user,id=net0
   device=virtio-net-pci-non-transitional,netdev=net0,romfile=

``%h`` is your home directory (`specifiers`_). ``netdev=`` gives the guest
`QEMU`_'s `user-mode network`_ through the card the ``device=`` after it
adds; ``romfile=`` with no value leaves out the card's iPXE boot ROM,
which the firmware would otherwise load. The file keeps the devices QEMU
adds on its own: without the VGA card, Debian's kernel resets the
machine right after GRUB's ``Loading Linux``. The machine's own file
keeps only its disk and a `vsock`_ device whose context id is the
machine's own, as :ref:`host-ssh` names it; it replaces the quick start's
file:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-debian.qemu

         [QEMU]
         drive=file=debian-14-genericcloud-amd64-daily.qcow2,if=virtio,format=qcow2
         device=vhost-vsock-pci-non-transitional,guest-cid=43

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-fedora.qemu

         [QEMU]
         drive=file=Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2,if=virtio,format=qcow2
         device=vhost-vsock-pci-non-transitional,guest-cid=44

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-ubuntu.qemu

         [QEMU]
         drive=file=stonking-server-cloudimg-amd64.img,if=virtio,format=qcow2
         device=vhost-vsock-pci-non-transitional,guest-cid=45

The file's lines come first, so the disk is the first drive and the seed
the second. Each line of :key:`[QEMU]` is one QEMU option with its dash
removed, passed on as written: ``m=2G`` is ``-m 2G``, and ``nographic=``
with an empty value is the bare flag ``-nographic``.
:cmd:`qemu-system-x86_64(1)` documents them all. The file is named after
the machine, and the unit after the file:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: console
         :class: cmd-host

         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user start qemu-system@cloud-debian.service
         $ journalctl --user-unit=qemu-system@cloud-debian.service --follow

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: console
         :class: cmd-host

         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user start qemu-system@cloud-fedora.service
         $ journalctl --user-unit=qemu-system@cloud-fedora.service --follow

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: console
         :class: cmd-host

         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user start qemu-system@cloud-ubuntu.service
         $ journalctl --user-unit=qemu-system@cloud-ubuntu.service --follow

:cmd:`journalctl` shows the serial console. `SeaBIOS`_ starts the
image's `GRUB`_, and cloud-init reads the seed:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

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

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: console
         :caption: guest
         :class: cmd-guest

         SeaBIOS (version rel-1.17.0-0-gb52ca86e094d-prebuilt.qemu.org)
         Booting from Hard Disk...
         GRUB loading.......
         Welcome to GRUB!
           Booting `Fedora Linux (6.19.10-300.fc44.x86_64) 44 (Cloud Edition)'
         ...
         Welcome to Fedora Linux 44 (Cloud Edition)!
         ...
         [    7.195740] cloud-init[889]: Cloud-init v. 25.3 finished at Tue, 27 Oct 2026 15:21:32 +0000. Datasource DataSourceNoCloud [seed=/dev/vdb].  Up 7.19 seconds
         ...
         Try contacting this VM's SSH server via 'ssh vsock%44' from host.

   .. tab-item:: Ubuntu
      :sync: ubuntu

      Ubuntu's GRUB prints nothing to the console:

      .. code-block:: console
         :caption: guest
         :class: cmd-guest

         SeaBIOS (version rel-1.17.0-0-gb52ca86e094d-prebuilt.qemu.org)
         Booting from Hard Disk...
         ...
         Welcome to Ubuntu 26.10!
         ...
         [   11.260189] cloud-init[810]: Cloud-init v. 26.2-0ubuntu2 finished at Tue, 27 Oct 2026 15:20:59 +0000. Datasource DataSourceNoCloud [seed=/dev/vdb].  Up 11.25 seconds
         ...
         Try contacting this VM's SSH server via 'ssh vsock%45' from host.

The machine registers with :cmd:`machined <systemd-machined>`
(:ref:`host-machined`), which records the id:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: console
         :class: cmd-host

         $ machinectl --user list
         MACHINE      CLASS SERVICE     OS VERSION ADDRESSES
         cloud-debian vm    qemu-system -  -       -

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: console
         :class: cmd-host

         $ machinectl --user list
         MACHINE      CLASS SERVICE     OS VERSION ADDRESSES
         cloud-fedora vm    qemu-system -  -       -

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: console
         :class: cmd-host

         $ machinectl --user list
         MACHINE      CLASS SERVICE     OS VERSION ADDRESSES
         cloud-ubuntu vm    qemu-system -  -       -

The guest's :cmd:`systemd-ssh-generator(8)` listens for SSH on vsock.
:ref:`host-ssh` already names the machine by its context id, so its name
is an :cmd:`ssh` alias:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: console
         :class: cmd-host

         $ ssh cloud-debian 'hostname; uname --kernel-release'
         Warning: Permanently added 'vsock/43' (ED25519) to the list of known hosts.
         cloud-debian
         7.2.11+deb14-cloud-amd64

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: console
         :class: cmd-host

         $ ssh cloud-fedora 'hostname; uname --kernel-release'
         Warning: Permanently added 'vsock/44' (ED25519) to the list of known hosts.
         cloud-fedora
         6.19.10-300.fc44.x86_64

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: console
         :class: cmd-host

         $ ssh cloud-ubuntu 'hostname; uname --kernel-release'
         Warning: Permanently added 'vsock/45' (ED25519) to the list of known hosts.
         cloud-ubuntu
         7.3.0-9-generic

With :key:`SshPrivateKey=` from the drop-in, the stop powers the guest off
over SSH first (:ref:`SSH poweroff <host-poweroff>`):

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: console
         :class: cmd-host

         $ systemctl --user stop qemu-system@cloud-debian.service

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: console
         :class: cmd-host

         $ systemctl --user stop qemu-system@cloud-fedora.service

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: console
         :class: cmd-host

         $ systemctl --user stop qemu-system@cloud-ubuntu.service

To start a guest over, stop it and download its image again; cloud-init
reads the seed on the fresh image's first boot.

.. _cloud-network:

2. A network between two machines
=================================

A second machine of the same distribution, ``cloud-<distro>-peer``, joins
the first on a multicast segment, as :doc:`network` explains. It boots a
copy of the image of its own:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: console
         :class: cmd-host

         $ curl --location --remote-name --create-dirs --output-dir ~/.local/state/qemu-system/cloud-debian-peer \
             https://cloud.debian.org/images/cloud/forky/daily/latest/debian-14-genericcloud-amd64-daily.qcow2
         $ cd ~/.local/state/qemu-system/cloud-debian-peer
         $ curl --silent --location https://cloud.debian.org/images/cloud/forky/daily/latest/SHA512SUMS | sha512sum --check --ignore-missing
         debian-14-genericcloud-amd64-daily.qcow2: OK

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: console
         :class: cmd-host

         $ curl --location --remote-name --create-dirs --output-dir ~/.local/state/qemu-system/cloud-fedora-peer \
             https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2
         $ cd ~/.local/state/qemu-system/cloud-fedora-peer
         $ curl --silent --location https://download.fedoraproject.org/pub/fedora/linux/releases/44/Cloud/x86_64/images/Fedora-Cloud-44-1.7-x86_64-CHECKSUM | sha256sum --check --ignore-missing
         Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2: OK
         sha256sum: WARNING: 17 lines are improperly formatted

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: console
         :class: cmd-host

         $ curl --location --remote-name --create-dirs --output-dir ~/.local/state/qemu-system/cloud-ubuntu-peer \
             https://cloud-images.ubuntu.com/stonking/current/stonking-server-cloudimg-amd64.img
         $ cd ~/.local/state/qemu-system/cloud-ubuntu-peer
         $ curl --silent --location https://cloud-images.ubuntu.com/stonking/current/SHA256SUMS | sha256sum --check --ignore-missing
         stonking-server-cloudimg-amd64.img: OK

``cloud-.qemu.d/`` applies to the peer too, so its file holds only its
disk and context id, as the first machine's does:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-debian-peer.qemu

         [QEMU]
         drive=file=debian-14-genericcloud-amd64-daily.qcow2,if=virtio,format=qcow2
         device=vhost-vsock-pci-non-transitional,guest-cid=51

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-fedora-peer.qemu

         [QEMU]
         drive=file=Fedora-Cloud-Base-Generic-44-1.7.x86_64.qcow2,if=virtio,format=qcow2
         device=vhost-vsock-pci-non-transitional,guest-cid=52

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-ubuntu-peer.qemu

         [QEMU]
         drive=file=stonking-server-cloudimg-amd64.img,if=virtio,format=qcow2
         device=vhost-vsock-pci-non-transitional,guest-cid=53

Each machine gets its card in a drop-in directory of its own, whose lines
apply to that machine only:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-debian.qemu.d/20-lan.conf

         [QEMU]
         netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
         device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:43,romfile=

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-debian-peer.qemu.d/20-lan.conf

         [QEMU]
         netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
         device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:51,romfile=

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-fedora.qemu.d/20-lan.conf

         [QEMU]
         netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
         device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:44,romfile=

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-fedora-peer.qemu.d/20-lan.conf

         [QEMU]
         netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
         device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:52,romfile=

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-ubuntu.qemu.d/20-lan.conf

         [QEMU]
         netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
         device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:45,romfile=

      .. code-block:: ini
         :caption: ~/.config/qemu/systemd/cloud-ubuntu-peer.qemu.d/20-lan.conf

         [QEMU]
         netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
         device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:53,romfile=

cloud-init sets the addresses (:ref:`network-addresses`). Each
configuration names both cards by their MAC addresses; the user-mode
card keeps QEMU's first default address and asks for one over DHCP:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

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
         :caption: ~/bootstrap/cloud/cloud-debian-peer.network

         version: 2
         ethernets:
           user:
             match:
               macaddress: "52:54:00:12:34:56"
             dhcp4: true
           lan:
             match:
               macaddress: "52:54:00:00:00:51"
             addresses: [10.0.100.51/24]

   .. tab-item:: Fedora
      :sync: fedora

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

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-fedora-peer.network

         version: 2
         ethernets:
           user:
             match:
               macaddress: "52:54:00:12:34:56"
             dhcp4: true
           lan:
             match:
               macaddress: "52:54:00:00:00:52"
             addresses: [10.0.100.52/24]

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-ubuntu.network

         version: 2
         ethernets:
           user:
             match:
               macaddress: "52:54:00:12:34:56"
             dhcp4: true
           lan:
             match:
               macaddress: "52:54:00:00:00:45"
             addresses: [10.0.100.45/24]

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-ubuntu-peer.network

         version: 2
         ethernets:
           user:
             match:
               macaddress: "52:54:00:12:34:56"
             dhcp4: true
           lan:
             match:
               macaddress: "52:54:00:00:00:53"
             addresses: [10.0.100.53/24]

cloud-init reads the network configuration for a new ``instance-id``
only, so the first machine's meta-data gets a new one:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-debian.meta-data

         instance-id: cloud-debian-lan
         local-hostname: cloud-debian

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-debian-peer.meta-data

         instance-id: cloud-debian-peer
         local-hostname: cloud-debian-peer

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-fedora.meta-data

         instance-id: cloud-fedora-lan
         local-hostname: cloud-fedora

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-fedora-peer.meta-data

         instance-id: cloud-fedora-peer
         local-hostname: cloud-fedora-peer

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-ubuntu.meta-data

         instance-id: cloud-ubuntu-lan
         local-hostname: cloud-ubuntu

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-ubuntu-peer.meta-data

         instance-id: cloud-ubuntu-peer
         local-hostname: cloud-ubuntu-peer

Write both seeds, :ref:`regenerate <host-regenerate>`,
:ref:`start <host-start>` the first machine and the peer, and each
reaches the other:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-debian.network \
             ~/.local/state/qemu-system/cloud-debian/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-debian.meta-data
         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-debian-peer.network \
             ~/.local/state/qemu-system/cloud-debian-peer/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-debian-peer.meta-data
         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user start qemu-system@cloud-debian.service
         $ systemctl --user start qemu-system@cloud-debian-peer.service
         $ ssh cloud-debian ping -c 1 10.0.100.51
         PING 10.0.100.51 (10.0.100.51) 56(84) bytes of data.
         64 bytes from 10.0.100.51: icmp_seq=1 ttl=64 time=0.302 ms
         ...
         $ ssh cloud-debian-peer ping -c 1 10.0.100.43
         PING 10.0.100.43 (10.0.100.43) 56(84) bytes of data.
         64 bytes from 10.0.100.43: icmp_seq=1 ttl=64 time=0.193 ms
         ...

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-fedora.network \
             ~/.local/state/qemu-system/cloud-fedora/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-fedora.meta-data
         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-fedora-peer.network \
             ~/.local/state/qemu-system/cloud-fedora-peer/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-fedora-peer.meta-data
         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user start qemu-system@cloud-fedora.service
         $ systemctl --user start qemu-system@cloud-fedora-peer.service
         $ ssh cloud-fedora ping -c 1 10.0.100.52
         PING 10.0.100.52 (10.0.100.52) 56(84) bytes of data.
         64 bytes from 10.0.100.52: icmp_seq=1 ttl=64 time=0.353 ms
         ...
         $ ssh cloud-fedora-peer ping -c 1 10.0.100.44
         PING 10.0.100.44 (10.0.100.44) 56(84) bytes of data.
         64 bytes from 10.0.100.44: icmp_seq=1 ttl=64 time=0.160 ms
         ...

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-ubuntu.network \
             ~/.local/state/qemu-system/cloud-ubuntu/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-ubuntu.meta-data
         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-ubuntu-peer.network \
             ~/.local/state/qemu-system/cloud-ubuntu-peer/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-ubuntu-peer.meta-data
         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user start qemu-system@cloud-ubuntu.service
         $ systemctl --user start qemu-system@cloud-ubuntu-peer.service
         $ ssh cloud-ubuntu ping -c 1 10.0.100.53
         PING 10.0.100.53 (10.0.100.53) 56(84) bytes of data.
         64 bytes from 10.0.100.53: icmp_seq=1 ttl=64 time=0.321 ms
         ...
         $ ssh cloud-ubuntu-peer ping -c 1 10.0.100.45
         PING 10.0.100.45 (10.0.100.45) 56(84) bytes of data.
         64 bytes from 10.0.100.45: icmp_seq=1 ttl=64 time=0.192 ms
         ...

.. _cloud-home:

3. Your home
============

Share your home as :doc:`home` describes. Add its section to the drop-in:

.. code-block:: diff
   :caption: ~/.config/qemu/systemd/cloud-.qemu.d/10-cloud.conf

    drive=file=seed.iso,if=virtio,format=raw,readonly=on
    netdev=user,id=net0
    device=virtio-net-pci-non-transitional,netdev=net0,romfile=
   +
   +[virtiofsd]
   +shared-dir=%h
   +tag=home
   +sandbox=namespace
   +uid-map=:0:%U:1:
   +gid-map=:0:%G:1:

Mount it in the guest as root's home (:ref:`home-cloud`), with your home
in place of ``/home/user``:

.. code-block:: diff
   :caption: ~/bootstrap/cloud/user-data

    ssh_authorized_keys:
      - ssh-ed25519 AAAA... bootstrap # from ~/bootstrap/system/ssh/id_ed25519.pub
   +write_files:
   +  - path: /etc/fstab
   +    append: true
   +    content: |
   +      home /home/user virtiofs defaults 0 0
   +      none /home/user/.config/systemd tmpfs ro,nosuid,nodev,mode=0555 0 0
   +      none /home/user/.local/share/systemd tmpfs ro,nosuid,nodev,mode=0555 0 0
   +      /root/.ssh /home/user/.ssh none bind 0 0
   +runcmd:
   +  - [mount, --mkdir, /home/user]
   +  - [mount, --mkdir, /home/user/.config/systemd]
   +  - [mount, --mkdir, /home/user/.local/share/systemd]
   +  - [mount, --mkdir, /home/user/.ssh]
   +  - [sed, --in-place, --regexp-extended, 's|^(root:([^:]*:){4})/root:|\1/home/user:|', /etc/passwd]

``/etc/fstab`` mounts them at every boot, and ``runcmd`` on the first;
``sed`` sets root's home, since :cmd:`usermod` refuses while root runs
PID 1. cloud-init runs both once per ``instance-id``, so give the machine
a new one:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-debian.meta-data

         instance-id: cloud-debian-home
         local-hostname: cloud-debian

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-fedora.meta-data

         instance-id: cloud-fedora-home
         local-hostname: cloud-fedora

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: yaml
         :caption: ~/bootstrap/cloud/cloud-ubuntu.meta-data

         instance-id: cloud-ubuntu-home
         local-hostname: cloud-ubuntu

Write its seed again, with its network configuration, then
:ref:`regenerate <host-regenerate>` and :ref:`restart <host-start>`.
Root's home is now yours, with the two systemd directories and ``~/.ssh``
covered, and a file root writes there belongs to you on the host:

.. tab-set::
   :sync-group: distro

   .. tab-item:: Debian
      :sync: debian

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-debian.network \
             ~/.local/state/qemu-system/cloud-debian/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-debian.meta-data
         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user restart qemu-system@cloud-debian.service
         $ ssh cloud-debian 'pwd; findmnt --noheadings --output=TARGET,FSTYPE --submounts ~; touch ~/bootstrap/cloud/from-debian'
         Warning: Permanently added 'vsock/43' (ED25519) to the list of known hosts.
         /home/user
         /home/user                        virtiofs
         ├─/home/user/.config/systemd      tmpfs
         ├─/home/user/.local/share/systemd tmpfs
         └─/home/user/.ssh                 ext4
         $ ls --format=long ~/bootstrap/cloud/from-debian
         -rw-r--r-- 1 user user 0 Oct 27 16:22 /home/user/bootstrap/cloud/from-debian

   .. tab-item:: Fedora
      :sync: fedora

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-fedora.network \
             ~/.local/state/qemu-system/cloud-fedora/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-fedora.meta-data
         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user restart qemu-system@cloud-fedora.service
         $ ssh cloud-fedora 'pwd; findmnt --noheadings --output=TARGET,FSTYPE --submounts ~; touch ~/bootstrap/cloud/from-fedora'
         Warning: Permanently added 'vsock/44' (ED25519) to the list of known hosts.
         /home/user
         /home/user                        virtiofs
         ├─/home/user/.config/systemd      tmpfs
         ├─/home/user/.local/share/systemd tmpfs
         └─/home/user/.ssh                 btrfs
         $ ls --format=long ~/bootstrap/cloud/from-fedora
         -rw-r--r-- 1 user user 0 Oct 27 16:21 /home/user/bootstrap/cloud/from-fedora

   .. tab-item:: Ubuntu
      :sync: ubuntu

      .. code-block:: console
         :class: cmd-host

         $ nix shell nixpkgs#cloud-utils --command \
             cloud-localds --network-config ~/bootstrap/cloud/cloud-ubuntu.network \
             ~/.local/state/qemu-system/cloud-ubuntu/seed.iso \
             ~/bootstrap/cloud/user-data ~/bootstrap/cloud/cloud-ubuntu.meta-data
         $ systemctl --user start qemu-system-generator.service
         $ systemctl --user restart qemu-system@cloud-ubuntu.service
         $ ssh cloud-ubuntu 'pwd; findmnt --noheadings --output=TARGET,FSTYPE --submounts ~; touch ~/bootstrap/cloud/from-ubuntu'
         Warning: Permanently added 'vsock/45' (ED25519) to the list of known hosts.
         /home/user
         /home/user                        virtiofs
         ├─/home/user/.config/systemd      tmpfs
         ├─/home/user/.local/share/systemd tmpfs
         └─/home/user/.ssh                 ext4
         $ ls --format=long ~/bootstrap/cloud/from-ubuntu
         -rw-r--r-- 1 user user 0 Oct 27 16:21 /home/user/bootstrap/cloud/from-ubuntu

.. _Debian: https://cloud.debian.org/images/cloud/
.. _Fedora: https://fedoraproject.org/cloud/download
.. _Ubuntu: https://cloud-images.ubuntu.com
.. _cloud-init: https://cloudinit.readthedocs.io
.. _user-mode network: https://www.qemu.org/docs/master/system/devices/net.html#using-the-user-mode-network-stack
.. _systemd: https://systemd.io
.. _QEMU: https://www.qemu.org/docs/master/system/invocation.html
.. _GRUB: https://www.gnu.org/software/grub/manual/grub/grub.html
.. _SeaBIOS: https://www.seabios.org/
.. _vsock: https://man7.org/linux/man-pages/man7/vsock.7.html
.. _specifiers: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
.. _module reference: https://cloudinit.readthedocs.io/en/latest/reference/modules.html
.. _NoCloud: https://cloudinit.readthedocs.io/en/latest/reference/datasources/nocloud.html
