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
      # The generator needs systemd, but the docs build anywhere Sphinx does,
      # so their apps also cover macOS.
      docsSystems = systems ++ [ "aarch64-darwin" ];
      forSystems =
        systems': f:
        nixpkgs.lib.genAttrs systems' (
          system:
          f (
            import nixpkgs {
              inherit system;
              config = { };
              overlays = [ ];
            }
          )
        );
      forAllSystems = forSystems systems;

      # Sphinx and its extensions, kept to the major versions in
      # docs/requirements.txt, which Read the Docs installs.
      docsPython =
        pkgs:
        pkgs.python3.withPackages (ps: [
          ps.sphinx
          ps.sphinx-autobuild
          ps.sphinx-copybutton
          ps.sphinx-design
          ps.pydata-sphinx-theme
        ]);
    in
    {
      overlays.default = final: prev: {
        qemu-system-generator = final.callPackage ./nix/package.nix { };
      };

      packages = forAllSystems (pkgs: rec {
        qemu-system-generator = pkgs.callPackage ./nix/package.nix { };
        default = qemu-system-generator;
        dagu = pkgs.callPackage ./nix/dagu.nix { };
        act = pkgs.callPackage ./nix/act.nix { };
      });

      apps = forSystems docsSystems (
        pkgs:
        let
          # Sphinx writes into the checkout, so the docs apps run from its
          # root wherever they are started.
          mkDocsApp =
            {
              name,
              description,
              text,
            }:
            {
              type = "app";
              program = pkgs.lib.getExe (
                pkgs.writeShellApplication {
                  inherit name;
                  runtimeInputs = [
                    pkgs.git
                    (docsPython pkgs)
                    pkgs.scdoc
                    pkgs.mandoc
                  ];
                  text = ''
                    root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
                      echo "${name}: run this inside the qemu-system-generator checkout" >&2
                      exit 1
                    }
                    cd "$root"
                    ${text}
                  '';
                }
              );
              meta = { inherit description; };
            };
        in
        pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          default = {
            type = "app";
            program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/qemu-system-generator";
          };
        }
        // {
          docs = mkDocsApp {
            name = "qemu-system-generator-docs";
            description = "Render the documentation to docs/_build/html with Sphinx";
            text = ''
              sphinx-build --write-all docs docs/_build/html
              echo "docs ready: docs/_build/html/index.html"
            '';
          };

          serve = mkDocsApp {
            name = "qemu-system-generator-serve";
            description = "Live-render the docs on 127.0.0.1, rebuilding on save (arg: port)";
            text = ''
              port="''${1:-8001}"
              sphinx-autobuild --write-all docs docs/_build/html --host 127.0.0.1 --port "$port"
            '';
          };
        }
      );

      nixosModules.default =
        { ... }:
        {
          imports = [ ./nix/module.nix ];
          nixpkgs.overlays = [ self.overlays.default ];
        };

      devShells = forAllSystems (pkgs: {
        default =
          let
            cacert = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
          in
          pkgs.mkShell {
            inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.rustfmt
              pkgs.clippy
              pkgs.rust-analyzer
              pkgs.scdoc
              # mandoc's own man would shadow the host's, and tests/verify needs
              # that one to find the systemd and QEMU pages Documentation= cites.
              (pkgs.runCommand "mandoc-only" { } ''
                mkdir --parents $out/bin
                ln --symbolic ${pkgs.mandoc}/bin/mandoc $out/bin/
              '')
              (docsPython pkgs)
              pkgs.markdownlint-cli2
              pkgs.python3Packages.doc8
              pkgs.systemd
              pkgs.nixfmt
              pkgs.python3
              # What steps/ and dags/ run besides, under nix develop
              # --ignore-env, which keeps only HOME and XDG_RUNTIME_DIR.
              self.packages.${pkgs.stdenv.hostPlatform.system}.dagu
              pkgs.cloud-utils
              pkgs.cpio
              pkgs.curl
              pkgs.gettext
              pkgs.git
              pkgs.jq
              pkgs.nix
              pkgs.openssh
              pkgs.procps
              pkgs.qemu-utils
              pkgs.socat
              pkgs.util-linux
            ];
            env = {
              RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
              NIX_SSL_CERT_FILE = cacert;
            };
            # nix develop never takes SSL_CERT_FILE from a shell.
            shellHook = ''
              export SSL_CERT_FILE=${cacert}
            '';
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

          # The pages must build without a warning, as Read the Docs builds
          # them (.readthedocs.yaml).
          docs =
            pkgs.runCommand "${package.pname}-docs"
              {
                src = pkgs.lib.fileset.toSource {
                  root = ./.;
                  fileset = pkgs.lib.fileset.unions [
                    ./docs
                    ./man
                  ];
                };
                nativeBuildInputs = [
                  (docsPython pkgs)
                  pkgs.scdoc
                  pkgs.mandoc
                ];
              }
              ''
                cp --recursive $src source
                chmod --recursive u+w source
                sphinx-build --fail-on-warning --builder html source/docs $out
              '';

          # markdownlint-cli2 reads .markdownlint-cli2.jsonc and doc8 reads
          # doc8.ini.
          markdown =
            pkgs.runCommand "${package.pname}-markdown"
              {
                src = pkgs.lib.fileset.toSource {
                  root = ./.;
                  fileset = pkgs.lib.fileset.unions [
                    (pkgs.lib.fileset.fileFilter (file: file.hasExt "md") ./.)
                    ./.markdownlint-cli2.jsonc
                  ];
                };
                nativeBuildInputs = [ pkgs.markdownlint-cli2 ];
              }
              ''
                cd $src
                markdownlint-cli2 '**/*.md'
                touch $out
              '';

          rst =
            pkgs.runCommand "${package.pname}-rst"
              {
                src = pkgs.lib.fileset.toSource {
                  root = ./.;
                  fileset = pkgs.lib.fileset.unions [
                    ./docs
                    ./doc8.ini
                  ];
                };
                nativeBuildInputs = [ pkgs.python3Packages.doc8 ];
              }
              ''
                cd $src
                doc8 docs
                touch $out
              '';

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
                export QEMU_SYSTEM_GENERATOR_UNIT_DIRS=${src}/examples
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
