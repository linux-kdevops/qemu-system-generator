<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Build the guide's kernel as a Nix derivation

Status: research, 2026-10-28.

The knix guide builds the kernel with `make` in the reader's tree and
boots knix's `imageless` closure, which has no kernel of its own and
takes `/lib/modules` over virtiofs. Its "Why knix" section says nixpkgs
"can build a kernel from your tree, but as a Nix derivation, which
rebuilds from scratch after every change". That sentence dates from the
guide's first version (441c7be) and was never measured. The steps
already take the other road: ADR 0002 builds the same configuration
through knix's `lib.buildKernel`, which hands the `.config` to nixpkgs'
`linuxManualConfig`; v7.3 built in 3 min 54 s from scratch, and an
unchanged input returned the same store path at once.

## The two roads

`make` in the tree, the guide's road:

- Incremental: a one-line change rebuilds a few objects, and the tree
  keeps `vmlinux`, `compile_commands.json` and the objects that `gdb`,
  `clangd`, `make M=` and `git bisect` work with.
- Nix never sees the result. The machine points at a path in the tree,
  so nothing is cached, shared with another host or kept by a GC root.
- The closure must not carry a kernel, which is what knix's `imageless`
  module provides: an initrd with no modules, boot-critical drivers built
  in, and `/lib/modules` mounted from the host.

A derivation, nixpkgs' road (`linuxManualConfig` or
`linuxPackages_custom`, `doc/packages/linux.section.md`):

- The kernel is a store path keyed by its inputs: built once, reused,
  copied between hosts with `nix copy`, kept by a GC root.
- It can be the closure's own kernel (`boot.kernelPackages`), so a plain
  NixOS closure boots it with matching modules in the initrd and in
  `/lib/modules`, with no `modules` share and no knix module.
- Every change builds from scratch in the sandbox, and `src` is the
  whole tree copied into the store, so a dirty build directory has to be
  filtered out. With the kernel in the closure, a kernel change also
  rebuilds the initrd and the system.
- nixpkgs itself does not iterate this way for modules: its manual's
  example of the edit-compile-run loop enters the kernel's build
  environment with `nix-shell` and runs `make M=...` ("Developing kernel
  modules", `doc/packages/linux.section.md`,
  <https://nixos.org/manual/nixpkgs/stable/#sec-linux-kernel-developing-modules>).

So the derivation is the Nix-friendly road, reproducible and shareable,
and `make` is the kernel developer's road, incremental and in the tree.
The open question is how far apart they are in practice.

## The experiment

On v7.3 with the guide's `defconfig kvm_guest.config` configuration,
after a one-line change to one file:

1. `make O=build` in the tree: wall time.
2. `linuxManualConfig` with `src` the same tree, filtered: wall time,
   and the time to copy the tree into the store.
3. A plain NixOS closure with that kernel as `boot.kernelPackages`: the
   time to rebuild the closure after the kernel changes, and whether it
   boots under the generator with only the store share.
4. Whether ccache, through `extra-sandbox-paths`, narrows the gap in 2.

The result decides whether the guide adds a derivation section next to
the `make` one, whether "Why knix" keeps its sentence as written, and
whether a reader who does not change the kernel between boots is better
served by the derivation and a knix-free closure.
