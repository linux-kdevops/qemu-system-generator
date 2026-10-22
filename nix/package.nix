# SPDX-License-Identifier: copyleft-next-0.3.1
{
  lib,
  rustPlatform,
  installShellFiles,
  scdoc,
  qemu,
  virtiofsd,
  socat,
  systemd,
  openssh,
  # Directory holding the qemu-system-* binaries the generated units run
  # when a file sets no QemuBinary=.
  qemuBinDir ? "${qemu}/bin",
  # The virtiofsd binary the generated units run when a file sets no
  # VirtiofsdBinary=.
  virtiofsdBinary ? "${virtiofsd}/bin/virtiofsd",
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "qemu-system-generator";
  version = (lib.importTOML ../Cargo.toml).package.version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../build.rs
      ../src
      ../data
      ../docs
      ../examples
      ../tests
      ../README.md
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    installShellFiles
    scdoc
  ];

  # The paths the generated units call; see build.rs.
  env = {
    QEMU_BINDIR = qemuBinDir;
    VIRTIOFSD = virtiofsdBinary;
    QMP_POWERDOWN = "${placeholder "out"}/share/qemu/systemd/qmp-powerdown";
    SOCAT = "${socat}/bin/socat";
    VARLINKCTL = "${systemd}/bin/varlinkctl";
    SSH = "${openssh}/bin/ssh";
  };

  postInstall = ''
    scdoc < docs/qemu-systemd.unit.5.scd > qemu-systemd.unit.5
    scdoc < docs/qemu-system-generator.8.scd > qemu-system-generator.8
    installManPage qemu-systemd.unit.5 qemu-system-generator.8
    mkdir --parents $out/share/qemu/systemd $out/lib/systemd/system-generators $out/lib/systemd/user-generators
    install --mode=0644 data/qmp-powerdown $out/share/qemu/systemd/
    ln --symbolic ../../../bin/qemu-system-generator $out/lib/systemd/system-generators/qemu-system-generator
    ln --symbolic ../../../bin/qemu-system-generator $out/lib/systemd/user-generators/qemu-system-generator
    mkdir --parents $out/share/doc/qemu-system-generator
    cp --recursive examples $out/share/doc/qemu-system-generator/
  '';

  meta = {
    description = "systemd generator that turns .qemu files into QEMU services";
    longDescription = ''
      Reads .qemu files, systemd unit-file syntax with a [QEMU] section
      that passes QEMU's command line through, and generates the service,
      socket and virtiofsd units that run the machine under systemd.
    '';
    license = {
      shortName = "copyleft-next-0.3.1";
      fullName = "copyleft-next 0.3.1";
      spdxId = "copyleft-next-0.3.1";
      url = "https://spdx.org/licenses/copyleft-next-0.3.1.html";
      free = true;
      redistributable = true;
    };
    mainProgram = "qemu-system-generator";
    platforms = lib.platforms.linux;
  };
})
