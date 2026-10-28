<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Put q35's virtio devices behind PCIe root ports

Status: research, 2026-10-28. Nothing is built; this is what was
measured and what the change would take.

## Today

Every q35 machine in the guides, the steps and the examples plugs its
virtio devices straight into the root bus, `pcie.0`: the cards, vsock
and serial a file names with `device=`, and the `vhost-user-fs-pci` the
generator adds for each `[virtiofsd]` section (`src/backend/qemu.rs`,
which picks `vhost-user-fs-device` on microvm instead). All of them are
non-transitional, virtio 1.0 only.

On the root bus QEMU makes such a device an integrated endpoint, and
`virtio_pci_realize()` in `hw/virtio/virtio-pci.c` adds the PCI Express
capability only when the bus is express and not a root bus:

    bool pcie_port = pci_bus_is_express(pci_get_bus(pci_dev)) &&
                     !pci_bus_is_root(pci_get_bus(pci_dev));

So the guest sees conventional PCI devices with no extended
configuration space. QEMU's `docs/pcie.txt` allows that (section 2.1:
"Place only legacy PCI devices on the Root Complex") but recommends
something else for PCI Express devices (section 2.2): "Always use PCI
Express Root Ports to start PCI Express hierarchies", and "Plug only PCI
Express devices into PCI Express Ports". Section 5 adds that nothing on
a root bus can be hot-plugged.

## The change

In a guide's file, one root port per device, each device naming its
port with `bus=`:

    device=pcie-root-port,id=rp1,chassis=1
    device=pcie-root-port,id=rp2,chassis=2
    netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10047-:22
    device=virtio-net-pci-non-transitional,netdev=net0,romfile=,bus=rp1
    device=vhost-vsock-pci-non-transitional,guest-cid=47,bus=rp2

The (chassis, slot) pair must differ between ports, and slot defaults to
0 (`pcie.txt`, section 2.2). Ports can also be grouped as functions of
one multi-function device (section 2.2.2) to save root bus slots.

The guides can only place the devices they write. The virtiofs devices
come from the generator, so on q35 it would emit a port of its own per
share and plug the device into it:

    -device pcie-root-port,id=<port id>,chassis=<chassis>,slot=<n>
    -device vhost-user-fs-pci,...,tag=<tag>,bus=<port id>

This is one more machine-dependent branch next to the microvm one. The
`.qemu` format does not change; the generated units and
`tests/expected/dryrun.txt` do.

## The open problem: whose chassis numbers

The generator's ports must not collide with the ports a file declares,
and the backend does not interpret `[QEMU]` lines beyond the few it
reads (`m=`, `machine=` or `M=`, `uuid=`, `device=` for what it
needs). Three ways out, none chosen:

1. Reserve a chassis for the generator (a high number, slots 1 to N for
   the shares) and document it in `qemu-systemd.unit(5)`. Simple, but a
   file that uses the same chassis fails in QEMU, not in the generator.
2. Read the file's `device=pcie-root-port` lines to find free numbers.
   Collision-free, but it widens what the decoder interprets.
3. Let the file name the port for a share, through a new key outside
   `[virtiofsd]`, whose keys stay virtiofsd's options verbatim. A format
   change.

## What it buys and costs

It buys the PCI Express capability and extended configuration space on
every device, and hotplug: `device_add` and `device_del` over the
machine's QMP socket at run time, which no root bus device allows.

Measured on the direct guide's fastest q35 machine (kernel 6.18.59,
middle of three boots, time from `systemctl --user start` to the first
`ssh` command), with one root port for each of the file's four devices
and the virtiofs devices still on `pcie.0`: 4.41 s against 4.38 s
without ports, within the run-to-run spread. The guest's kernel share
grew from 0.42 to 0.51 s.

It costs two lines per device in every guide and example, the generator
logic above, and a rewrite of every q35 entry in `dryrun.txt`.

## Why not now

No guide hot-plugs a device or needs a PCI Express only feature, so the
flat layout stays. Root ports become worth it when the lifecycle pages
document hotplug over QMP, or a guide needs a feature that only a PCI
Express endpoint has.

## Verifying it

- `qemu-system-generator --user --dryrun` shows a port per share and
  `bus=` on each `vhost-user-fs-pci`; `tests/update-expected` and review.
- In the guest, `lspci -vv` lists each virtio device with an
  `Express (v2) Endpoint` capability, and `findmnt --types=virtiofs`
  still lists every share.
- Every DAG passes, and the guides' times are measured again as
  `docs/bootstrap/fast.rst` describes.
