# SPDX-License-Identifier: copyleft-next-0.3.1
{
  config,
  lib,
  knix,
  ...
}:
{
  imports = [
    knix.nixosModules.profiles.devel
    ./home.nix
  ];

  networking.hostName = "bootstrap";
  systemd.network.networks = {
    "10-lan-bootstrap" = {
      matchConfig.MACAddress = "52:54:00:00:00:49";
      address = [ "10.0.100.49/24" ];
    };
    "10-lan-peer" = {
      matchConfig.MACAddress = "52:54:00:00:00:50";
      address = [ "10.0.100.50/24" ];
    };
  };
  services.openssh.hostKeys = [
    {
      type = "ed25519";
      path = "/etc/ssh/ssh_host_ed25519_key";
    }
  ];
  security.pam.services.sshd.startSession = lib.mkForce false;
  console.enable = false;
  systemd.suppressedSystemUnits = [
    "systemd-journal-catalog-update.service"
    "systemd-update-done.service"
  ];

  users.users.root.openssh.authorizedKeys.keys = [ "$key" ];
  users.users.${config.knix.user.name}.openssh.authorizedKeys.keys = [
    "$key"
  ];
}
