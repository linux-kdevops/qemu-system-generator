<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->
# Bootstrap: a NixOS guest from a .qemu file

This walk-through boots a [NixOS] guest under your user's [systemd]
manager, from one `.qemu` file that grows by one feature per step. The
guest's kernel, initrd and system come from [nixpkgs], and its root
file system is a tmpfs: `/nix/store` is the host's own, shared over
[virtiofs]. There is no disk image, and no step needs root.

The file format is specified in
[`qemu-systemd.unit(5)`](qemu-systemd.unit.5.scd) and the generator in
[`qemu-system-generator(8)`](qemu-system-generator.8.scd). To boot your
own kernel and [QEMU] instead, continue with
[bootstrap-knix.md](bootstrap-knix.md) once this page works.

The outputs below are real, trimmed to the lines that matter, with the
home directory shown as `/home/user`.

## What you need

- A Linux host with [KVM], where your user can open `/dev/kvm`.
- systemd with a user manager, which is what
  [`systemctl`] `--user` talks to.
- [Nix] with flakes enabled.
- A checkout of this repository.

## Preparation

Everything lives in one directory:

```shell
mkdir --parents ~/nixos-bootstrap
```

### A throwaway SSH key

The guest accepts this key for root, so keep it for this tutorial only.
Create it with [`ssh-keygen`]:

```shell
ssh-keygen -t ed25519 -N "" -C bootstrap -f ~/nixos-bootstrap/id_ed25519
```

### The guest

The guest is an ordinary NixOS configuration. Create
`~/nixos-bootstrap/vm/flake.nix`:

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    {
      nixosConfigurations.bootstrap = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [ ./configuration.nix ];
      };
      packages.x86_64-linux.default =
        self.nixosConfigurations.bootstrap.config.system.build.toplevel;
    };
}
```

and `~/nixos-bootstrap/vm/configuration.nix`, with the public key from
`~/nixos-bootstrap/id_ed25519.pub` pasted in:

```nix
{ modulesPath, ... }:
{
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

  boot.loader.grub.enable = false;
  fileSystems."/" = {
    device = "none";
    fsType = "tmpfs";
    options = [ "mode=0755" ];
  };
  fileSystems."/nix/store" = {
    device = "store";
    fsType = "virtiofs";
  };

  networking.hostName = "bootstrap";
  services.openssh.enable = true;
  users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
  users.users.root.initialHashedPassword = "";

  system.stateVersion = "26.05";
}
```

The [`qemu-guest.nix`] profile puts the virtio and virtiofs drivers in
the initrd. There is no boot loader: QEMU loads the kernel itself. The
store arrives over virtiofs under the tag `store`, which step 3 sets up.
The empty root password is for logging in on the console in step 7.

Build it with [`nix build`]:

```shell
cd ~/nixos-bootstrap/vm
nix build
```

`result` now holds the system. Its `kernel` and `initrd` are what QEMU
boots, and its `init` is what the kernel runs once the store is mounted:

```console
$ ls -l result/kernel result/initrd
lrwxrwxrwx 1 root root 71 Jan  1  1970 result/initrd -> /nix/store/3rxnmhxn6xfh2hwiakgircj03j4gvd1g-initrd-linux-6.18.57/initrd
lrwxrwxrwx 1 root root 65 Jan  1  1970 result/kernel -> /nix/store/maq0fxv0r4p9bac4haqy45c4vhlc33ag-linux-6.18.57/bzImage
```

### The generator

From the checkout of this repository:

```shell
nix build --out-link ~/nixos-bootstrap/generator
```

systemd runs user generators only from root-owned directories
([`systemd.generator(7)`]), so a user manager never runs this one on its
own. Two user units do it instead:

```console
$ systemctl --user enable --now \
    ~/nixos-bootstrap/generator/lib/systemd/user/qemu-system-generator.service \
    ~/nixos-bootstrap/generator/lib/systemd/user/qemu-system-generator.path
