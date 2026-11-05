# SPDX-License-Identifier: copyleft-next-0.3.1
{
  boot.kernelParams = [
    "console=ttyS0"
    "systemd.tty.term.console=vt220"
    "systemd.tty.rows.console=24"
    "systemd.tty.columns.console=80"
    "quiet"
  ];
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
  systemd.services.systemd-boot-random-seed.enable = false;
}
