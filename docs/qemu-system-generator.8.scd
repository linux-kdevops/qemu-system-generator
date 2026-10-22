qemu-system-generator(8)

# NAME

qemu-system-generator - systemd generator for .qemu files

# SYNOPSIS

*qemu-system-generator* [*--user*|*--system*] [*--dryrun*] \
	[_normal-dir_ [_early-dir_ _late-dir_]]

# DESCRIPTION

*qemu-system-generator* is a systemd generator (*systemd.generator*(7)).
The service manager runs it at boot and on every *systemctl
daemon-reload*; it reads every .qemu file on the search path described in
*qemu-systemd.unit*(5) and writes, into _normal-dir_, a service unit per
file together with the socket and service units of each virtiofsd the
machine uses, and the symlinks that *[Install]* asks for. It exits
afterwards; nothing of it stays between systemd and QEMU.

It is installed under _/usr/lib/systemd/system-generators/_ and
_/usr/lib/systemd/user-generators/_ as symlinks to one binary. The scope
is taken from *$SYSTEMD_SCOPE*, which the service manager sets, or from
*--user* and *--system*; without either, the effective user id decides.

With *--dryrun* nothing is written: every generated file is printed after
a *---path---* line, and every symlink as *---path -> target---*. Together
with *$QEMU_SYSTEMD_UNIT_DIRS* this checks a file before deploying it:

```
QEMU_SYSTEMD_UNIT_DIRS=. qemu-system-generator --user --dryrun
```

# OPTIONS

*--user*, *--system*
	The scope to generate for, overriding *$SYSTEMD_SCOPE*.

*--dryrun*
	Print the generated files instead of writing them.

*--version*
	Print the version.

*--help*
	Print usage.

# ENVIRONMENT

*$SYSTEMD_SCOPE*
	Set by the service manager to _system_ or _user_.

*$QEMU_SYSTEMD_UNIT_DIRS*
	Colon-separated directories that replace the search path.

# FILES

_/etc/qemu/systemd/_, _/run/qemu/systemd/_, _/usr/share/qemu/systemd/_
	Search path of the system manager.

_~/.config/qemu/systemd/_
	A user's own files; see *qemu-systemd.unit*(5) for the full list.

_/usr/share/qemu/systemd/qmp-powerdown_
	The QMP commands the generated *ExecStop=* writes to the machine's
	control socket.

_/run/systemd/generator/_, _$XDG_RUNTIME_DIR/systemd/generator/_
	Where the generated units land.

# EXIT STATUS

0 when every file was generated, 1 when any file failed (the others are
still written), 2 on a usage error. A failing file is reported on standard
error, which the service manager forwards to the journal.

# SEE ALSO

*qemu-systemd.unit*(5), *systemd.generator*(7), *systemctl*(1)
