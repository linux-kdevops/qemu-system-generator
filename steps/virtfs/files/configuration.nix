# SPDX-License-Identifier: copyleft-next-0.3.1
{ lib, modulesPath, ... }:
let
  share = tag: options: {
    device = tag;
    fsType = "9p";
    options = [
      "trans=virtio"
      "version=9p2000.L"
    ]
    ++ options;
  };
in
{
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

  networking.hostName = "bootstrap-virtfs";
  services.openssh.enable = true;
  services.openssh.hostKeys = [
    { type = "ed25519"; path = "/etc/ssh/ssh_host_ed25519_key"; }
  ];
  security.pam.services.sshd.startSession = lib.mkForce false;
  users.users.root.openssh.authorizedKeys.keys = [ "$key" ];

  boot.loader.grub.enable = false;
  boot.initrd.availableKernelModules = [
    "9p"
    "9pnet_virtio"
  ];
  fileSystems."/" = {
    device = "none";
    fsType = "tmpfs";
    options = [ "mode=0755" ];
  };
  fileSystems."/nix/store" = share "store" [ "cache=loose" ];
  fileSystems."/mnt/none" = share "none" [ "msize=262144" ];
  fileSystems."/mnt/xattr" = share "xattr" [ ];
  fileSystems."/mnt/file" = share "file" [ ];
  fileSystems."/mnt/ro" = share "ro" [ ];
  fileSystems."/mnt/writeout" = share "writeout" [ "cache=mmap" ];
  fileSystems."/mnt/shorthand" = share "shorthand" [ "cache=loose" ];

  system.stateVersion = "26.05";
}
