.. SPDX-License-Identifier: copyleft-next-0.3.1

=============================
A network between machines
=============================

Each machine's user-mode network belongs to its own QEMU process: the
guests reach the internet through it, but not each other. A second card
on QEMU's ``dgram`` network backend joins them, as an NFS server and its
client would need. The cloud, direct and knix guides join two machines
this way, with the lines below.

The segment
===========

When ``remote.host`` is a multicast address, every QEMU that names the
same group and port receives what the others send, so the cards share
one Ethernet segment, as if plugged into one bridge. It needs no root, no
TAP device and no daemon. Each machine gets the same ``netdev=`` line and
a card of its own:

.. code-block:: ini

   [QEMU]
   netdev=dgram,id=lan,remote.type=inet,remote.host=239.255.0.1,remote.port=5555,local.type=inet,local.host=127.0.0.1,local.port=0
   device=virtio-net-pci-non-transitional,netdev=lan,mac=52:54:00:00:00:43

``local.host=127.0.0.1`` keeps the segment on the host's loopback
interface; without it, the packets also leave through the interface
that routes ``239.255.0.1``, to every host on that link. QEMU requires
``local.port`` but does not use it, since the socket binds the group's
port. ``239.255.0.0/16`` is the local scope of `RFC 2365`_. Every
process on the host can join the group, so the segment is as private as
the host's loopback, no more.

The ``mac=`` must differ between the machines. Without one, each QEMU
gives its second card the same address, ``52:54:00:12:34:57``, since it
counts up from ``52:54:00:12:34:56`` per process. The guides take the
last byte from the machine's context id. A third machine joins with the
same ``netdev=`` line, a ``mac=`` and an address of its own. On microvm
the card is a ``virtio-net-device`` (:ref:`direct-microvm`).

.. _network-addresses:

Addresses
=========

Nothing on the segment hands out addresses, so each guest sets its own
from its card's MAC address, ``10.0.100.<context id>/24`` in the guides.

A cloud image gets them from cloud-init's `network configuration`_. It
replaces the configuration cloud-init would write itself, which covers
only the first card, so it has to name both; :ref:`cloud-network` writes
one per machine.

Two NixOS machines can boot one closure. The closure then holds both
addresses, as two `systemd-networkd`_ networks, and each card takes the
one that matches its MAC address. The ``10-`` names put them ahead of the
network that asks for every other card's address over DHCP. The
closure's hostname is the first machine's, and ``systemd.hostname=`` on
the second machine's kernel command line takes precedence over it
(:cmd:`kernel-command-line(7)`). :ref:`direct-peer` and :ref:`knix-peer`
do this.

.. _RFC 2365: https://www.rfc-editor.org/rfc/rfc2365
.. _network configuration: https://cloudinit.readthedocs.io/en/latest/reference/network-config-format-v2.html
.. _systemd-networkd: https://www.freedesktop.org/software/systemd/man/latest/systemd-networkd.service.html
