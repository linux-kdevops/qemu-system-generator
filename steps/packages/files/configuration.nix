# SPDX-License-Identifier: copyleft-next-0.3.1
{
  lib,
  modulesPath,
  pkgs,
  ...
}:
{
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

  environment.systemPackages = [
    pkgs.damo
    pkgs.fio
    pkgs.nfstest
    pkgs.pynfs
    pkgs.xfstests
  ];
  networking.hostName = "bootstrap-packages";
  # nfstest finds the guest's address through an IPv4 route, and with
  # dhcpcd's default of any address, boot finishes on slirp's IPv6 router
  # advertisement before the DHCPv4 lease.
  networking.dhcpcd.wait = "ipv4";
  services.openssh.enable = true;
  services.openssh.hostKeys = [
    {
      type = "ed25519";
      path = "/etc/ssh/ssh_host_ed25519_key";
    }
  ];
  security.pam.services.sshd.startSession = lib.mkForce false;
  users.users.root.openssh.authorizedKeys.keys = [ "$key" ];

  nixpkgs.hostPlatform = "x86_64-linux";
  system.stateVersion = "26.05";
}
