// SPDX-License-Identifier: copyleft-next-0.3.1

//! The QEMU backend: every `[QEMU]` line is one QEMU option, passed
//! through verbatim. The decoder reads a few of those lines to size or
//! allow what the lifecycle needs; it never rewrites, drops or reorders
//! them. Each `[virtiofsd]` section is decoded the same way into one
//! virtiofsd process. The format is specified in
//! `man/qemu-systemd.unit.5.scd`.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::plan::{Arg, Helper, InvalidTag, Paths, Plan, SshPoweroff, Tag};
use crate::search::UnitName;
use crate::unitfile::{Entry, Section, UnitFile};

pub const SECTION: &str = "QEMU";
pub const VM_SECTION: &str = "VM";
pub const VIRTIOFSD_SECTION: &str = "virtiofsd";
/// The sections the backend reads.
pub const SECTIONS: [&str; 3] = [VM_SECTION, SECTION, VIRTIOFSD_SECTION];

const QEMU_BINARY_KEY: &str = "QemuBinary";
const VIRTIOFSD_BINARY_KEY: &str = "VirtiofsdBinary";
const SSH_PRIVATE_KEY_KEY: &str = "SshPrivateKey";
const VM_KEYS: [&str; 3] = [QEMU_BINARY_KEY, VIRTIOFSD_BINARY_KEY, SSH_PRIVATE_KEY_KEY];

/// What the machined registration calls the registering service.
pub const SERVICE: &str = "qemu-system";

/// The binaries compiled in as defaults; see `build.rs`.
pub const QEMU: &str = env!("QEMU");
pub const VIRTIOFSD: &str = env!("VIRTIOFSD");

/// Device nodes a machine opens through its command line, allowed like
/// `systemd-vmspawn@.service` does. A node that does not exist on the
/// host is skipped by systemd (`src/core/bpf-devices.c`).
const DEVICE_ALLOW: [&str; 5] = [
    "/dev/kvm rw",
    "/dev/vhost-vsock rw",
    "/dev/vhost-net rw",
    "/dev/net/tun rwm",
    "char-pts rw",
];

/// The VFIO container is a misc device; the IOMMU group nodes under
/// `/dev/vfio/` share the "vfio" character device region
/// (`drivers/vfio/group.c`), so one group entry covers every group.
const VFIO_DEVICE_ALLOW: [&str; 2] = ["/dev/vfio/vfio rw", "char-vfio rw"];

/// Locked memory QEMU needs beyond guest RAM to pin it for an assigned
/// device, in MiB.
const MEMLOCK_OVERHEAD_MIB: u64 = 256;

/// Virtqueue size of the generated vhost-user-fs device.
const VHOST_USER_FS_QUEUE_SIZE: u32 = 1024;

/// Id of the generated shared memory backend.
const MEMORY_BACKEND_ID: &str = "mem";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    MissingTag { path: PathBuf, line: usize },
    InvalidTag(InvalidTag),
    DuplicateTag(Tag),
    MissingMemory,
    MemorySize { value: String, source: SizeError },
    GuestCid(String),
    SshWithoutVsock,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTag { line, .. } => {
                write!(f, "line {line}: [{VIRTIOFSD_SECTION}] has no tag=")
            }
            Self::InvalidTag(_) => write!(f, "[{VIRTIOFSD_SECTION}]"),
            Self::DuplicateTag(tag) => write!(f, "[{VIRTIOFSD_SECTION}] tag={tag} repeated"),
            Self::MissingMemory => write!(
                f,
                "[{VIRTIOFSD_SECTION}] needs m= in [{SECTION}] to size the shared memory backend"
            ),
            Self::MemorySize { value, .. } => write!(f, "[{SECTION}] m={value}"),
            Self::GuestCid(value) => write!(
                f,
                "[{SECTION}] guest-cid={value} is not a decimal number, which the machined registration needs"
            ),
            Self::SshWithoutVsock => write!(
                f,
                "[{VM_SECTION}] {SSH_PRIVATE_KEY_KEY}= needs a device=vhost-vsock...,guest-cid= line in [{SECTION}] to reach the guest"
            ),
        }
    }
}

