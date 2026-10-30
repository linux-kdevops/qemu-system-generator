<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Cover more of the generator, and of kdevops' guests, in CI

Status: planned, 2026-10-30. Nothing is built; this lists what no DAG
exercises yet, in the order to take it.

## Today

The guides' DAGs (`disk`, `uki`, `direct`, `knix`, `cloud`) boot their
machines under the user manager. `nested` installs the generator for
the system manager in a guest, and `runner` runs the CI workflow in a
guest made like ubuntu-latest. `nvme` boots ten NVMe controllers, each
set up its own way, `virtfs` a guest whose store and shares all come
over 9p, and `test` runs `cargo test`. Machines talk to each other only
through `-netdev dgram` on a UDP multicast group (direct, knix, cloud),
one `ping` each way.

The first section below takes what `qemu-systemd.unit(5)` specifies and
no DAG reaches. The second takes what kdevops (`linux-kdevops/kdevops`)
gives its libvirt and qemu-system-units guests and these DAGs lack, as
its `playbooks/roles/gen_nodes/templates/drives.j2`,
`guestfs_q35.j2.xml` and `kconfigs/Kconfig.libvirt` render them.

## The generator's own logic

1. **Stopping.** On a machine with `SSHPrivateKey=`, `systemctl stop`
   powers the guest off over SSH on vsock, and the journal shows no QMP
   fallback. A second machine, without the key, whose `systemd-logind`
   has `HandlePowerKey=ignore`, is killed after a `TimeoutStopSec=` its
   file shortens in `[Service]`. An empty `ExecStop=` followed by one of
   the file's own replaces the generated commands. Runs on the GitHub
   runner.
2. **`[Install]` under the user manager.** `WantedBy=machines.target`
   starts the machine through `systemctl --user start machines.target`,
   `Alias=vm@.service` makes it reachable as `vm@<name>.service`,
   `RequiredBy=` and `UpheldBy=` become links in the generator's output
   directory, and a value the generator cannot link is reported and
   skipped. `nested` covers `WantedBy=` for the system manager only.
3. **VFIO behind a vIOMMU.** An outer guest with
   `intel-iommu,intremap=on,caching-mode=on` binds an emulated device to
   vfio-pci and passes it to an inner machine. In the system manager it
   checks the generated `DeviceAllow=` for the group nodes (and
   `/dev/iommu` with the iommufd backend) and `LimitMEMLOCK=infinity`.
   Under a user manager the same machine shows the 8 MiB
   `DefaultLimitMEMLOCK=` the man page warns of. Builds on `nested`.
   Open: which emulated device the inner guest can drive once passed
   through.
4. **Two machines, ordered.** A server machine listens with
   `netdev=stream` on a unix socket in its runtime directory; the
   client connects, its `[Unit]` holding
   `Requires=qemu-system@<server>.service` and `After=` the same, and
   the two ping. Checks that the `[Unit]` lines are copied. Runs on the
   GitHub runner, with no root.

## kdevops' guests

1. **PCIe topology.** Put the `nvme` DAG's controllers behind
   `pcie-root-port`s on a `pxb-pcie` bus and check each controller's
   path in the guest's sysfs. A file may declare its own ports today;
   only the virtiofs devices the generator adds need
   `notes/plans/pcie-root-ports.md`.
2. **Large blocks.** Add namespaces with 16 KiB and 64 KiB logical
   blocks to the `nvme` DAG, the sizes kdevops' largeio sweeps. The
   check shows whether the guest kernel takes blocks above its page
   size.
3. **Other storage buses.** virtio-blk with an `iothread` and its own
   block sizes, `virtio-scsi-pci` with `scsi-hd`, and `ide-hd`, under
   `aio=threads` and the `cache=` modes the `nvme` DAG leaves out
   (writethrough, writeback, directsync, unsafe). Open: a DAG of its
   own, or the `nvme` DAG grown into a storage DAG.
4. **Bridge with tap.** Two machines on a bridge through tap and
   vhost-net check the generated `DeviceAllow=` for `/dev/net/tun` and
   `/dev/vhost-net`. Making the bridge needs root, and steps may not use
   sudo on the host, so it runs in the `nested` guest's system manager.
5. **CXL.** `machine=q35,cxl=on`, a `pxb-cxl` with a `cxl-rp` and a
   `cxl-type3` over `memory-backend-file,share=on`, and the
   `cxl-fmw.0.*` properties on a second `machine=` line, as QEMU's
   `docs/system/devices/cxl.rst` shows; `cxl list` in the guest. It
   checks how QEMU merges those `machine=` lines with the generator's
   own `memory-backend=` next to a virtiofs share. kdevops has this
   topology only in code nothing calls (`scripts/workflows/cxl/`).
6. **A guest that crashes.** `panic=-1` on the kernel command line and
   `no-reboot=`, then sysrq-c in the guest: the panic shows in
   `journalctl --user --unit`, the unit stops on its own, and
   `steps/machine/stop` still passes on it. With `vmcoreinfo`, a QMP
   `dump-guest-memory` leaves a dump in the state directory.

## Small additions to DAGs that exist

1. Sockets under `%t/qemu-system/%i/`: a `-gdb unix:` stub, the guest
   agent's `virtserialport`, a `virtconsole`; reached from the host with
   `socat`, which shows that `%t` and `%i` expand in `[QEMU]` lines.
2. A forwarded SSH port, `hostfwd=tcp:127.0.0.1:N-:22`, the man page's
   first example; no step uses one.
3. The path unit: write a `.qemu` file and wait for its unit without
   `steps/generator/run`, which every DAG uses to restart the generator
   by hand.
4. Binaries: `QEMUBinary=` as a bare file name and as a `%h` path, and
   `VirtiofsdBinary=`.
5. Redeploy: a new closure and its drop-in while the machine runs. It
   keeps its kernel until restarted, then boots the new one.
6. A broken file: one `.qemu` the generator rejects and the others'
   units still load; a `[QEMU]` line QEMU rejects fails its unit with
   QEMU's message in the journal.

## Research

1. arm64: `machine=virt` with AAVMF and GICv3 under TCG on the x86
   runner, or natively on an arm64 runner, which checks the compiled
   default `qemu-system-<host arch>`. Whether GitHub's arm64 runners
   have `/dev/kvm` is unchecked. kdevops gives its virt guests a TPM
   through swtpm, a daemon the generator cannot start: that would be a
   new helper section, not a test.
2. Live migration: a second machine started with
   `incoming=unix:%t/...` and a QMP `migrate` to it.
