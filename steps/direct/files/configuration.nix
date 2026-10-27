# SPDX-License-Identifier: copyleft-next-0.3.1
{ lib, modulesPath, ... }:
{
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

  networking.hostName = "$name";
  networking.useNetworkd = true;
  systemd.network.networks."10-lan" = {
    matchConfig.MACAddress = "52:54:00:00:00:$cid";
    address = [ "10.0.100.$cid/24" ];
  };
  services.openssh.enable = true;
  services.openssh.hostKeys = [
    { type = "ed25519"; path = "/etc/ssh/ssh_host_ed25519_key"; }
  ];
  security.pam.services.sshd.startSession = lib.mkForce false;
  users.users.root.openssh.authorizedKeys.keys = [ "$key" ];

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

  system.stateVersion = "26.05";
}
