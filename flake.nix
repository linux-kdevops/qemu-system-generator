# SPDX-License-Identifier: copyleft-next-0.3.1
{
  description = "systemd generator that turns .qemu files into QEMU services";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              config = { };
              overlays = [ ];
            }
          )
        );
    in
    {
      overlays.default = final: prev: {
        qemu-system-generator = final.callPackage ./nix/package.nix { };
      };

      packages = forAllSystems (pkgs: rec {
        qemu-system-generator = pkgs.callPackage ./nix/package.nix { };
        default = qemu-system-generator;
      });

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/qemu-system-generator";
        };
      });

      nixosModules.default =
        { ... }:
        {
          imports = [ ./nix/module.nix ];
          nixpkgs.overlays = [ self.overlays.default ];
        };

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          packages = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.clippy
            pkgs.rust-analyzer
            pkgs.scdoc
            pkgs.systemd
            pkgs.nixfmt
            pkgs.python3
          ];
          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });

      checks = forAllSystems (
        pkgs:
        let
          package = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
          src = package.src;
        in
        {
          inherit package;

          clippy = package.overrideAttrs (old: {
            pname = "${old.pname}-clippy";
            nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
            buildPhase = "cargo clippy --all-targets -- --deny warnings";
            installPhase = "touch $out";
            doCheck = false;
          });

          rustfmt = pkgs.runCommand "${package.pname}-rustfmt" { nativeBuildInputs = [ pkgs.rustfmt ]; } ''
            cd ${src}
            rustfmt --check --edition 2024 src/main.rs build.rs tests/*.rs
            touch $out
          '';

          # The generated units must load in a service manager. The build
          # sandbox has no /run/systemd/, so the check runs as a user manager
          # rooted in a scratch runtime directory; the system manager's units
          # differ only by one ordering line (tests/examples.rs). It has no
          # man either, so the Documentation= references are checked by
          # tests/verify on a host instead.
          verify =
            pkgs.runCommand "${package.pname}-verify"
              {
                nativeBuildInputs = [
                  package
                  pkgs.systemd
                ];
              }
              ''
                export QEMU_SYSTEMD_UNIT_DIRS=${src}/examples
                export HOME=$PWD/home XDG_RUNTIME_DIR=$PWD/run XDG_CONFIG_HOME=$PWD/home/.config
                mkdir --parents generated home run
                qemu-system-generator --user generated
                ls generated
                SYSTEMD_UNIT_PATH="$PWD/generated:${pkgs.systemd}/example/systemd/user:" systemd-analyze --user verify --man=no generated/*.service generated/*.socket
                touch $out
              '';
        }
      );

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