impl DecodeError {
    /// The file the error is in, when it is about one line of it.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::MissingTag { path, .. } => Some(path),
            _ => None,
        }
    }
}

impl Error for DecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTag(err) => Some(err),
            Self::MemorySize { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<InvalidTag> for DecodeError {
    fn from(err: InvalidTag) -> Self {
        Self::InvalidTag(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeError {
    NotANumber(String),
    Overflow,
}

impl fmt::Display for SizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotANumber(field) => write!(
                f,
                "{field:?} is not an integer with an optional B, K, M, G, T, P or E suffix"
            ),
            Self::Overflow => f.write_str("size does not fit in 64 bits of mebibytes"),
        }
    }
}

impl Error for SizeError {}

/// The first comma-separated field of a QEMU option value, which names
/// the driver or type (`-device nvme,...`, `-machine q35,...`).
fn first_field(value: &str) -> &str {
    value.split(',').next().unwrap_or_default()
}

/// A `key=` property inside a comma-separated option value.
fn property<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    value
        .split(',')
        .find_map(|f| f.strip_prefix(key)?.strip_prefix('='))
}

/// The size field of `-m`: its first field, or `size=` (`qemu-options.hx`).
fn size_field(m: &str) -> Result<&str, SizeError> {
    let first = first_field(m);
    let size = property(m, "size").unwrap_or(first);
    if size.is_empty() {
        Err(SizeError::NotANumber(String::new()))
    } else {
        Ok(size)
    }
}

/// The `-object memory-backend-memfd,size=` matching `-m`. QEMU reads a
/// bare number as mebibytes for `-m` but as bytes for `size=`, so a bare
/// number gets an M; anything else is passed on as written.
fn memfd_size(m: &str) -> Result<String, SizeError> {
    let size = size_field(m)?;
    if size.bytes().all(|b| b.is_ascii_digit()) {
        Ok(format!("{size}M"))
    } else {
        Ok(size.to_owned())
    }
}

/// The `-m` size in mebibytes, read the way `qemu_strtosz_MiB` reads it
/// (`util/cutils.c`): a bare integer is mebibytes, a suffix is one of
/// B, K, M, G, T, P or E in either case. Fractions are not read; a file
/// that needs them writes the size in a smaller unit.
fn mebibytes(m: &str) -> Result<u64, SizeError> {
    let size = size_field(m)?;
    let digits_end = size
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(size.len());
    let (digits, suffix) = size.split_at(digits_end);
    let not_a_number = || SizeError::NotANumber(size.to_owned());
    if digits.is_empty() {
        return Err(not_a_number());
    }
    let number: u64 = digits.parse().map_err(|_| SizeError::Overflow)?;
    let mib = match suffix.to_ascii_uppercase().as_str() {
        "" | "M" => Some(number),
        "B" => Some(number.div_ceil(1 << 20)),
        "K" => Some(number.div_ceil(1 << 10)),
        "G" => number.checked_mul(1 << 10),
        "T" => number.checked_mul(1 << 20),
        "P" => number.checked_mul(1 << 30),
        "E" => number.checked_mul(1 << 40),
        _ => return Err(not_a_number()),
    };
    mib.ok_or(SizeError::Overflow)
}

/// A QEMU option line as the file wrote it: `-key value`.
fn qemu_arg(entry: &Entry) -> Arg {
    let option = format!("-{}", entry.key);
    if entry.value.is_empty() {
        [option].into()
    } else {
        [option, entry.value.clone()].into()
    }
}

/// A virtiofsd option line as the file wrote it: `--key=value`, one
/// word, which its clap parser reads for every option.
fn virtiofsd_arg(entry: &Entry) -> Arg {
    if entry.value.is_empty() {
        [format!("--{}", entry.key)].into()
    } else {
        [format!("--{}={}", entry.key, entry.value)].into()
    }
}

/// The lifecycle values a `[VM]` section sets.
struct Vm<'a> {
    qemu_binary: String,
    virtiofsd_binary: String,
    ssh_private_key: Option<&'a str>,
}

