# SPDX-License-Identifier: copyleft-next-0.3.1
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.virtualisation.qemu-system-generator;
in
{
  options.virtualisation.qemu-system-generator = {
    enable = lib.mkEnableOption "the systemd generator for .qemu files";

    package = lib.mkPackageOption pkgs "qemu-system-generator" { };

    units = lib.mkOption {
      type = lib.types.attrsOf (lib.types.either lib.types.lines lib.types.path);
      default = { };
      example = lib.literalExpression ''
        {
          "fedora.qemu" = '''
            [QEMU]
            machine=q35
            accel=kvm
            m=2048
            drive=file=fedora.raw,if=virtio,format=raw
            nographic=
          ''';
          "nixos.qemu" = ./nixos.qemu;
        }
      '';
      description = ''
        Files installed under {file}`/etc/qemu/systemd/`, by name, for the
        system manager. See {manpage}`qemu-systemd.unit(5)`.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.generators.qemu-system-generator = "${cfg.package}/lib/systemd/system-generators/qemu-system-generator";
    systemd.user.generators.qemu-system-generator = "${cfg.package}/lib/systemd/user-generators/qemu-system-generator";

    environment.systemPackages = [ cfg.package ];

    environment.etc = lib.mapAttrs' (
      name: value:
      lib.nameValuePair "qemu/systemd/${name}" (
        if builtins.isPath value || lib.isStorePath value then { source = value; } else { text = value; }
      )
    ) cfg.units;
  };
}
