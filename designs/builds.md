<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Builds

## Where a build happens

The knix guide and its steps build the kernel and QEMU with `make` in a
clone of the release, inside knix's `build-kernel` and `build-qemu`
shells, and then add the install to the Nix store. Neither builds as a
derivation (nixpkgs' `linuxManualConfig`, knix's `lib.buildKernel` and
`lib.buildQemu`): a derivation builds everything again after any change,
without ccache, and its log is Nix's rather than the commands the guide
teaches.

QEMU v11.1.3 `x86_64-softmmu` and Linux v7.3 `defconfig` with GCC, timed
back to back on one host with 8 cores and 16 threads. The derivations
took the tree without `.git` and `build/`. The `make` rows start from an
empty ccache and leave out the install:

| Build             | Cold       | One-line change | Unchanged |
| ----------------- | ---------- | --------------- | --------- |
| QEMU, `make`      | 6 min 11 s | 8 s             | 0.5 s     |
| QEMU, derivation  | 3 min 45 s | full build      | 2.6 s     |
| Linux, `make`     | 5 min 19 s | 22 s            | 8.7 s     |
| Linux, derivation | 4 min 16 s | full build      | 13.4 s    |

With nothing changed, the Linux derivation still reads the 1.6 GB tree
to hash it. A derivation gives the same path on every host, which a
binary cache could serve: measure again when CI fetches builds instead
of making them.

## How a build enters the store

An install goes in the way its files need, and `nix build PATH
--out-link` links the path as a garbage-collector root.

- The kernel loads nothing from the store, so `make dir-pkg` installs it
  and its modules and `nix store add` adds them: the path follows from
  the files alone. Of 181 kernels from v7.1 to v7.4-rc1, only
  `CONFIG_RUST` kernels name a store path, Rust's library sources in the
  panic messages of `core`, and the kernel never reads it.
- QEMU's binaries load from the store through their interpreter and
  `RUNPATH`, which `nix store add` does not record: a collection could
  delete those paths, and `nix copy` would leave them behind. Its
  install is the output of a derivation that copies it, with the
  `build-qemu` shell's `inputDerivation` as an input
  (`steps/knix/files/store.nix`), so Nix records the store paths the
  copy names. It keeps a second copy of the install until a collection,
  gives the same files a new path whenever the shell changes, and drops
  a store path the install names outside the shell's closure. Without
  `__structuredAttrs`, `inputDerivation` leaves out a shell's `env`
  (nixpkgs' `pkgs/stdenv/generic/make-derivation.nix`).

`nix store add-path` is deprecated since Nix 2.19. Neither it nor
`nix store add` records a reference. A content-addressed derivation would
give a path from the files that records its references, but it is an
experimental feature of Nix.
