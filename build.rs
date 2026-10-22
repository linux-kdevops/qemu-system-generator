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

fn main() {
    for (var, default) in DEFAULTS {
        println!("cargo:rerun-if-env-changed={var}");
        let value = env::var(var).unwrap_or_else(|_| default.to_owned());
        println!("cargo:rustc-env={var}={value}");
    }
}
