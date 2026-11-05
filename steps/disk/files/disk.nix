# SPDX-License-Identifier: copyleft-next-0.3.1
{
  boot.kernelParams = [
    "console=ttyS0"
    "systemd.tty.term.console=vt220"
    "systemd.tty.rows.console=24"
    "systemd.tty.columns.console=80"
    "quiet"
  ];
  boot.loader.timeout = 0;
  image.modules.qemu.image.baseName = "bootstrap";
  image.modules.qemu-efi.image.baseName = "bootstrap";
}
