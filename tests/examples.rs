// SPDX-License-Identifier: copyleft-next-0.3.1

//! The examples are the conformance suite: their generated units are
//! compared with `tests/expected/dryrun.txt`. Regenerate the
//! expectation with `tests/update-expected` after a deliberate format
//! change.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_qemu-system-generator");
const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn examples() -> PathBuf {
    Path::new(ROOT).join("examples")
}

fn dryrun(scope: &str, dir: &Path) -> Output {
    Command::new(BIN)
        .args([scope, "--dryrun"])
        .env("QEMU_SYSTEMD_UNIT_DIRS", dir)
        .output()
        .expect("run generator")
}

/// The qemu-system binary the generator defaults to on this host, from
/// the directory compiled in by build.rs.
fn default_qemu() -> String {
    let target = match env::consts::ARCH {
        "x86" => "i386",
        "powerpc64" => "ppc64",
        "powerpc" => "ppc",
        other => other,
    };
    format!("{}/qemu-system-{target}", env!("QEMU_BINDIR"))
}

/// The expected output with its placeholders filled in: everything that
/// depends on the checkout or on the build environment.
fn expected() -> String {
    let text = fs::read_to_string(Path::new(ROOT).join("tests/expected/dryrun.txt")).unwrap();
    [
        ("@EXAMPLES@", examples().to_str().unwrap().to_owned()),
        ("@QEMU@", default_qemu()),
        ("@VIRTIOFSD@", env!("VIRTIOFSD").to_owned()),
        ("@QMP_POWERDOWN@", env!("QMP_POWERDOWN").to_owned()),
        ("@SOCAT@", env!("SOCAT").to_owned()),
        ("@VARLINKCTL@", env!("VARLINKCTL").to_owned()),
        ("@SSH@", env!("SSH").to_owned()),
    ]
    .iter()
    .fold(text, |text, (placeholder, value)| {
        text.replace(placeholder, value)
    })
}

#[test]
fn examples_generate_the_expected_units() {
    let out = dryrun("--system", &examples());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "generator failed: {stderr}");
    assert!(stderr.is_empty(), "unexpected warnings: {stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout != expected() {
        let got = Path::new(ROOT).join("target/dryrun.got.txt");
        fs::write(&got, stdout.as_bytes()).unwrap();
        panic!(
            "output differs from tests/expected/dryrun.txt; actual output saved to {}",
            got.display()
        );
    }
}

#[test]
fn user_scope_differs_only_in_network_ordering() {
    // The units use %t, %S and %h so that one spelling serves both
    // managers; only the system manager has a network.target to wait for.
    let system = dryrun("--system", &examples());
    let user = dryrun("--user", &examples());
    assert!(system.status.success() && user.status.success());
    let system = String::from_utf8_lossy(&system.stdout).replace("After=network.target\n", "");
    assert_eq!(system, String::from_utf8_lossy(&user.stdout));
}

#[test]
fn a_broken_file_fails_without_stopping_the_others() {
    let tmp = env::temp_dir().join(format!(
        "qemu-system-generator-examples-{}",
        std::process::id()
    ));
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join("good.qemu"), "[QEMU]\nm=1\n").unwrap();
    fs::write(
        tmp.join("bad.qemu"),
        "[QEMU]\ndevice=vhost-vsock-pci,guest-cid=x\n",
    )
    .unwrap();
    fs::write(tmp.join("share.qemu"), "[virtiofsd]\ntag=t\n").unwrap();
    let out = dryrun("--system", &tmp);
    fs::remove_dir_all(&tmp).unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr.contains("bad.qemu: [QEMU] guest-cid=x is not a decimal number"),
        "{stderr}"
    );
    assert!(
        stderr.contains("share.qemu: [virtiofsd] needs m= in [QEMU]"),
        "{stderr}"
    );
    assert!(stdout.contains("---qemu-system@good.service---"));
    assert!(!stdout.contains("---qemu-system@bad.service---"));
}

#[test]
fn a_unit_name_is_generated_once() {
    let tmp = env::temp_dir().join(format!(
        "qemu-system-generator-conflict-{}",
        std::process::id()
    ));
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join("a-b.qemu"), "[QEMU]\nm=1\n[virtiofsd]\ntag=c\n").unwrap();
    fs::write(tmp.join("a.qemu"), "[QEMU]\nm=1\n[virtiofsd]\ntag=b-c\n").unwrap();
    let out = dryrun("--system", &tmp);
    fs::remove_dir_all(&tmp).unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr.contains("a-b.qemu: virtiofsd@a-b-c.socket is already generated from "),
        "{stderr}"
    );
    assert_eq!(stdout.matches("---virtiofsd@a-b-c.socket---").count(), 1);
}

#[test]
fn usage_errors_exit_with_two() {
    let out = Command::new(BIN).arg("--bogus").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let out = Command::new(BIN).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let out = Command::new(BIN).arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("qemu-system-generator "));
}
