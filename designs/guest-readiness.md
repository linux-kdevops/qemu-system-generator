<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->

# Guest readiness: QEMU should relay the guest's READY=1

`qemu-system@.service` is `Type=simple`: QEMU master (bddb785e79) calls
no `sd_notify()`, so `systemctl start` returns once QEMU has forked, long
before the guest can be used. `steps/machine/wait` finds out by asking the
guest over SSH whether its systemd has finished starting, and the DAG
retries it, because OpenSSH ignores `ConnectionAttempts` behind a
`ProxyCommand` (`sshconnect.c:579`).

## What the guest offers

systemd 253 gave the guest a way to say it is up. PID 1 reads the
`vmm.notify_socket` credential and sends `sd_notify()` messages to that
address; `READY=1` goes out once the job queue is empty
(`manager_send_ready_on_idle()` in `src/core/manager.c`), the moment
`is-system-running --wait` would return. One restart of the knix guest
sent 35 messages, each over its own connection and none ending in a
newline: the previous boot's shutdown (`X_SYSTEMD_UNIT_INACTIVE=` per
target, `EXIT_STATUS=0`), then `X_SYSTEMD_HOSTNAME=`,
`X_SYSTEMD_MACHINE_ID=`, `X_SYSTEMD_UNIT_ACTIVE=` per target, and last
`READY=1` with `STATUS=Ready.`.

## The option tried: the step listens

The machine's file passes the credential and the step listens. For a
firmware boot the file has an `smbios=` line under `[QEMU]`:

    smbios=type=11,value=io.systemd.credential:vmm.notify_socket=vsock-stream:2:PORT

For a direct kernel boot, `systemd.set_credential=vmm.notify_socket:`
followed by the same address goes on `append=`; it needs neither SMBIOS
firmware nor `dmi-sysfs` in the guest. The address must be
`vsock-stream`: a bare `vsock:` tries `SOCK_DGRAM` first, which
vhost-vsock lacks. The port must be above 1023, since binding a lower one
needs `CAP_NET_BIND_SERVICE` (`net/vmw_vsock/af_vsock.c`).

The step takes the port from the unit's `ExecStart=`, opens
`socat VSOCK-LISTEN:PORT,fork` before restarting the machine, and returns
on a line that is exactly `READY=1`. Each connection forks a
`grep --line-regexp READY=1`, which writes the whole line at once, so two
messages cannot run together in the pipe the children share. The knix
guest reported ready 1 s after the restart.

It costs every machine a step starts the credential line, and a guest
whose systemd predates 253 fails the step when the timeout runs out. The
guest is not authenticated: any local guest that knows the port can send
`READY=1`, and the worst it does is end the wait early. Asking over SSH
works for any guest with sshd, so the step keeps it until QEMU relays.

## How systemd does it

systemd's own managers relay. systemd-vmspawn binds an `AF_VSOCK`
listener, hands the guest `vsock-stream:2:PORT` (`add_vsock_credential()`
in `src/vmspawn/vmspawn.c`), and with `--notify-ready=yes`, the default,
sends its own `READY=1` to its service manager only after the guest's.
It forwards `READY`, `STATUS` and `EXIT_STATUS` and drops the rest, so a
guest cannot set `MAINPID=` or `STOPPING=1` on a host unit.
systemd-nspawn does the same for a container (`src/nspawn/nspawn.c`). The
unit's main process is the machine manager, and `systemctl start` means
the guest has booted. QEMU is that process here, and it does not relay.

## Rejected options

Making `qemu-system@` itself `Type=notify` with stock tools cannot work.
Only a process in the unit's cgroup may send its `READY=1`
(`NotifyAccess=all`), systemd kills what `ExecStartPre=` leaves running
before the next command (`systemd.service(5)`), and on a `Type=notify`
unit `ExecStartPost=` runs only once the unit is ready. What is left is a
wrapper as `ExecStart=` or the generator running beside QEMU, and the
principles allow neither.

A generated `qemu-system-ready@NAME.socket` on `ListenStream=vsock::PORT`,
ordered before the machine, activating a `Type=notify` companion whose
`socat ACCEPT-FD:3,fork` handler runs `systemd-notify --ready` on a
`READY=1` line, gives `systemctl start` of the companion the meaning the
step has. The socket lets the listener exist before QEMU while readiness
arrives after it; a single unit ordered before the machine would
deadlock, and one ordered after would race the guest. It costs a shell
string in a generated unit, `sh`, `grep` and `systemd-notify` at run
time, and a third unit prefix, and the readiness still sits on a sibling
rather than on the machine. Whether socat's `ACCEPT-FD` takes a vsock
socket from systemd is unverified.

A socket with `Accept=yes`, as `systemd-ssh-generator` writes for
`vsock::22`, starts one service per connection, and none of them can mark
another unit ready. `systemd-notify --fork` is the receiving side of the
protocol, but it listens on an autobound `AF_UNIX` socket
(`src/shared/notify-recv.c`), which a guest cannot reach.
systemd-vmspawn as a second launcher would relay correctly, but it builds
the QEMU command line itself, and `[QEMU]` lines are passed verbatim.
Waiting for `X_SYSTEMD_UNIT_ACTIVE=ssh-access.target` would be earlier
than `READY=1` for a step that only needs SSH, but the target comes from
`systemd-ssh-generator`, and the knix guest sent no such message.

## What would make it proper

QEMU should relay, as vmspawn does. That takes two QEMU changes and one
here.

Michael Tokarev's "[PATCH] implement systemd startup notification for
qemu-system" (`20251217131909.3713900-1-mjt@tls.msk.ru`) is the first:
QEMU sends `READY=1` to an `AF_UNIX` `$NOTIFY_SOCKET` from
`os_setup_post()`, the last call of `qemu_init()`. With it, `Type=notify`
means QEMU is initialised, its devices realised and its QMP socket
listening, which already fixes `ExecStartPost=` racing QEMU and lets a
bad `[QEMU]` line fail `systemctl start`. It says nothing about the
guest.

The second change holds that `READY=1` back until the guest's: QEMU binds
an `AF_VSOCK` listener (it has the code, `vsock_listen_saddr()` in
`util/qemu-sockets.c`), waits for the guest's `READY=1`, and forwards
only `READY`, `STATUS` and `EXIT_STATUS`, with an option to keep the
first patch's behaviour for guests without systemd 253.

The generator then emits `Type=notify` and adds the credential itself, as
it already adds the virtiofs devices and the CID registration: SMBIOS for
a firmware boot, and `-fw_cfg
name=opt/io.systemd.credentials/vmm.notify_socket,string=...` for a
direct kernel boot (`src/core/import-creds.c`), which leaves `append=`
alone but needs `CONFIG_FW_CFG_SYSFS` in the guest. It needs a QEMU floor
or an opt-in from the file, because an unpatched QEMU never sends
`READY=1` and its start would time out. `steps/machine/wait` then goes,
and the DAGs' start becomes `systemctl --user restart`.

A stock relay in systemd, a listener on vsock that turns the guest's
`READY=1` into its own unit's, would make the socket option free of the
shell string. It is worth less than the QEMU change: vmspawn already
shows where systemd puts the relay.
