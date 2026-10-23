qemu-systemd.unit(5)

# NAME

qemu-systemd.unit - systemd units from .qemu files

# SYNOPSIS

_name_.qemu

Search path of the system manager, highest precedence first:

- /run/qemu/systemd/
- /etc/qemu/systemd/
- /usr/share/qemu/systemd/

Search path of a user manager:

- $XDG_RUNTIME_DIR/qemu/systemd/
- $XDG_CONFIG_HOME/qemu/systemd/, by default ~/.config/qemu/systemd/
- /etc/qemu/systemd/users/$UID/
- /etc/qemu/systemd/users/
- /usr/share/qemu/systemd/users/$UID/
- /usr/share/qemu/systemd/users/

# DESCRIPTION

A .qemu file declares one virtual machine in systemd unit-file syntax.
*qemu-system-generator*(8), a systemd generator, reads the files on the
search path at boot and on every *systemctl daemon-reload* and writes an
ordinary service unit for each, plus a socket and a service for every
virtiofsd the machine uses. The generated units are started, stopped and
inspected with *systemctl*(1) like any other, and the machine is registered
with *systemd-machined.service*(8) so that *machinectl*(1) lists it.

The file passes QEMU's command line through. The section named after the
backend, *[QEMU]*, holds one QEMU option per line, spelled as *qemu-system*
spells it; the generator never translates, validates or reorders those
lines. What it adds is the lifecycle: directories, device access, the
control socket it stops the machine through, the registration, and the
daemons a virtiofs share needs. Everything else about systemd units,
dependencies, resource control and sandboxing, is written in the usual
*[Unit]*, *[Service]* and *[Install]* sections and handed to systemd
unchanged.

The file name is the machine name: _nixos.qemu_ becomes
_qemu-system@nixos.service_, and the machine is registered as _nixos_. A
file in a higher-precedence directory hides a file of the same name
further down. The name is made of letters, digits, *:*, *\_*, *.* and
*-*; machined additionally wants a valid host name, so a name with *:* or
*\_* runs but is not registered.

# FILE FORMAT

Lines follow *systemd.syntax*(7): _Key=Value_ with whitespace around the
equals sign dropped, lines starting with *#* or *;* are comments, a
trailing backslash continues the line on the next one (comment lines inside
a continuation are skipped). Sections keep their lines in file order, and a
repeated key repeats the line; what an empty value means is said for each
section below. Unlike a systemd unit, an empty value never resets a list.

Every section except *[virtiofsd]* may appear more than once; the
occurrences are merged in file order. Each *[virtiofsd]* section is one
daemon.

Two systemd rules reach into values that end up in a command line
(*systemd.service*(5), "Command lines"): *%* starts a specifier, so a
literal percent sign is written *%%*, and *$NAME* standing alone is an
environment variable, so a literal dollar sign is written *$$*. The
generator quotes every argument as *systemd.syntax*(7) "Quoting" reads it
back, so spaces, quotes and backslashes in a value need no escaping in the
file. Specifiers are useful on purpose: *%i* is the machine name, *%t*
the runtime directory root, *%S* the state directory root, *%h* the home
directory, *%U* and *%G* the user and group id. See *systemd.unit*(5),
"Specifiers".

## Drop-ins

A directory _name.qemu.d/_ next to the file holds _\*.conf_ files that are
appended to it in file-name order. The type directory _qemu.d/_ applies to
every file; for a name with dashes, _foo-.qemu.d/_ and _foo-bar-.qemu.d/_
apply to _foo-bar-baz.qemu_, which is how a family of machines shares its
lines. A file name further down that hierarchy replaces the same name
further up. All directories on the search path are consulted. A file or
directory whose name starts with a dot is skipped, as systemd skips it.

# SECTIONS

## The [VM] section

Values that are not options of a backend binary.

