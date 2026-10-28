# SPDX-License-Identifier: copyleft-next-0.3.1
#
# The two kernels and QEMU the knix machines boot, built as derivations
# from the release tarballs, so equal inputs are built once, and the
# static BusyBox of the fast machine's initramfs.
{
  inputs = {
    knix.url = "github:linux-kdevops/knix";
    kfragments = {
      url = "github:linux-kdevops/kfragments";
      flake = false;
    };
  };

  outputs =
    { knix, kfragments, ... }:
    let
      pkgs = knix.inputs.nixpkgs.legacyPackages.x86_64-linux;
      src = pkgs.fetchurl {
        url = "https://cdn.kernel.org/pub/linux/kernel/v7.x/linux-7.3.tar.xz";
        sha256 = "0ls902gz6h1ydcnwx2zzdcglg52f14hmaxk2ncg341hfy4j3qdl7";
      };
      version = "7.3";
    in
    {
      packages.x86_64-linux = {
        busybox = pkgs.pkgsStatic.busybox;

        kernel = knix.lib.buildKernel pkgs {
          inherit src version;
          configure = ''
            make O=$B defconfig kvm_guest.config
            scripts/config --file $B/.config \
                --enable FUSE_FS --enable VIRTIO_FS \
                --enable VSOCKETS --enable VIRTIO_VSOCKETS
            make O=$B olddefconfig
          '';
        };

        fast-kernel = knix.lib.buildKernel pkgs {
          inherit src version;
          configure = ''
            F=${kfragments}/kernel/configs
            make O=$B allnoconfig
            scripts/kconfig/merge_config.sh -y -n -O $B $B/.config \
                $F/core/64bit.config $F/core/core.config $F/core/modules.config \
                $F/core/systemd.config $F/core/initrd.config $F/arch/x86_64.config \
                $F/core/acpi-poweroff.config $F/virt/virtio-fs.config \
                $F/virt/virtio-net.config $F/virt/vsock.config $F/core/localversion.config \
                $F/builtin/core/systemd.config $F/builtin/virt/virtio-fs.config \
                $F/builtin/virt/virtio-net.config $F/builtin/virt/vsock.config \
                ${./fast.config}
            make O=$B olddefconfig
          '';
        };

        qemu = knix.lib.buildQemu pkgs {
          src = pkgs.fetchurl {
            url = "https://download.qemu.org/qemu-11.1.0.tar.xz";
            hash = "sha256-buHRph9oISR2snEIwm2l9EncCbYm1C+CeboNwuCPqFg=";
          };
          version = "11.1.0";
          configureFlags = [
            "--target-list=x86_64-softmmu"
            "--enable-kvm"
            "--enable-slirp"
            "--disable-docs"
          ];
        };
      };
    };
}
