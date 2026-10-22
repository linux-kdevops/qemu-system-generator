<!-- SPDX-License-Identifier: copyleft-next-0.3.1 -->
# Bootstrap with your own kernel and QEMU

This page continues [bootstrap.md](bootstrap.md) from its last step and
boots the same guest on a [Linux] kernel and a [QEMU] you build
yourself. Run that page first; everything here changes its
`bootstrap.qemu` and reuses `~/nixos-bootstrap`.

The outputs below are real, trimmed to the lines that matter, with the
home directory shown as `/home/user`.

## Why knix

A [NixOS] system from [nixpkgs] carries its kernel: the initrd holds that
kernel's modules, and `/lib/modules` in the running system is that
kernel's too. nixpkgs can build a kernel from your tree, but as a [Nix]
derivation, which rebuilds from scratch after every change.

[knix] keeps the kernel out of the closure. Its `imageless` backend
builds a NixOS system with no kernel of its own and an initrd with no
modules, which expects the boot-critical drivers built in and mounts
`/lib/modules` over virtiofs under the tag `modules`. You build the
kernel in your tree with `make` as usual, and the same closure boots
every build. knix also provides the `build-kernel` and `build-qemu`
shells, which bring the toolchains.

## Preparation

### The kernel

Configure the release as a KVM guest with virtiofs built in, build it
and install its modules next to it, inside the `build-kernel` shell
([`nix develop`]):

```shell
git clone --depth 1 --branch v7.3 \
    https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git ~/nixos-bootstrap/linux
cd ~/nixos-bootstrap/linux
K="nix develop github:linux-kdevops/knix#build-kernel --command"
B=$HOME/nixos-bootstrap/linux-build
$K make O=$B defconfig kvm_guest.config
$K scripts/config --file $B/.config \
    --enable FUSE_FS --enable VIRTIO_FS \
    --enable VSOCKETS --enable VIRTIO_VSOCKETS
$K make O=$B olddefconfig
$K make O=$B --jobs=$(nproc)
$K make O=$B INSTALL_MOD_PATH=$HOME/nixos-bootstrap/linux-install modules_install
```

`kvm_guest.config` turns on the virtio drivers; the
[kernel build documentation] covers `O=` and the configuration targets.
The kernel is `~/nixos-bootstrap/linux-build/arch/x86/boot/bzImage`, and
the modules are under `~/nixos-bootstrap/linux-install/lib/modules/`.

### QEMU

Build the release inside the `build-qemu` shell and install it in the
workspace ([QEMU build system]):

```shell
git clone --depth 1 --branch v11.1.2 \
    https://gitlab.com/qemu-project/qemu.git ~/nixos-bootstrap/qemu
mkdir --parents ~/nixos-bootstrap/qemu-build
cd ~/nixos-bootstrap/qemu-build
Q="nix develop github:linux-kdevops/knix#build-qemu --command"
$Q ~/nixos-bootstrap/qemu/configure --prefix=$HOME/nixos-bootstrap/qemu-install \
    --target-list=x86_64-softmmu --enable-kvm --enable-slirp --disable-docs
$Q make --jobs=$(nproc) install
```

```console
$ ~/nixos-bootstrap/qemu-install/bin/qemu-system-x86_64 --version
QEMU emulator version 11.1.2 (v11.1.2)
```

### The guest

Start from knix's template ([`nix flake init`]) and point its input at
knix upstream:

```shell
mkdir --parents ~/nixos-bootstrap/knix
cd ~/nixos-bootstrap/knix
nix flake init --template github:linux-kdevops/knix
```

```diff
-    knix.url = "path:/path/to/knix";
+    knix.url = "github:linux-kdevops/knix";
```

Give the guest the same name, key and share as before in `default.nix`,
with your public key pasted in:

```diff
     #   knix.nixosModules.testSuites.blktests
+    knix.nixosModules.mounts.shares
   ];

-  networking.hostName = "vm";
+  knix.shares."/mnt/host".tag = "host";
+
+  networking.hostName = "bootstrap";

...
   # SSH keys for the root and unprivileged accounts.
-  # users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 ..." ];
-  # users.users.${config.knix.user.name}.openssh.authorizedKeys.keys = [ "ssh-ed25519 ..." ];
+  users.users.root.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
+  users.users.${config.knix.user.name}.openssh.authorizedKeys.keys = [ "ssh-ed25519 AAAA... bootstrap" ];
```