*QemuBinary=*
	The QEMU binary, the first word of *ExecStart=*: an absolute path,
	which may begin with a specifier such as *%h*, or a file name that
	systemd looks up in its search path (*systemd.service*(5), "Command
	lines"). Default: _qemu-system-_ for the host
	architecture in the directory compiled into the generator.

*VirtiofsdBinary=*
	The virtiofsd binary used by every *[virtiofsd]* section, read as
	*QemuBinary=* is. Default: compiled into the generator.

*SshPrivateKey=*
	Path of a private key with which root logs in to the guest. When set,
	stopping the service first runs *systemctl poweroff* in the guest over
	SSH through *root@vsock/*_cid_, which *systemd-ssh-proxy*(1) routes,
	and only then presses the QMP power button. Needs the vsock device
	described under *[QEMU]* below.

## The [QEMU] section

Each line is one QEMU option. The key is the option name without its
leading dash, the value is the option's one argument, verbatim. An empty
value is a bare flag. A repeated key repeats the option. The command line
is the file order.

```
machine=q35          -machine q35
m=2048               -m 2048
device=nvme,serial=a -device nvme,serial=a
nographic=           -nographic
S=                   -S
```

The generator holds no list of QEMU options. A misspelt key reaches QEMU
and fails there; *qemu-system*(1) and *qemu-system-\* -help* are the
reference, and *systemd-analyze verify* cannot catch it. The following
lines are read, but still emitted exactly as written:

*machine=*
	A value containing _microvm_ switches the generated vhost-user-fs
	device to its virtio-mmio name. A value containing _memory-backend=_
	tells the generator that the file supplies its own shared memory
	backend for virtiofs.

*m=*
	The guest memory, read twice. A *[virtiofsd]* section needs a shared
	memory backend of the same size: the first field, or the _size=_
	field, is copied, and a bare number, which *-m* reads as mebibytes,
	is given an *M*. A VFIO device needs that memory locked:
	*LimitMEMLOCK=* is set to it plus 256 MiB, read as an integer with an
	optional B, K, M, G, T, P or E suffix.

*uuid=*
	Sent as the _id_ of the machined registration, so that
	*machinectl status* shows the UUID the guest sees.

*device=vhost-vsock...,guest-cid=N*
	The CID, a decimal number, is sent as _vSockCid_ of the registration;
	afterwards *ssh machine/*_name_ reaches the guest through
	*systemd-ssh-proxy*(1). *SshPrivateKey=* uses the same device.

*device=vfio-...*
	Grants the service the VFIO device nodes and raises *LimitMEMLOCK=*,
	since assigned devices pin guest memory. Binding the device to the
	vfio-pci driver is host configuration outside the file.

The generator appends its own lines after the file's: the virtiofs wiring
described below and *-qmp* on the control socket. The ids _mem_ and
_virtiofs\__tag_ are its; a file that defines them collides in QEMU. A
second *-qmp* from the file is fine, QEMU serves several.

A 9p share is two ordinary lines and needs no daemon:

```
fsdev=local,id=fsdev_share,path=/srv/share,security_model=none,multidevs=remap
device=virtio-9p-pci-non-transitional,fsdev=fsdev_share,mount_tag=share
```

## The [virtiofsd] section

One section per virtiofs share. Each key is a virtiofsd long option
without its leading dashes, the value its argument, an empty value a bare
flag, in file order; *virtiofsd --help* is the reference.

*tag=*
	Required. Names the daemon's units, _virtiofsd@name-tag.socket_ and
	_.service_, and the mount tag the guest sees. Letters, digits, dot,
	underscore and dash. Since both names may hold a dash, _a-b.qemu_ with
	tag _c_ and _a.qemu_ with tag _b-c_ name the same units; the file
	later in name order is reported and not generated. The line is passed to virtiofsd like any other;
	its *--tag* advertises the same tag through the vhost-user CONFIG
	feature, which QEMU 8.1 and later ignore and QEMU 7.1 to 8.0 cannot
	handle.

*shared-dir=*
	Read for *RequiresMountsFor=* on the daemon's unit when it is an
	absolute path without specifiers; otherwise passed on like any key.

The generator runs the daemon socket-activated, appending *--fd=3* to its
command line, so *socket-path=* and *socket=* must not be set. On the
QEMU side it adds a *-chardev socket* and a *-device vhost-user-fs-pci*
(or *-device vhost-user-fs-device* on microvm) per section and, once, an
*-object memory-backend-memfd* with *-machine memory-backend=mem* unless
the file names its own memory backend. The unprivileged sandbox that
virtiofsd needs is written in the file, as the examples do:

```
[virtiofsd]
shared-dir=%h
tag=home
sandbox=namespace
uid-map=:0:%U:1:
gid-map=:0:%G:1:
xattr=
no-announce-submounts=
```

## [Unit], [Service] and [Install]

Copied into the generated service after the generator's own lines, so a
scalar setting in *[Service]* such as *TimeoutStopSec=* or *KillMode=*
overrides the generated one, and an empty *ExecStop=* clears the
generated stop commands before new ones. *Description=* in *[Unit]*
replaces the default. *[Install]* is honoured the way *systemctl enable*
would: *WantedBy=*, *RequiredBy=*, *UpheldBy=* and *Alias=* become
symlinks in the generator's output directory; a generated unit cannot be
enabled afterwards. Each value must be a unit name; specifiers are not
expanded there. Since the service is an instance, systemd accepts only an
*Alias=* that is a service template, which gets the machine name as its
instance, or that instance spelled out: _nixos.qemu_ with
*Alias=vm@.service* is also _vm@nixos.service_. A value the generator
cannot link is reported and skipped. The user manager has
_machines.target_ as the system manager does.

# GENERATED UNITS

For _nixos.qemu_ with one *[virtiofsd]* section tagged _store_, the
generator writes _qemu-system@nixos.service_,
_virtiofsd@nixos-store.socket_ and _virtiofsd@nixos-store.service_ into
its normal-priority output directory, _/run/systemd/generator/_ or
_$XDG_RUNTIME_DIR/systemd/generator/_ (*systemd.generator*(7)), where
*systemctl cat* shows them.

The service carries *SourcePath=* back to the file, is part of and
ordered before _machines.target_, after _network.target_ under the system
manager, *Requires=* and *After=* each daemon's socket and *Requires=*
its service, and then:

```
[Service]
Type=simple
SyslogIdentifier=qemu-system@%i
RuntimeDirectory=qemu-system/%i
StateDirectory=qemu-system/%i
WorkingDirectory=%S/qemu-system/%i
ExecStart=<QemuBinary> <[QEMU] lines> <virtiofs lines> \\
    -qmp unix:%t/qemu-system/%i/qmp.sock,server=on,wait=off
ExecStartPost=-varlinkctl call %t/systemd/machine/io.systemd.Machine \\
    io.systemd.Machine.Register "{...}"
ExecStop=-socat OPEN:/usr/share/qemu/systemd/qmp-powerdown!!OPEN:/dev/null,wronly \\
    UNIX-CONNECT:%t/qemu-system/%i/qmp.sock
KillSignal=SIGCONT
KillMode=mixed
TimeoutStopSec=2min
Slice=machine.slice
DevicePolicy=closed
DeviceAllow=/dev/kvm rw
DeviceAllow=/dev/vhost-vsock rw
DeviceAllow=/dev/vhost-net rw
DeviceAllow=/dev/net/tun rwm
DeviceAllow=char-pts rw
```

Relative paths in the file, such as a disk image, resolve in the state
directory: _/var/lib/qemu-system/nixos/_ under the system manager,
_~/.local/state/qemu-system/nixos/_ under a user manager. The runtime
directory holds the QMP socket and anything the file puts under
_%t/qemu-system/%i/_.

Stopping the service sends _system_powerdown_ over QMP, the ACPI power
button, by writing the two commands in _qmp-powerdown_ to the socket.
QEMU exits on *SIGTERM* without powering the guest down, so the kill
signal is *SIGCONT*, which it ignores, and systemd waits for the process
to exit on its own; a guest that does not act within *TimeoutStopSec=* is
killed. With *SshPrivateKey=* a guest-side *systemctl poweroff* over SSH
runs first and the QMP command is the fallback.

A VFIO device adds *DeviceAllow=/dev/vfio/vfio rw*, *DeviceAllow=char-vfio
rw*, which covers every IOMMU group node, and *LimitMEMLOCK=*.

The registration names the machine after the file, with class _vm_ and
service _qemu-system_. It is best effort: without
*systemd-machined.service*(8) the machine still runs.

The daemon's socket listens on _%t/virtiofsd/name-tag/virtiofsd.sock_ and
its service stops when nothing needs it (*StopWhenUnneeded=*). It is
ordered *Before=* the machine so that it outlives the guest's unmount
during shutdown.

# EXAMPLES

## A cloud image with a forwarded SSH port

```
[Unit]
Description=Fedora cloud image

[QEMU]
machine=q35
accel=kvm
cpu=host
m=2048
smp=2
drive=file=fedora.raw,if=virtio,format=raw
drive=file=seed.iso,if=virtio,format=raw,readonly=on
nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
device=virtio-rng-pci-non-transitional
device=vhost-vsock-pci-non-transitional,guest-cid=42
nographic=
serial=mon:stdio

[Install]
WantedBy=machines.target
```

## A family of machines

_fstests-.qemu.d/10-machine.conf_ holds the lines every _fstests-\*.qemu_
shares; each file holds what differs:

```
[Unit]
Description=fstests on xfs

[QEMU]
append=console=ttyS0 root=/dev/vda rw fstests=xfs

[Install]
WantedBy=machines.target
```

More examples ship in _/usr/share/doc/qemu-system-generator/examples/_.

# ENVIRONMENT

*$QEMU_SYSTEMD_UNIT_DIRS*
	A colon-separated list of directories that replaces the search path,
	for testing with *qemu-system-generator --dryrun*.

# SEE ALSO

*qemu-system-generator*(8), *qemu-system*(1), *systemd.unit*(5),
*systemd.service*(5), *systemd.exec*(5), *systemd.resource-control*(5),
*systemd.kill*(5), *systemd.generator*(7), *systemd.syntax*(7),
*machinectl*(1), *systemd-ssh-proxy*(1)
