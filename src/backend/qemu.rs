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

use super::json;
use crate::plan::{Arg, Helper, InvalidTag, Paths, Plan, SshPoweroff, Tag};
use crate::search::UnitName;
use crate::unitfile::{Entry, Section, UnitFile};

const SECTION: &str = "QEMU";
const VM_SECTION: &str = "VM";
const VIRTIOFSD_SECTION: &str = "virtiofsd";
/// The sections the backend reads.
pub const SECTIONS: [&str; 3] = [VM_SECTION, SECTION, VIRTIOFSD_SECTION];

const QEMU_BINARY_KEY: &str = "QEMUBinary";
const VIRTIOFSD_BINARY_KEY: &str = "VirtiofsdBinary";
const SSH_PRIVATE_KEY_KEY: &str = "SSHPrivateKey";
const VM_KEYS: [&str; 3] = [QEMU_BINARY_KEY, VIRTIOFSD_BINARY_KEY, SSH_PRIVATE_KEY_KEY];

/// The machine's process, as its units and its registration name it.
const SERVICE: &str = "qemu-system";

/// The binaries compiled in as defaults; see `build.rs`.
const QEMU: &str = env!("QEMU");
const VIRTIOFSD: &str = env!("VIRTIOFSD");

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
/// With an iommufd backend QEMU opens `/dev/iommu`, a misc device, and
/// `/dev/vfio/devices/vfio*`, which share "vfio-dev"
/// (`drivers/vfio/device_cdev.c`); a group entry matches one region name
/// exactly.
const VFIO_DEVICE_ALLOW: [&str; 4] = [
    "/dev/vfio/vfio rw",
    "char-vfio rw",
    "/dev/iommu rw",
    "char-vfio-dev rw",
];

/// The `-device` drivers that assign a host device through the kernel's
/// VFIO (`hw/vfio/`), with the vfio-platform family that QEMU 10.2
/// removed (`docs/about/removed-features.rst`). `vfio-user-pci` talks to
/// a server over a socket instead and needs none of it.
const VFIO_DRIVERS: [&str; 7] = [
    "vfio-pci",
    "vfio-pci-nohotplug",
    "vfio-ccw",
    "vfio-ap",
    "vfio-platform",
    "vfio-amd-xgbe",
    "vfio-calxeda-xgmac",
];

/// Virtqueue size of the generated vhost-user-fs device.
const VHOST_USER_FS_QUEUE_SIZE: u32 = 1024;

/// Ids of the objects the generator adds start with `systemd-`, which
/// leaves the short ids a file picks for itself free.
const MEMORY_BACKEND_ID: &str = "systemd-mem";
const CHARDEV_ID_PREFIX: &str = "systemd-virtiofs-";

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    MissingTag {
        path: PathBuf,
        line: usize,
    },
    InvalidTag {
        path: PathBuf,
        line: usize,
        source: InvalidTag,
    },
    DuplicateTag {
        path: PathBuf,
        line: usize,
        tag: Tag,
    },
    MissingMemory,
    MemorySize(String),
    GuestCid(String),
    SshWithoutVsock,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTag { line, .. } => {
                write!(f, "line {line}: [{VIRTIOFSD_SECTION}] has no tag=")
            }
            Self::InvalidTag { line, .. } => write!(f, "line {line}: [{VIRTIOFSD_SECTION}]"),
            Self::DuplicateTag { line, tag, .. } => {
                write!(f, "line {line}: [{VIRTIOFSD_SECTION}] tag={tag} repeated")
            }
            Self::MissingMemory => write!(
                f,
                "[{VIRTIOFSD_SECTION}] needs m= in [{SECTION}] to size the shared memory backend"
            ),
            Self::MemorySize(value) => write!(f, "[{SECTION}] m={value} has no size"),
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
            Self::MissingTag { path, .. }
            | Self::InvalidTag { path, .. }
            | Self::DuplicateTag { path, .. } => Some(path),
            _ => None,
        }
    }
}

impl Error for DecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTag { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The first comma-separated field of a QEMU option value, which names
/// the driver or type (`-device nvme,...`, `-machine q35,...`).
fn first_field(value: &str) -> &str {
    value.split_once(',').map_or(value, |(first, _)| first)
}

/// A `key=` property inside a comma-separated option value.
fn property<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    value
        .split(',')
        .find_map(|f| f.strip_prefix(key)?.strip_prefix('='))
}