```shell
nix build .#toplevel
```

This closure has no `kernel` or `initrd` link. Its `boot.json` names the
initrd and `init`:

```console
$ jq '."org.nixos.bootspec.v1" | {initrd, init}' result/boot.json
{
  "initrd": "/nix/store/pbx7c8f26kng743zyrabx8p9qplpi6zj-initrd-linux-6.18.34/initrd",
  "init": "/nix/store/m907x9z5b9cab8ab57cgs1sjsh8i4ghr-nixos-system-bootstrap-26.05.20260608.bd0ff2d/init"
}
```

The initrd's name mentions a nixpkgs kernel, but knix empties its module
lists, so it boots any kernel that has the boot-critical drivers built
in.

## 1. Your kernel

The kernel comes from your build, the initrd from the knix closure, and
`/lib/modules` from a second share:

```diff
 nographic=
 serial=mon:stdio
-kernel=%h/nixos-bootstrap/vm/result/kernel
-initrd=%h/nixos-bootstrap/vm/result/initrd
+kernel=%h/nixos-bootstrap/linux-build/arch/x86/boot/bzImage
 nic=user,model=virtio-net-pci-non-transitional,hostfwd=tcp:127.0.0.1:10022-:22
...
 gid-map=:0:%G:1:
+
+[virtiofsd]
+shared-dir=%h/nixos-bootstrap/linux-install/lib/modules
+tag=modules
+sandbox=namespace
+uid-map=:0:%U:1:
+gid-map=:0:%G:1:

 [virtiofsd]
 shared-dir=%h/nixos-bootstrap/share
```

The drop-in now carries the initrd too. [`jq`] writes it from
`boot.json`, and knix wants `root=tmpfs` on the command line:

```shell
jq --raw-output '."org.nixos.bootspec.v1" |
    "[QEMU]\ninitrd=\(.initrd)\nappend=root=tmpfs console=ttyS0 console=hvc0 init=\(.init)"' \
    ~/nixos-bootstrap/knix/result/boot.json \
    > ~/.config/qemu/systemd/bootstrap.qemu.d/10-closure.conf
systemctl --user start qemu-system-generator.service
systemctl --user restart qemu-system@bootstrap.service
```

```console
$ ssh -i ~/nixos-bootstrap/id_ed25519 root@machine/bootstrap \
    'uname -r; findmnt --noheadings --output=TARGET,SOURCE,FSTYPE --types=virtiofs'
7.3.0
/lib/modules modules virtiofs
/nix/store   store   virtiofs
└─/nix/store store   virtiofs
/mnt/host    host    virtiofs
```

After a kernel rebuild, `make modules_install` again and restart. After
a closure rebuild, rerun the `jq` command and regenerate.

## 2. Your QEMU

`QemuBinary=` replaces the QEMU the generator was built with. It becomes
the first word of `ExecStart=`, which must be a literal absolute path,
so write your home directory out:

```diff
 [VM]
+QemuBinary=/home/user/nixos-bootstrap/qemu-install/bin/qemu-system-x86_64
 SshPrivateKey=%h/nixos-bootstrap/id_ed25519
```

```console
$ readlink /proc/$(systemctl --user show --property=MainPID --value qemu-system@bootstrap.service)/exe
/home/user/nixos-bootstrap/qemu-install/bin/qemu-system-x86_64
```

The guest boots in about 7 seconds and stops in 1.7.

[Linux]: https://www.kernel.org
[QEMU]: https://www.qemu.org/docs/master/system/invocation.html
[NixOS]: https://nixos.org/manual/nixos/stable/
[nixpkgs]: https://github.com/NixOS/nixpkgs
[Nix]: https://nix.dev/manual/nix/latest/
[knix]: https://github.com/linux-kdevops/knix
[`nix develop`]: https://nix.dev/manual/nix/latest/command-ref/new-cli/nix3-develop
[kernel build documentation]: https://docs.kernel.org/admin-guide/README.html
[QEMU build system]: https://www.qemu.org/docs/master/devel/build-system.html
[`nix flake init`]: https://nix.dev/manual/nix/latest/command-ref/new-cli/nix3-flake-init
[`jq`]: https://jqlang.org/manual/