fn decode_vm<'a>(file: &'a UnitFile, warnings: &mut Vec<String>) -> Vm<'a> {
    for section in file.sections_named(VM_SECTION) {
        for entry in section
            .entries
            .iter()
            .filter(|e| !VM_KEYS.contains(&e.key.as_str()))
        {
            warnings.push(format!(
                "{}: unknown [{VM_SECTION}] key {}=, ignoring",
                section.at(entry.line),
                entry.key
            ));
        }
    }
    Vm {
        qemu_binary: file
            .last(VM_SECTION, QEMU_BINARY_KEY)
            .unwrap_or(QEMU)
            .to_owned(),
        virtiofsd_binary: file
            .last(VM_SECTION, VIRTIOFSD_BINARY_KEY)
            .unwrap_or(VIRTIOFSD)
            .to_owned(),
        ssh_private_key: file.last(VM_SECTION, SSH_PRIVATE_KEY_KEY),
    }
}

/// What the decoder reads from the `[QEMU]` lines without changing them.
#[derive(Debug, Default, PartialEq, Eq)]
struct Observed<'a> {
    microvm: bool,
    own_memory_backend: bool,
    memory: Option<&'a str>,
    uuid: Option<&'a str>,
    vsock_cid: Option<u64>,
    vfio: bool,
}