/// The `key=` of an option value whose first field may leave out
/// `key=`, as QEMU reads "type" for `-machine` and "driver" for
/// `-device`: the first field unless that is a `key=value`, else the
/// `key=` field.
fn implied<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    let first = first_field(value);
    if first.contains('=') {
        property(value, key)
    } else {
        Some(first)
    }
}

/// What the decoder reads from a `-device` value.
struct Device<'a> {
    driver: String,
    guest_cid: Option<&'a str>,
}

/// A `-device` value in either form QEMU takes: a JSON object when it
/// starts with `{`, comma-separated fields otherwise
/// (`QEMU_OPTION_device` in `system/vl.c`).
fn device(value: &str) -> Option<Device<'_>> {
    if value.starts_with('{') {
        let members = json::members(value)?;
        let member = |key: &str| members.iter().find(|(k, _)| k == key).map(|&(_, v)| v);
        Some(Device {
            driver: json::string(member("driver")?)?,
            guest_cid: member("guest-cid"),
        })
    } else {
        Some(Device {
            driver: implied(value, "driver")?.to_owned(),
            guest_cid: property(value, "guest-cid"),
        })
    }
}

/// The `-object memory-backend-memfd,size=` matching `-m`, whose size is
/// its first field or its `size=` field (`qemu-options.hx`). QEMU reads
/// a bare number as mebibytes for `-m` but as bytes for `size=`, so a
/// bare number gets an M; anything else is passed on as written.
fn memfd_size(m: &str) -> Option<String> {
    let size = property(m, "size").unwrap_or_else(|| first_field(m));
    if size.is_empty() {
        None
    } else if size.bytes().all(|b| b.is_ascii_digit()) {
        Some(format!("{size}M"))
    } else {
        Some(size.to_owned())
    }
}

/// A QEMU option line as the file wrote it: `-key value`.
fn qemu_arg(entry: &Entry) -> Arg {
    let option = format!("-{}", entry.key);
    if entry.value.is_empty() {
        vec![option]
    } else {
        vec![option, entry.value.clone()]
    }
}

/// A virtiofsd option line as the file wrote it: `--key=value`, one
/// word, which its clap parser reads for every option.
fn virtiofsd_arg(entry: &Entry) -> Arg {
    if entry.value.is_empty() {
        vec![format!("--{}", entry.key)]
    } else {
        vec![format!("--{}={}", entry.key, entry.value)]
    }
}

/// The lifecycle values a `[VM]` section sets.
struct Vm<'a> {
    qemu_binary: String,
    virtiofsd_binary: String,
    ssh_private_key: Option<&'a str>,
}

