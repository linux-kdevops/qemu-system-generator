<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Split the knix steps per machine

Status: planned, 2026-10-28.

`steps/direct/install` and `steps/direct/build` take `--name VM` and
write one machine each, from a `case` that holds the machine's context
id and processor count; `dags/direct.yaml` calls each step once per
machine. knix's steps still write both machines at once:

- `steps/knix/install` loops over `bootstrap-knix` and
  `bootstrap-knix-peer`, packing each machine's context id, processors,
  kernel and firmware into one colon-separated word.
- `steps/knix/build` builds what the machines share (both kernels,
  QEMU, BusyBox, the guest's closure and the BusyBox initramfs), then
  writes both drop-ins: the BusyBox one with the closure's store path,
  and the peer's, with links to the closure's initrd and init, from
  `steps/knix/files/peer-closure.conf`.

The split, as direct has it:

- `steps/knix/install --name VM` writes the one machine's `.qemu` from a
  `case` like direct's; the guest flake and the initramfs's `init`,
  which both machines share, stay in a step without `--name`.
- The shared builds become a step of their own, and
  `steps/knix/build --name VM` writes only that machine's drop-in, from
  a template under `steps/knix/files/` filled by `envsubst`, as
  `10-closure.conf` and `peer-closure.conf` already are.
- `dags/knix.yaml` runs the shared steps once, then install and build
  once per machine, in the order `dags/direct.yaml` uses.

`notes/steps.md` holds the rules the new steps follow. The knix DAG
passing, and leaving no `.qemu` file or state directory behind, is the
check.