fn observe<'a>(entries: &[&'a Entry]) -> Result<Observed<'a>, DecodeError> {
    let mut seen = Observed::default();
    for entry in entries {
        match entry.key.as_str() {
            "machine" => {
                seen.microvm |= entry.value.contains("microvm");
                seen.own_memory_backend |= entry.value.contains("memory-backend=");
            }
            "m" => seen.memory = Some(&entry.value),
            "uuid" => seen.uuid = Some(&entry.value),
            "device" => {
                let driver = first_field(&entry.value);
                seen.vfio |= driver.starts_with("vfio-");
                if driver.starts_with("vhost-vsock")
                    && let Some(cid) = property(&entry.value, "guest-cid")
                {
                    seen.vsock_cid = Some(
                        cid.parse()
                            .map_err(|_| DecodeError::GuestCid(cid.to_owned()))?,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(seen)
}

fn decode_helper(section: &Section, binary: &str) -> Result<Helper, DecodeError> {
    let tag: Tag = section
        .entries
        .iter()
        .rfind(|e| e.key == "tag")
        .ok_or_else(|| DecodeError::MissingTag {
            path: section.path.clone(),
            line: section.line,
        })?
        .value
        .parse()?;
    let requires_mounts = section
        .entries
        .iter()
        .filter(|e| e.key == "shared-dir" && e.value.starts_with('/') && !e.value.contains('%'))
        .map(|e| e.value.clone())
        .collect();
    let args = section
        .entries
        .iter()
        .map(virtiofsd_arg)
        .chain(std::iter::once(["--fd=3".to_owned()].into()))
        .collect();
    Ok(Helper {
        program: VIRTIOFSD_SECTION,
        tag,
        binary: binary.to_owned(),
        args,
        requires_mounts,
    })
}

/// The lines that wire the helpers into the machine: one shared memory
/// backend, unless the file names its own, then a chardev and a
/// vhost-user-fs device per helper.
fn share_args(
    seen: &Observed<'_>,
    helpers: &[Helper],
    machine: &UnitName,
    paths: &impl Paths,
) -> Result<Vec<Arg>, DecodeError> {
    let mut args = Vec::new();
    if helpers.is_empty() {
        return Ok(args);
    }
    if !seen.own_memory_backend {
        let memory = seen.memory.ok_or(DecodeError::MissingMemory)?;
        let size = memfd_size(memory).map_err(|source| DecodeError::MemorySize {
            value: memory.to_owned(),
            source,
        })?;
        args.push(Arg::from([
            "-object".to_owned(),
            format!("memory-backend-memfd,id={MEMORY_BACKEND_ID},size={size},share=on"),
        ]));
        args.push(Arg::from([
            "-machine".to_owned(),
            format!("memory-backend={MEMORY_BACKEND_ID}"),
        ]));
    }
    let fs_device = if seen.microvm {
        "vhost-user-fs-device"
    } else {
        "vhost-user-fs-pci"
    };
    for helper in helpers {
        let tag = &helper.tag;
        let chardev = format!("virtiofs_{tag}");
        let socket = paths.helper_socket(machine, helper.program, tag);
        args.push(Arg::from([
            "-chardev".to_owned(),
            format!("socket,id={chardev},path={socket}"),
        ]));
        args.push(Arg::from([
            "-device".to_owned(),
            format!(
                "{fs_device},queue-size={VHOST_USER_FS_QUEUE_SIZE},chardev={chardev},tag={tag}"
            ),
        ]));
    }
    Ok(args)
}

/// The locked memory an assigned device needs: guest RAM plus QEMU's
/// own allocations.
fn memlock_mib(seen: &Observed<'_>) -> Result<Option<u64>, DecodeError> {
    if !seen.vfio {
        return Ok(None);
    }
    let memory = seen.memory.ok_or(DecodeError::MissingMemory)?;
    let size_error = |source| DecodeError::MemorySize {
        value: memory.to_owned(),
        source,
    };
    let ram = mebibytes(memory).map_err(size_error)?;
    ram.checked_add(MEMLOCK_OVERHEAD_MIB)
        .map(Some)
        .ok_or_else(|| size_error(SizeError::Overflow))
}

pub struct Decoded {
    pub plan: Plan,
    pub warnings: Vec<String>,
}

pub fn decode(
    machine: &UnitName,
    file: &UnitFile,
    paths: &impl Paths,
) -> Result<Decoded, DecodeError> {
    let mut warnings = Vec::new();
    let vm = decode_vm(file, &mut warnings);

    let qemu: Vec<&Entry> = file.entries(SECTION).collect();
    let seen = observe(&qemu)?;

    let mut helpers: Vec<Helper> = Vec::new();
    for section in file.sections_named(VIRTIOFSD_SECTION) {
        let helper = decode_helper(section, &vm.virtiofsd_binary)?;
        if helpers.iter().any(|h| h.tag == helper.tag) {
            return Err(DecodeError::DuplicateTag(helper.tag));
        }
        helpers.push(helper);
    }

    // The command line: the file's lines, then what the lifecycle adds.
    let control_socket = format!("{}/qmp.sock", paths.runtime_dir(machine));
    let args: Vec<Arg> = qemu
        .iter()
        .copied()
        .map(qemu_arg)
        .chain(share_args(&seen, &helpers, machine, paths)?)
        .chain(std::iter::once(Arg::from([
            "-qmp".to_owned(),
            format!("unix:{control_socket},server=on,wait=off"),
        ])))
        .collect();

    let mut device_allow = DEVICE_ALLOW.to_vec();
    if seen.vfio {
        device_allow.extend(VFIO_DEVICE_ALLOW);
    }

    let ssh_poweroff = vm
        .ssh_private_key
        .map(|key| {
            seen.vsock_cid
                .map(|vsock_cid| SshPoweroff {
                    private_key: key.to_owned(),
                    vsock_cid,
                })
                .ok_or(DecodeError::SshWithoutVsock)
        })
        .transpose()?;

    Ok(Decoded {
        plan: Plan {
            binary: vm.qemu_binary,
            args,
            helpers,
            device_allow,
            memlock_mib: memlock_mib(&seen)?,
            vsock_cid: seen.vsock_cid,
            uuid: seen.uuid.map(str::to_owned),
            ssh_poweroff,
            control_socket,
        },
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPaths;

    impl Paths for TestPaths {
        fn runtime_dir(&self, machine: &UnitName) -> String {
            format!("/run/{machine}")
        }

        fn helper_socket(&self, machine: &UnitName, program: &str, tag: &Tag) -> String {
            format!("/run/{program}/{machine}-{tag}.sock")
        }
    }

    fn decode_text(text: &str) -> Result<Decoded, DecodeError> {
        let name: UnitName = "vm".parse().unwrap();
        decode(&name, &text.parse().unwrap(), &TestPaths)
    }

    fn command_line(plan: &Plan) -> Vec<String> {
        plan.args.iter().map(|a| a.0.join(" ")).collect()
    }

    #[test]
    fn memfd_sizes() {
        assert_eq!(memfd_size("2048").unwrap(), "2048M");
        assert_eq!(memfd_size("2G").unwrap(), "2G");
        assert_eq!(memfd_size("size=4G,slots=2,maxmem=16G").unwrap(), "4G");
        assert_eq!(memfd_size("512M,maxmem=1G").unwrap(), "512M");
        let empty = Err(SizeError::NotANumber(String::new()));
        assert_eq!(memfd_size(""), empty);
        assert_eq!(memfd_size(",slots=2"), empty);
    }

    #[test]
    fn mebibyte_sizes() {
        assert_eq!(mebibytes("2048").unwrap(), 2048);
        assert_eq!(mebibytes("2048M").unwrap(), 2048);
        assert_eq!(mebibytes("8g").unwrap(), 8192);
        assert_eq!(mebibytes("1T").unwrap(), 1 << 20);
        assert_eq!(mebibytes("1048576k").unwrap(), 1024);
        assert_eq!(mebibytes("1048577b").unwrap(), 2);
        assert_eq!(mebibytes("size=1G,slots=2,maxmem=16G").unwrap(), 1024);
        assert_eq!(
            mebibytes("1.5G"),
            Err(SizeError::NotANumber("1.5G".to_owned()))
        );
        assert_eq!(mebibytes("G"), Err(SizeError::NotANumber("G".to_owned())));
        assert_eq!(mebibytes("99999999999E"), Err(SizeError::Overflow));
    }

    #[test]
    fn properties() {
        assert_eq!(first_field("nvme,serial=x"), "nvme");
        assert_eq!(first_field(""), "");
        assert_eq!(
            property("vhost-vsock-pci,guest-cid=7,id=v", "guest-cid"),
            Some("7")
        );
        assert_eq!(property("vhost-vsock-pci", "guest-cid"), None);
        assert_eq!(property("a,guest-cidx=1", "guest-cid"), None);
    }

    #[test]
    fn qemu_lines_pass_through_in_order() {
        let decoded = decode_text(
            "[QEMU]\nmachine=q35\nm=1024\nnographic=\ndevice=a\nmachine=usb=on\ndevice=b\n",
        )
        .unwrap();
        assert_eq!(decoded.warnings, [] as [String; 0]);
        let plan = decoded.plan;
        assert_eq!(plan.binary, QEMU);
        assert_eq!(
            command_line(&plan),
            [
                "-machine q35",
                "-m 1024",
                "-nographic",
                "-device a",
                "-machine usb=on",
                "-device b",
                "-qmp unix:/run/vm/qmp.sock,server=on,wait=off",
            ]
        );
        assert_eq!(plan.device_allow, DEVICE_ALLOW);
        assert_eq!(plan.memlock_mib, None);
        assert_eq!((plan.vsock_cid, plan.uuid), (None, None));
        assert_eq!(plan.helpers, [] as [Helper; 0]);
        assert!(plan.ssh_poweroff.is_none());
    }

    #[test]
    fn virtiofsd_sections_become_helpers() {
        let decoded = decode_text(
            "[QEMU]\nm=2G\n[virtiofsd]\nshared-dir=/srv/a\ntag=a\nxattr=\n[virtiofsd]\nshared-dir=%h\ntag=b\n",
        )
        .unwrap();
        let plan = decoded.plan;
        let tags: Vec<_> = plan.helpers.iter().map(|h| h.tag.to_string()).collect();
        assert_eq!(tags, ["a", "b"]);
        assert_eq!(plan.helpers[0].requires_mounts, ["/srv/a"]);
        assert_eq!(plan.helpers[1].requires_mounts, [] as [String; 0]);
        assert_eq!(plan.helpers[0].binary, VIRTIOFSD);
        let words: Vec<_> = plan.helpers[0].args.iter().map(|a| a.0.join(" ")).collect();
        assert_eq!(
            words,
            ["--shared-dir=/srv/a", "--tag=a", "--xattr", "--fd=3"]
        );
        assert_eq!(
            command_line(&plan)[1..],
            [
                "-object memory-backend-memfd,id=mem,size=2G,share=on",
                "-machine memory-backend=mem",
                "-chardev socket,id=virtiofs_a,path=/run/virtiofsd/vm-a.sock",
                "-device vhost-user-fs-pci,queue-size=1024,chardev=virtiofs_a,tag=a",
                "-chardev socket,id=virtiofs_b,path=/run/virtiofsd/vm-b.sock",
                "-device vhost-user-fs-pci,queue-size=1024,chardev=virtiofs_b,tag=b",
                "-qmp unix:/run/vm/qmp.sock,server=on,wait=off",
            ]
        );
    }

    #[test]
    fn microvm_and_own_memory_backend() {
        let decoded = decode_text(
            "[QEMU]\nmachine=microvm,memory-backend=ram\nobject=memory-backend-memfd,id=ram,size=1G,share=on\n[virtiofsd]\ntag=a\n",
        )
        .unwrap();
        let line = command_line(&decoded.plan);
        assert!(
            line.iter()
                .all(|a| !a.starts_with("-machine memory-backend"))
        );
        assert!(
            line.iter()
                .any(|a| a.starts_with("-device vhost-user-fs-device,"))
        );
    }

    #[test]
    fn observed_lines() {
        let decoded = decode_text(
            "[QEMU]\nm=4G\nuuid=u\ndevice=vhost-vsock-pci-non-transitional,guest-cid=42\ndevice=vfio-pci,host=0000:01:00.0\n",
        )
        .unwrap();
        let plan = decoded.plan;
        assert_eq!(plan.vsock_cid, Some(42));
        assert_eq!(plan.uuid.as_deref(), Some("u"));
        assert_eq!(plan.memlock_mib, Some(4096 + 256));
        assert_eq!(
            plan.device_allow.len(),
            DEVICE_ALLOW.len() + VFIO_DEVICE_ALLOW.len()
        );
        assert!(plan.device_allow.contains(&"char-vfio rw"));
    }

    #[test]
    fn vm_section() {
        let decoded = decode_text(
            "[VM]\nQemuBinary=/opt/qemu\nVirtiofsdBinary=/opt/virtiofsd\nSshPrivateKey=%h/.ssh/key\nColour=blue\n[QEMU]\nm=1\ndevice=vhost-vsock-pci,guest-cid=3\n[virtiofsd]\ntag=t\n",
        )
        .unwrap();
        assert_eq!(decoded.warnings.len(), 1);
        assert!(decoded.warnings[0].contains("Colour="));
        let plan = decoded.plan;
        assert_eq!(plan.binary, "/opt/qemu");
        assert_eq!(plan.helpers[0].binary, "/opt/virtiofsd");
        assert_eq!(
            plan.ssh_poweroff,
            Some(SshPoweroff {
                private_key: "%h/.ssh/key".to_owned(),
                vsock_cid: 3
            })
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            decode_text("[QEMU]\nm=1\n[virtiofsd]\nshared-dir=/a\n").err(),
            Some(DecodeError::MissingTag {
                path: PathBuf::new(),
                line: 3
            })
        );
        assert!(matches!(
            decode_text("[QEMU]\nm=1\n[virtiofsd]\ntag=a b\n").err(),
            Some(DecodeError::InvalidTag(_))
        ));
        assert_eq!(
            decode_text("[QEMU]\nm=1\n[virtiofsd]\ntag=a\n[virtiofsd]\ntag=a\n").err(),
            Some(DecodeError::DuplicateTag("a".parse().unwrap()))
        );
        assert_eq!(
            decode_text("[virtiofsd]\ntag=a\n").err(),
            Some(DecodeError::MissingMemory)
        );
        assert_eq!(
            decode_text("[QEMU]\ndevice=vfio-pci,host=1\n").err(),
            Some(DecodeError::MissingMemory)
        );
        assert!(matches!(
            decode_text("[QEMU]\nm=1.5G\ndevice=vfio-pci,host=1\n").err(),
            Some(DecodeError::MemorySize { .. })
        ));
        assert_eq!(
            decode_text("[QEMU]\ndevice=vhost-vsock-pci,guest-cid=0x2a\n").err(),
            Some(DecodeError::GuestCid("0x2a".to_owned()))
        );
        assert_eq!(
            decode_text("[VM]\nSshPrivateKey=/k\n[QEMU]\nm=1\n").err(),
            Some(DecodeError::SshWithoutVsock)
        );
    }
}
