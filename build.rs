// SPDX-License-Identifier: copyleft-next-0.3.1
//
// Paths the generated units call, fixed when the generator is built.
// Packagers set them in the build environment; a bare name is resolved
// by systemd on its compiled-in search path (systemd.service(5),
// "Command lines").

use std::env;

const DEFAULTS: [(&str, &str); 6] = [
    ("QEMU_BINDIR", "/usr/bin"),
    ("VIRTIOFSD", "/usr/libexec/virtiofsd"),
    ("QMP_POWERDOWN", "/usr/share/qemu/systemd/qmp-powerdown"),
    ("SOCAT", "socat"),
    ("VARLINKCTL", "varlinkctl"),
    ("SSH", "ssh"),
];

/// The `qemu-system-<target>` that emulates the target the generator is
/// built for, by the names of QEMU's `configs/targets/*-softmmu.mak`.
fn qemu_target() -> String {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let little = env::var("CARGO_CFG_TARGET_ENDIAN").is_ok_and(|e| e == "little");
    match arch.as_str() {
        "x86" => "i386".to_owned(),
        "powerpc64" => "ppc64".to_owned(),
        "powerpc" => "ppc".to_owned(),
        "mips" | "mips64" if little => format!("{arch}el"),
        _ => arch,
    }
}

fn main() {
    for (var, default) in DEFAULTS {
        println!("cargo:rerun-if-env-changed={var}");
        let value = env::var(var).unwrap_or_else(|_| default.to_owned());
        if var == "QEMU_BINDIR" {
            println!("cargo:rustc-env=QEMU={value}/qemu-system-{}", qemu_target());
        }
        println!("cargo:rustc-env={var}={value}");
    }
}
