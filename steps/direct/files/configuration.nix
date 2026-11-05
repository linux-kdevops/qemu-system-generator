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
  users.users.root.initialHashedPassword = "";

  nixpkgs.hostPlatform = "x86_64-linux";
  system.stateVersion = "26.05";
}
