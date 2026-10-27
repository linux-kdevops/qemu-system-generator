# SPDX-License-Identifier: copyleft-next-0.3.1
{ lib, knix, ... }:
{
  imports = [ ./home.nix ];

  networking.hostName = "bootstrap";
  systemd.network.networks = {
    "10-lan-bootstrap" = {
      matchConfig.MACAddress = "52:54:00:00:00:42";
      address = [ "10.0.100.42/24" ];
    };
    "10-lan-peer" = {
      matchConfig.MACAddress = "52:54:00:00:00:46";
      address = [ "10.0.100.46/24" ];
    };
  };
  services.openssh.hostKeys = [
    { type = "ed25519"; path = "/etc/ssh/ssh_host_ed25519_key"; }
  ];
  security.pam.services.sshd.startSession = lib.mkForce false;
  users.users.root.openssh.authorizedKeys.keys = [ "$key" ];
}