fn decode_vm<'a>(file: &'a UnitFile, warn: &mut impl FnMut(String)) -> Vm<'a> {
    for section in file.sections_named(VM_SECTION) {
        for entry in section
            .entries
            .iter()
            .filter(|e| !VM_KEYS.contains(&e.key.as_str()))
        {
            warn(format!(
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
#[derive(Default)]
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
            "machine" | "M" => {
                // QEMU keeps the last value of a property repeated
                // across -machine options.
                if let Some(name) = implied(&entry.value, "type") {
                    seen.microvm = name == "microvm";
                }
                seen.own_memory_backend |= property(&entry.value, "memory-backend").is_some();
            }
            "numa" => seen.own_memory_backend |= property(&entry.value, "memdev").is_some(),
            "m" => seen.memory = Some(&entry.value),
            "uuid" => seen.uuid = Some(&entry.value),
            "device" => {
                let Some(device) = device(&entry.value) else {
                    continue;
                };
                seen.vfio |= VFIO_DRIVERS.contains(&device.driver.as_str());
                if device.driver.starts_with("vhost-vsock")
                    && let Some(cid) = device.guest_cid
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

/// One `[virtiofsd]` section, whose tag must be new among `helpers`.
fn decode_helper(
    section: &Section,
    binary: &str,
    helpers: &[Helper],
) -> Result<Helper, DecodeError> {
    let entry = section
        .entries
        .iter()
        .rfind(|e| e.key == "tag")
        .ok_or_else(|| DecodeError::MissingTag {
            path: section.path.clone(),
            line: section.line,
        })?;
    let tag: Tag = entry
        .value
        .parse()
        .map_err(|source| DecodeError::InvalidTag {
            path: section.path.clone(),
            line: entry.line,
            source,
        })?;
    if helpers.iter().any(|h| h.tag == tag) {
        return Err(DecodeError::DuplicateTag {
            path: section.path.clone(),
            line: entry.line,
            tag,
        });
    }
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
        .chain(std::iter::once(vec!["--fd=3".to_owned()]))
        .collect();
    Ok(Helper {
        program: VIRTIOFSD_SECTION,
        documentation: "https://gitlab.com/virtio-fs/virtiofsd",
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
        let size = memfd_size(memory).ok_or_else(|| DecodeError::MemorySize(memory.to_owned()))?;
        args.push(vec![
            "-object".to_owned(),
            format!("memory-backend-memfd,id={MEMORY_BACKEND_ID},size={size},share=on"),
        ]);
        args.push(vec![
            "-machine".to_owned(),
            format!("memory-backend={MEMORY_BACKEND_ID}"),
        ]);
    }
    let fs_device = if seen.microvm {
        "vhost-user-fs-device"
    } else {
        "vhost-user-fs-pci"
    };
    for helper in helpers {
        let tag = &helper.tag;
        let chardev = format!("{CHARDEV_ID_PREFIX}{tag}");
        let socket = paths.helper_socket(machine, helper.program, tag);
        args.push(vec![
            "-chardev".to_owned(),
            format!("socket,id={chardev},path={socket}"),
        ]);
        args.push(vec![
            "-device".to_owned(),
            format!(
                "{fs_device},queue-size={VHOST_USER_FS_QUEUE_SIZE},chardev={chardev},tag={tag}"
            ),
        ]);
    }
    Ok(args)
}

/// The plan of `file`; a line the backend ignores is reported through
/// `warn`.
pub fn decode(
    machine: &UnitName,
    file: &UnitFile,
    paths: &impl Paths,
    mut warn: impl FnMut(String),
) -> Result<Plan, DecodeError> {
    let vm = decode_vm(file, &mut warn);

    let qemu: Vec<&Entry> = file.entries(SECTION).collect();
    let seen = observe(&qemu)?;

    let mut helpers: Vec<Helper> = Vec::new();
    for section in file.sections_named(VIRTIOFSD_SECTION) {
        helpers.push(decode_helper(section, &vm.virtiofsd_binary, &helpers)?);
    }

    // The command line: the file's lines, then what the lifecycle adds.
    let control_socket = format!("{}/qmp.sock", paths.runtime_dir(machine, SERVICE));
    let args: Vec<Arg> = qemu
        .iter()
        .copied()
        .map(qemu_arg)
        .chain(share_args(&seen, &helpers, machine, paths)?)
        .chain(std::iter::once(vec![
            "-qmp".to_owned(),
            format!("unix:{control_socket},server=on,wait=off"),
        ]))
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

    Ok(Plan {
        program: SERVICE,
        binary: vm.qemu_binary,
        args,
        helpers,
        device_allow,
        lock_memory: seen.vfio,
        vsock_cid: seen.vsock_cid,
        uuid: seen.uuid.map(str::to_owned),
        ssh_poweroff,
        control_socket,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPaths;

    impl Paths for TestPaths {
        fn runtime_dir(&self, machine: &UnitName, program: &str) -> String {
            format!("/run/{program}/{machine}")
        }

        fn helper_socket(&self, machine: &UnitName, program: &str, tag: &Tag) -> String {
            format!("/run/{program}/{machine}-{tag}.sock")
        }
    }

    fn decode_text(text: &str) -> Result<Plan, DecodeError> {
        let name: UnitName = "vm".parse().unwrap();
        decode(&name, &text.parse().unwrap(), &TestPaths, |w| panic!("{w}"))
    }

    fn command_line(plan: &Plan) -> Vec<String> {
        plan.args.iter().map(|a| a.join(" ")).collect()
    }

    #[test]
    fn memfd_sizes() {
        assert_eq!(memfd_size("2048").unwrap(), "2048M");
        assert_eq!(memfd_size("2G").unwrap(), "2G");
        assert_eq!(memfd_size("size=4G,slots=2,maxmem=16G").unwrap(), "4G");
        assert_eq!(memfd_size("512M,maxmem=1G").unwrap(), "512M");
        assert_eq!(memfd_size(""), None);
        assert_eq!(memfd_size(",slots=2"), None);
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
        assert_eq!(implied("q35,accel=kvm", "type"), Some("q35"));
        assert_eq!(implied("accel=kvm,type=microvm", "type"), Some("microvm"));
        assert_eq!(implied("accel=kvm", "type"), None);
        assert_eq!(
            implied("host=x,driver=vfio-pci", "driver"),
            Some("vfio-pci")
        );
    }

    #[test]
    fn qemu_lines_pass_through_in_order() {
        let plan = decode_text(
            "[QEMU]\nmachine=q35\nm=1024\nnographic=\ndevice=a\nmachine=usb=on\ndevice=b\n",
        )
        .unwrap();
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
                "-qmp unix:/run/qemu-system/vm/qmp.sock,server=on,wait=off",
            ]
        );
        assert_eq!(plan.device_allow, DEVICE_ALLOW);
        assert!(!plan.lock_memory);
        assert_eq!((plan.vsock_cid, plan.uuid), (None, None));
        assert_eq!(plan.helpers, [] as [Helper; 0]);
        assert!(plan.ssh_poweroff.is_none());
    }

    #[test]
    fn virtiofsd_sections_become_helpers() {
        let plan = decode_text(
            "[QEMU]\nm=2G\n[virtiofsd]\nshared-dir=/path/to/a\ntag=a\nxattr=\n[virtiofsd]\nshared-dir=%h\ntag=b\n",
        )
        .unwrap();
        let tags: Vec<_> = plan.helpers.iter().map(|h| h.tag.to_string()).collect();
        assert_eq!(tags, ["a", "b"]);
        assert_eq!(plan.helpers[0].requires_mounts, ["/path/to/a"]);
        assert_eq!(plan.helpers[1].requires_mounts, [] as [String; 0]);
        assert_eq!(plan.helpers[0].binary, VIRTIOFSD);
        let words: Vec<_> = plan.helpers[0].args.iter().map(|a| a.join(" ")).collect();
        assert_eq!(
            words,
            ["--shared-dir=/path/to/a", "--tag=a", "--xattr", "--fd=3"]
        );
        assert_eq!(
            command_line(&plan)[1..],
            [
                "-object memory-backend-memfd,id=systemd-mem,size=2G,share=on",
                "-machine memory-backend=systemd-mem",
                "-chardev socket,id=systemd-virtiofs-a,path=/run/virtiofsd/vm-a.sock",
                "-device vhost-user-fs-pci,queue-size=1024,chardev=systemd-virtiofs-a,tag=a",
                "-chardev socket,id=systemd-virtiofs-b,path=/run/virtiofsd/vm-b.sock",
                "-device vhost-user-fs-pci,queue-size=1024,chardev=systemd-virtiofs-b,tag=b",
                "-qmp unix:/run/qemu-system/vm/qmp.sock,server=on,wait=off",
            ]
        );
    }

    #[test]
    fn microvm_and_own_memory_backend() {
        let plan = decode_text(
            "[QEMU]\nmachine=microvm,memory-backend=ram\nobject=memory-backend-memfd,id=ram,size=1G,share=on\n[virtiofsd]\ntag=a\n",
        )
        .unwrap();
        let line = command_line(&plan);
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
    fn numa_memdev_is_own_memory_backend() {
        let plan = decode_text(
            "[QEMU]\nobject=memory-backend-memfd,id=m0,size=1G,share=on\nnuma=node,memdev=m0\n[virtiofsd]\ntag=a\n",
        )
        .unwrap();
        let line = command_line(&plan);
        assert!(line.iter().all(|a| !a.starts_with("-machine")));
        assert_eq!(line.iter().filter(|a| a.starts_with("-object")).count(), 1);
    }

    #[test]
    fn m_is_machine() {
        let line =
            command_line(&decode_text("[QEMU]\nM=microvm\nm=1G\n[virtiofsd]\ntag=a\n").unwrap());
        assert!(
            line.iter()
                .any(|a| a.starts_with("-device vhost-user-fs-device,"))
        );
        let line = command_line(
            &decode_text("[QEMU]\nM=q35,memory-backend=ram\n[virtiofsd]\ntag=a\n").unwrap(),
        );
        assert!(line.iter().all(|a| !a.starts_with("-object")));
    }

    #[test]
    fn machine_type_is_a_field() {
        for (qemu, device) in [
            ("machine=type=microvm\n", "vhost-user-fs-device"),
            (
                "machine=microvm\nmachine=accel=kvm\n",
                "vhost-user-fs-device",
            ),
            ("machine=microvm\nmachine=q35\n", "vhost-user-fs-pci"),
            (
                "machine=q35,memory-backend=microvm-ram\n",
                "vhost-user-fs-pci",
            ),
        ] {
            let line = command_line(
                &decode_text(&format!("[QEMU]\n{qemu}m=1G\n[virtiofsd]\ntag=a\n")).unwrap(),
            );
            assert!(
                line.iter()
                    .any(|a| a.starts_with(&format!("-device {device},"))),
                "{qemu}"
            );
        }
    }

    #[test]
    fn observed_lines() {
        let plan = decode_text(
            "[QEMU]\nm=4G\nuuid=u\ndevice=vhost-vsock-pci-non-transitional,guest-cid=42\ndevice=vfio-pci,host=0000:01:00.0\n",
        )
        .unwrap();
        assert_eq!(plan.vsock_cid, Some(42));
        assert_eq!(plan.uuid.as_deref(), Some("u"));
        assert!(plan.lock_memory);
        assert_eq!(
            plan.device_allow.len(),
            DEVICE_ALLOW.len() + VFIO_DEVICE_ALLOW.len()
        );
        assert!(plan.device_allow.contains(&"char-vfio rw"));
        assert!(plan.device_allow.contains(&"/dev/iommu rw"));
        assert!(plan.device_allow.contains(&"char-vfio-dev rw"));
    }

    #[test]
    fn device_driver_may_be_named() {
        let plan = decode_text(
            "[QEMU]\ndevice=driver=vfio-pci,host=0000:01:00.0\ndevice=guest-cid=5,driver=vhost-vsock-pci\n",
        )
        .unwrap();
        assert!(plan.lock_memory);
        assert_eq!(plan.vsock_cid, Some(5));
    }

    #[test]
    fn json_devices() {
        let plan = decode_text(concat!(
            "[QEMU]\n",
            r#"device={"driver": "vfio-pci", "host": "0000:01:00.0"}"#,
            "\n",
            r"device={'driver': 'vhost-vsock-pci', 'guest-cid': 7}",
            "\n",
        ))
        .unwrap();
        assert!(plan.lock_memory);
        assert_eq!(plan.vsock_cid, Some(7));
        let plan = decode_text(concat!(
            "[QEMU]\n",
            r#"device={"driver":"vfio-user-pci","socket":{"type":"unix","path":"/path/to/sock"}}"#,
            "\n",
            r#"device={"driver":"vfio-pci","#,
            "\n",
        ))
        .unwrap();
        assert!(!plan.lock_memory);
        assert_eq!(
            decode_text("[QEMU]\ndevice={\"driver\":\"vhost-vsock-pci\",\"guest-cid\":\"7\"}\n")
                .err(),
            Some(DecodeError::GuestCid("\"7\"".to_owned()))
        );
    }

    #[test]
    fn vfio_drivers() {
        for (driver, vfio) in [
            ("vfio-pci", true),
            ("vfio-pci-nohotplug", true),
            ("vfio-ccw", true),
            ("vfio-ap", true),
            ("vfio-platform", true),
            ("vfio-user-pci", false),
            ("vfio-pci-igd-lpc-bridge", false),
        ] {
            let plan = decode_text(&format!("[QEMU]\ndevice={driver},x=y\n")).unwrap();
            assert_eq!(plan.lock_memory, vfio, "{driver}");
        }
    }

    #[test]
    fn vm_section() {
        let file: UnitFile = "[VM]\nQEMUBinary=/opt/qemu\nVirtiofsdBinary=/opt/virtiofsd\nSSHPrivateKey=%h/.ssh/key\nColour=blue\n[QEMU]\nm=1\ndevice=vhost-vsock-pci,guest-cid=3\n[virtiofsd]\ntag=t\n"
            .parse()
            .unwrap();
        let name: UnitName = "vm".parse().unwrap();
        let mut warnings = Vec::new();
        let plan = decode(&name, &file, &TestPaths, |w| warnings.push(w)).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Colour="));
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
            Some(DecodeError::InvalidTag { line: 4, .. })
        ));
        assert_eq!(
            decode_text("[QEMU]\nm=1\n[virtiofsd]\ntag=a\n[virtiofsd]\ntag=a\n").err(),
            Some(DecodeError::DuplicateTag {
                path: PathBuf::new(),
                line: 6,
                tag: "a".parse().unwrap()
            })
        );
        assert_eq!(
            decode_text("[virtiofsd]\ntag=a\n").err(),
            Some(DecodeError::MissingMemory)
        );
        assert_eq!(
            decode_text("[QEMU]\nm=,slots=2\n[virtiofsd]\ntag=a\n").err(),
            Some(DecodeError::MemorySize(",slots=2".to_owned()))
        );
        assert_eq!(
            decode_text("[QEMU]\ndevice=vhost-vsock-pci,guest-cid=0x2a\n").err(),
            Some(DecodeError::GuestCid("0x2a".to_owned()))
        );
        assert_eq!(
            decode_text("[VM]\nSSHPrivateKey=/k\n[QEMU]\nm=1\n").err(),
            Some(DecodeError::SshWithoutVsock)
        );
    }
}
