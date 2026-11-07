# SPDX-License-Identifier: copyleft-next-0.3.1
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
        hash = "sha256-hzY8JPEOBjIes2J2VSEJTpRHH2v/i84taz5A858ASVM=";
      };
      version = "7.3";
    in
    {
      packages.x86_64-linux = {
        busybox = pkgs.pkgsStatic.busybox;

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
