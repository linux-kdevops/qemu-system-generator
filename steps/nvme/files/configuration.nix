# SPDX-License-Identifier: copyleft-next-0.3.1
{ lib, modulesPath, pkgs, ... }:
{
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

  networking.hostName = "bootstrap-nvme";
  services.openssh.enable = true;
  services.openssh.hostKeys = [
    { type = "ed25519"; path = "/etc/ssh/ssh_host_ed25519_key"; }
  ];
  security.pam.services.sshd.startSession = lib.mkForce false;
  users.users.root.openssh.authorizedKeys.keys = [ "$key" ];
  environment.systemPackages = [ pkgs.nvme-cli ];

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