Created symlink '/home/user/.config/systemd/user/qemu-system-generator.service' → '/home/user/nixos-bootstrap/generator/lib/systemd/user/qemu-system-generator.service'.
Created symlink '/home/user/.config/systemd/user/default.target.wants/qemu-system-generator.service' → '/home/user/nixos-bootstrap/generator/lib/systemd/user/qemu-system-generator.service'.
Created symlink '/home/user/.config/systemd/user/qemu-system-generator.path' → '/home/user/nixos-bootstrap/generator/lib/systemd/user/qemu-system-generator.path'.
Created symlink '/home/user/.config/systemd/user/default.target.wants/qemu-system-generator.path' → '/home/user/nixos-bootstrap/generator/lib/systemd/user/qemu-system-generator.path'.
```

The service turns every `.qemu` file in `~/.config/qemu/systemd/` into
units and reloads the manager; it runs at login, and the
[`systemd.path(5)`] unit runs it again whenever that directory changes.
A save takes a moment to land, so every step below regenerates by hand
before starting the machine, which waits until the reload is done:

```shell
systemctl --user start qemu-system-generator.service
```

## 1. A machine

Create `~/.config/qemu/systemd/bootstrap.qemu`:

```ini
[QEMU]
machine=q35
accel=kvm
cpu=host
m=2G
smp=2
nographic=
serial=mon:stdio
```

Each line of `[QEMU]` is one QEMU option with its dash removed, passed on
as written: `m=2G` is `-m 2G`, and `nographic=` with an empty value is
the bare flag `-nographic`. [`qemu-system-x86_64(1)`] documents them all.
The file is named after the machine, and the unit after the file:

```shell
systemctl --user start qemu-system-generator.service
systemctl --user start qemu-system@bootstrap.service
journalctl --user-unit=qemu-system@bootstrap.service --follow
```

[`journalctl`] shows the serial console. With nothing to boot, the
firmware tries the disk, the CD and the network, and gives up:

```console
SeaBIOS (version rel-1.17.0-0-gb52ca86e094d-prebuilt.qemu.org)
Booting from Hard Disk...
Boot failed: could not read the boot disk
Booting from DVD/CD...
Boot failed: Could not read from CDROM (code 0003)
Booting from ROM...
iPXE (PCI 00:02.0) starting execution...ok
...
Nothing to boot: No such file or directory (http://ipxe.org/2d03e13b)
No more network devices
Boot failed: could not read the boot disk
No bootable device.
```

`systemctl --user stop` would press the power button and wait two
minutes for a guest that is not there. Kill it instead; QEMU exits on
`SIGTERM`:

```shell
systemctl --user kill qemu-system@bootstrap.service
```

## 2. A kernel

Point QEMU at the closure's kernel and initrd. `%h` is your home
directory, one of the [specifiers] systemd expands in a unit:

```diff
 nographic=
 serial=mon:stdio
+kernel=%h/nixos-bootstrap/vm/result/kernel
+initrd=%h/nixos-bootstrap/vm/result/initrd
+append=console=ttyS0
```

Regenerate and start as in step 1. Linux boots, and the initrd stops
where it looks for the store:

```console
[    0.000000] Linux version 6.18.57 (nixbld@localhost) (gcc (GCC) 16.2.0, GNU ld (GNU Binutils) 2.46) #1-NixOS SMP PREEMPT_DYNAMIC @1792058300
[    0.000000] Command line: console=ttyS0
...
[    3.121002] virtio-fs: tag <store> not found
[FAILED] Failed to mount /sysroot/nix/store.
[DEPEND] Dependency failed for Initrd File Systems.
[DEPEND] Dependency failed for Find NixOS closure.
...
You are in emergency mode. After logging in, type "journalctl -xb" to view
```

Kill it as in step 1.

## 3. The store

Two things are missing: the store under the tag `store`, and `init=`,
which tells the initrd which system in the store to boot:

```console
$ readlink ~/nixos-bootstrap/vm/result
/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f
```

Add both, with your own path in `init=`:

```diff
 kernel=%h/nixos-bootstrap/vm/result/kernel
 initrd=%h/nixos-bootstrap/vm/result/initrd
-append=console=ttyS0
+append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
+
+[virtiofsd]
+shared-dir=/nix/store
+tag=store
+sandbox=namespace
+uid-map=:0:%U:1:
+gid-map=:0:%G:1:
```

A `[virtiofsd]` section is one [virtiofsd] daemon, one share. Its keys
are the long options of `virtiofsd --help` without the dashes. The
generator runs the daemon from its own socket unit and adds the QEMU
lines that connect to it. `sandbox=`, `uid-map=` and `gid-map=` let it
run without root: `%U` and `%G` are your user and group ids, which the
guest's root maps to.

Regenerate and start. NixOS boots in about 14 seconds:

```console
[  OK  ] Mounted /sysroot/nix/store.
...
[  OK  ] Started SSH Daemon.
[  OK  ] Reached target Multi-User System.

<<< Welcome to NixOS 26.11.20261020.02c087f (x86_64) - ttyS0 >>>
```

The daemon came up with the machine:

```console
$ systemctl --user list-units 'virtiofsd@bootstrap-*'
  UNIT                              LOAD   ACTIVE SUB     DESCRIPTION
  virtiofsd@bootstrap-store.service loaded active running virtiofsd for bootstrap-store
  virtiofsd@bootstrap-store.socket  loaded active running virtiofsd socket for bootstrap-store
```

Now the guest answers the power button, so stop it the normal way:

```shell
systemctl --user stop qemu-system@bootstrap.service
```

```console
[  OK  ] Reached target System Power Off.
[   15.420532] reboot: Power down
Stopped qemu-system@bootstrap.service - QEMU System Virtual Machine bootstrap.
```

That took 1.7 seconds. The daemon stops with the machine, and its
socket stays listening for the next start.

## 4. A network

QEMU's user-mode network needs no privileges. Forward a local port to
the guest's SSH:

```diff
 append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
+nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
```

Regenerate, restart with `systemctl --user restart`, and log in with
[`ssh`]:

```console
$ ssh -i ~/nixos-bootstrap/id_ed25519 -p 10022 root@127.0.0.1 uname -r
6.18.57
```

## 5. vsock

A vsock device gives the guest a context id, a socket address the host
can reach without any network:

```diff
 nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
+device=vhost-vsock-pci-non-transitional,guest-cid=42
```

The generated unit registers each machine with your user's
[`systemd-machined.service(8)`]. With a vsock device it records the id,
and [`machinectl`] lists the machine:

```console
$ machinectl --user list
MACHINE   CLASS SERVICE     OS VERSION ADDRESSES
bootstrap vm    qemu-system -  -       -
```

NixOS listens for SSH on vsock by itself, through
[`systemd-ssh-generator(8)`], and [`systemd-ssh-proxy(1)`] lets `ssh`
on the host reach a registered machine by name:

```console
$ ssh -i ~/nixos-bootstrap/id_ed25519 root@machine/bootstrap systemctl is-system-running
running
```

`machine/` needs the `ssh` configuration that systemd ships as
`20-systemd-ssh-proxy.conf`, which most distributions link into
`/etc/ssh/ssh_config.d/`. Without it, step 4's forwarded port still
works.

## 6. Poweroff over SSH

`[VM]` holds what is not a QEMU option. With `SshPrivateKey=`, stopping
the machine first runs `systemctl poweroff` in the guest over vsock, and
presses the power button only if that fails:

```diff
+[VM]
+SshPrivateKey=%h/nixos-bootstrap/id_ed25519
+
 [QEMU]
 machine=q35
```

```console
$ systemctl --user stop qemu-system@bootstrap.service
...
Connection to vsock/42 closed by remote host.
...
[  OK  ] Reached target System Power Off.
[   16.084116] reboot: Power down
```

## 7. A console

The serial console goes to the journal, which you read but cannot type
into. Give the guest a second console, a virtio console on a socket in
the machine's runtime directory. `%t` is that directory's root and `%i`
the machine name:

```diff
-append=console=ttyS0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
+append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
 nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
 device=vhost-vsock-pci-non-transitional,guest-cid=42
+device=virtio-serial-pci-non-transitional
+chardev=socket,id=console0,path=%t/qemu-system/%i/console.sock,server=on,wait=off
+device=virtconsole,chardev=console0
```

Restart, then attach with [`socat`]. Press Enter for the prompt, log in
as root with no password, and leave with Ctrl-O:

```console
$ socat -,rawer,escape=0x0f UNIX-CONNECT:$XDG_RUNTIME_DIR/qemu-system/bootstrap/console.sock

bootstrap login: root
[root@bootstrap:~]# uname -r
6.18.57
```

## 8. The closure in a drop-in

`kernel=` and `initrd=` follow the `result` link, but `init=` names one
build, and every `nix build` makes a new one. Move `append=` to a
drop-in: the files in `bootstrap.qemu.d/` are appended to
`bootstrap.qemu`, so a command can rewrite that one line without
touching the file you edit by hand.

```diff
 initrd=%h/nixos-bootstrap/vm/result/initrd
-append=console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init
 nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
```

```shell
mkdir --parents ~/.config/qemu/systemd/bootstrap.qemu.d
cat > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf <<EOF
[QEMU]
append=console=ttyS0 console=hvc0 init=$(readlink ~/nixos-bootstrap/vm/result)/init
EOF
```

[`readlink`] fills in the path. To see what the generator makes of the
two files without writing anything:

```console
$ ~/nixos-bootstrap/generator/bin/qemu-system-generator --user --dryrun
---qemu-system@bootstrap.service---
...
    -append "console=ttyS0 console=hvc0 init=/nix/store/i7s2903dhm9g5z73vkcbpggmv1iqnvwy-nixos-system-bootstrap-26.11.20261020.02c087f/init" \
...
---virtiofsd@bootstrap-store.socket---
...
---virtiofsd@bootstrap-store.service---
...
```

The path unit does not watch the drop-in directory, so regenerate by
hand after rewriting it.

## 9. systemd settings

`[Unit]`, `[Service]` and `[Install]` are copied into the generated
service, so the machine takes a description, [resource limits] and an
install target like any unit:

```diff
 gid-map=:0:%G:1:
+
+[Unit]
+Description=NixOS bootstrap guest
+
+[Service]
+MemoryMax=3G
+CPUWeight=50
+
+[Install]
+WantedBy=machines.target
```

```console
$ systemctl --user status qemu-system@bootstrap.service
● qemu-system@bootstrap.service - NixOS bootstrap guest
     Loaded: loaded (/home/user/.config/qemu/systemd/bootstrap.qemu; enabled-runtime; preset: enabled)
     Active: active (running) since Thu 2026-10-22 16:46:55 CEST; 13s ago
     Memory: 340.4M (max: 3G, available: 2.6G, peak: 341.5M)
```

With `WantedBy=machines.target`, `systemctl --user start
machines.target` starts the machine too. Every generated machine is
also part of [`machines.target`], so `systemctl --user stop
machines.target` stops all of them, not only this one.

## 10. A share of your own

One more `[virtiofsd]` section is one more share:

```shell
mkdir --parents ~/nixos-bootstrap/share
echo "hello from the host" > ~/nixos-bootstrap/share/hello.txt
```

```diff
 gid-map=:0:%G:1:

+[virtiofsd]
+shared-dir=%h/nixos-bootstrap/share
+tag=host
+sandbox=namespace
+uid-map=:0:%U:1:
+gid-map=:0:%G:1:
+
 [Unit]
```

The guest mounts it by tag, so `configuration.nix` changes too:

```diff
   fileSystems."/nix/store" = {
     device = "store";
     fsType = "virtiofs";
   };
+  fileSystems."/mnt/host" = {
+    device = "host";
+    fsType = "virtiofs";
+  };
```

Rebuild, rewrite the drop-in as in step 8, regenerate and restart:

```console
$ ssh -i ~/nixos-bootstrap/id_ed25519 root@machine/bootstrap 'cat /mnt/host/hello.txt; touch /mnt/host/from-guest'
hello from the host
$ ls -l ~/nixos-bootstrap/share
-rw-r--r-- 1 user user  0 Oct 22 16:47 from-guest
-rw-rw-r-- 1 user user 20 Oct 22 16:47 hello.txt
```

The guest's root wrote `from-guest`; on the host it is yours.

## Notes

- `journalctl --user-unit=` finds a user service's messages;
  `journalctl --user --unit=` may not, depending on the journal's
  configuration.
- A machine stopped in the first seconds of its boot can neither take
  the SSH poweroff nor answer the power button yet. The stop then waits
  for `TimeoutStopSec=`, two minutes, before killing it;
  `systemctl --user kill` is quicker.
- In a `[virtiofsd]` section, `%i` is `name-tag`, the daemon's
  instance, not the machine name.
- A `qemu-system@.service` template of your own in
  `~/.config/systemd/user/` serves any machine the generator has not
  written, and drop-ins under `qemu-system@name.service.d/` apply to
  generated units too.

[NixOS]: https://nixos.org/manual/nixos/stable/
[systemd]: https://systemd.io
[nixpkgs]: https://github.com/NixOS/nixpkgs
[virtiofs]: https://virtio-fs.gitlab.io
[QEMU]: https://www.qemu.org/docs/master/system/invocation.html
[KVM]: https://docs.kernel.org/virt/kvm/index.html
[`systemctl`]: https://www.freedesktop.org/software/systemd/man/latest/systemctl.html
[Nix]: https://nix.dev/manual/nix/latest/
[`ssh-keygen`]: https://man7.org/linux/man-pages/man1/ssh-keygen.1.html
[`qemu-guest.nix`]: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/profiles/qemu-guest.nix
[`nix build`]: https://nix.dev/manual/nix/latest/command-ref/new-cli/nix3-build
[`systemd.generator(7)`]: https://www.freedesktop.org/software/systemd/man/latest/systemd.generator.html
[`systemd.path(5)`]: https://www.freedesktop.org/software/systemd/man/latest/systemd.path.html
[`qemu-system-x86_64(1)`]: https://www.qemu.org/docs/master/system/invocation.html
[`journalctl`]: https://www.freedesktop.org/software/systemd/man/latest/journalctl.html
[specifiers]: https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#Specifiers
[virtiofsd]: https://gitlab.com/virtio-fs/virtiofsd
[`ssh`]: https://man7.org/linux/man-pages/man1/ssh.1.html
[`systemd-machined.service(8)`]: https://www.freedesktop.org/software/systemd/man/latest/systemd-machined.service.html
[`machinectl`]: https://www.freedesktop.org/software/systemd/man/latest/machinectl.html
[`systemd-ssh-generator(8)`]: https://www.freedesktop.org/software/systemd/man/latest/systemd-ssh-generator.html
[`systemd-ssh-proxy(1)`]: https://www.freedesktop.org/software/systemd/man/latest/systemd-ssh-proxy.html
[`socat`]: http://www.dest-unreach.org/socat/doc/socat.html
[`readlink`]: https://man7.org/linux/man-pages/man1/readlink.1.html
[resource limits]: https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html
[`machines.target`]: https://www.freedesktop.org/software/systemd/man/latest/systemd.special.html
