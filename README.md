# qemu-system-generator

A [systemd] generator for `.qemu` files: one file per virtual machine,
in systemd [unit-file syntax], whose `[QEMU]` section is [QEMU]'s own
command line passed through line by line. The generator runs at boot and
on [`systemctl daemon-reload`][systemctl], writes
`qemu-system@<name>.service` and the `virtiofsd@<name>-<tag>` socket and
service of every directory the machine shares, then exits. Nothing stays
between systemd and QEMU.

It is modelled on [Podman]'s [Quadlet]: the search path, drop-ins,
`[Install]` and `--dryrun` work as in [`podman-systemd.unit(5)`][quadlet-man],
while `[QEMU]` keeps QEMU's own option names.

Status: research draft. The format is documented in
[man/qemu-systemd.unit.5.scd](man/qemu-systemd.unit.5.scd) and the
program in [man/qemu-system-generator.8.scd](man/qemu-system-generator.8.scd);
`docs/` holds the walk-throughs and builds them, with the man pages, into
a [Sphinx] site for [Read the Docs].
Nothing is released yet.

**License**: copyleft-next-0.3.1

## A file

```ini
[QEMU]
machine=q35
nodefaults=
accel=kvm
cpu=host
m=2048
smp=2
drive=file=fedora.raw,if=virtio,format=raw
netdev=user,id=net0,hostfwd=tcp:127.0.0.1:10022-:22
device=virtio-net-pci-non-transitional,netdev=net0,romfile=
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

Each `[QEMU]` key is a QEMU option without its dash and the value is the
option's argument, verbatim: [`man qemu-system`][QEMU] is the reference.
Each `[virtiofsd]` section is one [virtiofsd] with its options spelled
the same way: `virtiofsd --help` is the reference. `[Unit]`, `[Service]`
and `[Install]` go to systemd untouched. The generator adds the
lifecycle only: directories, device access, the [QMP] socket it powers
the guest down through, the [machined] registration, and the [virtiofs]
wiring.

Saved as `~/.config/qemu/systemd/fedora.qemu`:

```shell
systemctl --user daemon-reload
systemctl --user start qemu-system@fedora
machinectl --user list
ssh machine/fedora
systemctl --user stop qemu-system@fedora
```

## Getting started

[docs/getting-started/](docs/getting-started/) has the host requirements
(systemd and Nix), the three ways to install (for one user without root,
for the whole host, or through the flake's NixOS module) and a quick start
that boots a disk image you already have. The guides in
[docs/bootstrap/](docs/bootstrap/) then boot a guest from a [NixOS] disk
image, a [UKI], a kernel and an initrd, a kernel and a QEMU of your own,
or a distribution's cloud image.

Check a file before deploying it:

```shell
QEMU_SYSTEM_GENERATOR_UNIT_DIRS=examples \
    ./target/release/qemu-system-generator --user --dryrun
```

With [Nix]:

```shell
nix build
nix run . -- --user --dryrun
nix develop
```

## Layout

```text
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
nix/                     package, NixOS module and Dagu
steps/                   the commands the guides of docs/bootstrap/ teach
dags/, config.yaml       the Dagu DAGs that wire the steps, and their config
notes/                   rules for contributors, unpublished
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
[`systemd-analyze verify`][systemd-analyze] in both scopes.
`tests/update-expected` rewrites the expected output after a deliberate
format change.

`steps/` holds the commands that do what each guide of `docs/bootstrap/`
teaches, and `dags/` wires them for [Dagu], one DAG per guide;
`.github/workflows/bootstrap.yaml` runs every DAG. A run writes where the
guides write, so it replaces `~/bootstrap/<guide>` and the machines'
`.qemu` files, and adds the generator to your Nix profile:

```shell
nix develop --ignore-env \
    --keep-env-var HOME --keep-env-var XDG_RUNTIME_DIR \
    --command dagu --config config.yaml start disk
```

`notes/steps.md` has the rules the steps follow.

`nix flake check` also builds the documentation as Read the Docs does
and holds the Markdown and reStructuredText prose to 80 columns.
To render it, or to preview it while editing:

```shell
nix run .#docs           # render to docs/_build/html
nix run .#serve -- 8001  # live-render on 127.0.0.1:8001, rebuilding on save
```

On a remote host, forward the port first with
`ssh -L 8001:127.0.0.1:8001 <host>`.

[systemd]: https://systemd.io
[unit-file syntax]: https://www.freedesktop.org/software/systemd/man/latest/systemd.syntax.html
[QEMU]: https://www.qemu.org/docs/master/system/invocation.html
[systemctl]: https://www.freedesktop.org/software/systemd/man/latest/systemctl.html
[Podman]: https://podman.io
[Quadlet]: https://github.com/podman-container-tools/podman/tree/main/pkg/systemd/quadlet
[quadlet-man]: https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html
[Dagu]: https://github.com/dagucloud/dagu
[Sphinx]: https://www.sphinx-doc.org
[Read the Docs]: https://about.readthedocs.com
[virtiofsd]: https://gitlab.com/virtio-fs/virtiofsd
[QMP]: https://www.qemu.org/docs/master/interop/qmp-spec.html
[machined]: https://www.freedesktop.org/software/systemd/man/latest/systemd-machined.service.html
[virtiofs]: https://virtio-fs.gitlab.io
[NixOS]: https://nixos.org/manual/nixos/stable/
[UKI]: https://uapi-group.org/specifications/specs/unified_kernel_image/
[Nix]: https://nix.dev/manual/nix/latest/
[systemd-analyze]: https://www.freedesktop.org/software/systemd/man/latest/systemd-analyze.html
