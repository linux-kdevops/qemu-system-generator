# qemu-system-generator

A systemd generator for `.qemu` files: one file per virtual machine, in
systemd unit-file syntax, whose `[QEMU]` section is QEMU's own command
line passed through line by line. The generator runs at boot and on
`systemctl daemon-reload`, writes `qemu-system@<name>.service` and the
`virtiofsd@<name>-<tag>` socket and service of every directory the
machine shares, then exits. Nothing stays between systemd and QEMU.

Status: research draft. The format is documented in
[man/qemu-systemd.unit.5.scd](man/qemu-systemd.unit.5.scd) and the
program in [man/qemu-system-generator.8.scd](man/qemu-system-generator.8.scd);
`docs/` holds the walk-throughs and builds them, with the man pages, into
a Sphinx site for Read the Docs.
Nothing is released yet.

**License**: copyleft-next-0.3.1

## A file

```ini
[QEMU]
machine=q35
accel=kvm
cpu=host
m=2048
smp=2
drive=file=fedora.raw,if=virtio,format=raw
nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
device=vhost-vsock-pci-non-transitional,guest-cid=42
nographic=

[virtiofsd]
shared-dir=%h
tag=home
sandbox=namespace
uid-map=:0:%U:1:
gid-map=:0:%G:1:

[Install]
WantedBy=machines.target
```

Each `[QEMU]` key is a QEMU option without its dash and the value is
the option's argument, verbatim: `man qemu-system` is the reference.
Each `[virtiofsd]` section is one virtiofsd with its options spelled the
same way: `virtiofsd --help` is the reference. `[Unit]`, `[Service]` and
`[Install]` go to systemd untouched. The generator adds the lifecycle
only: directories, device access, the QMP socket it powers the guest
down through, the machined registration, and the virtiofs wiring.

Saved as `~/.config/qemu/systemd/fedora.qemu`:

```shell
systemctl --user daemon-reload
systemctl --user start qemu-system@fedora
machinectl --user list
ssh machine/fedora
systemctl --user stop qemu-system@fedora
```

## Quick start

```shell
cargo build --release
sudo install --mode=0755 target/release/qemu-system-generator /usr/libexec/
sudo ln --symbolic /usr/libexec/qemu-system-generator /usr/lib/systemd/system-generators/
sudo ln --symbolic /usr/libexec/qemu-system-generator /usr/lib/systemd/user-generators/
sudo mkdir --parents /usr/share/qemu/systemd
sudo install --mode=0644 data/qmp-powerdown /usr/share/qemu/systemd/
```

Without root, a user runs it through two user units instead; see
`qemu-system-generator(8)`. [docs/bootstrap.rst](docs/bootstrap.rst) walks
through booting a NixOS guest that way, and
[docs/bootstrap-knix.rst](docs/bootstrap-knix.rst) moves it onto a kernel
and a QEMU of your own.

The generated units call `socat`, `varlinkctl` and `ssh` by name; the
build environment can pin them, and the QEMU and virtiofsd binaries, to
absolute paths (see `build.rs`).

Check a file before deploying it:

```shell
QEMU_SYSTEMD_UNIT_DIRS=examples ./target/release/qemu-system-generator --user --dryrun
```

With Nix:

```shell
nix build
nix run . -- --user --dryrun
nix develop
```

The flake exports a package, an overlay and a NixOS module
(`virtualisation.qemu-system-generator.enable = true`, with `units` for
files under `/etc/qemu/systemd/`).

## Layout

```
src/unitfile.rs          unit-file syntax: sections as ordered lines, quoting
src/search.rs            search path, discovery, drop-ins
src/plan.rs              what a backend decodes a file into, launcher-neutral
src/backend/qemu.rs      [VM], [QEMU], [virtiofsd]
src/launcher/systemd.rs  a Plan into .service/.socket files and symlinks
src/main.rs              generator entry point and command line
data/qmp-powerdown       the QMP commands ExecStop= sends
data/qemu-system-generator.*  user units that run it without root
man/                     man pages, scdoc
docs/                    walk-throughs and the Sphinx site
examples/                one file per feature; the conformance inputs
tests/                   expected generator output, verify script
nix/                     package and NixOS module
```

The suffix selects the backend and a section named after the backend's
binary decodes to its command line; `[QEMU]` is the first backend, the
systemd launcher the first launcher. Another backend adds its section,
suffix and generator binary; another launcher turns the same plan into
its own files.

## Checks

```shell
cargo test
cargo clippy --all-targets
tests/verify
nix flake check
```

`tests/verify` generates the examples and loads them with
`systemd-analyze verify` in both scopes. `tests/update-expected` rewrites
the expected output after a deliberate format change.

`nix flake check` also builds the documentation as Read the Docs does.
To preview it while editing:

```shell
nix develop --command sphinx-autobuild docs docs/_build/html
```
