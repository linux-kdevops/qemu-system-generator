# SPDX-License-Identifier: copyleft-next-0.3.1
{ lib, modulesPath, ... }:
{
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

  networking.hostName = "bootstrap";
  services.openssh.enable = true;
  services.openssh.hostKeys = [
    { type = "ed25519"; path = "/etc/ssh/ssh_host_ed25519_key"; }
  ];
  security.pam.services.sshd.startSession = lib.mkForce false;
  users.users.root.openssh.authorizedKeys.keys = [ "$key" ];

  boot.kernelParams = [ "console=ttyS0" ];
  boot.loader.timeout = 0;
  image.modules.qemu-efi.image.baseName = "bootstrap";

  system.stateVersion = "26.05";
}
