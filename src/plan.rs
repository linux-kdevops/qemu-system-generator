// SPDX-License-Identifier: copyleft-next-0.3.1

//! What a backend decodes a file into, in terms no launcher owns: the
//! processes to run and what they need from the host. A launcher (today
//! systemd, see `launcher/`) turns a [`Plan`] into its own unit files.
//!
//! Values that end up in unit text stay `String`: a unit file is UTF-8
//! and may carry launcher specifiers such as `%t`, which are not paths
//! until the launcher expands them.

use std::error::Error;
use std::fmt;
use std::str::FromStr;

use crate::search::UnitName;

/// Words of a command line that belong together, such as an option and
/// its argument, so that a launcher can lay them out one group per line.
/// Each word reaches the program as it is written here.
pub type Arg = Vec<String>;

/// The mount tag of a share, which also names the helper that serves it.
/// It must fit in a unit name, in a QEMU id after `systemd-virtiofs-`
/// (`id_wellformed()` in QEMU's `util/id.c`), and in a `-device` value,
/// where a comma would start a new property.
#[derive(Debug, PartialEq, Eq)]
pub struct Tag(String);

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidTag(pub String);

impl fmt::Display for InvalidTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tag {:?} is not usable in a unit name: letters, digits, '_', '.' and '-' only",
            self.0
        )
    }
}

impl Error for InvalidTag {}

impl FromStr for Tag {
    type Err = InvalidTag;

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        let valid = !tag.is_empty()
            && tag
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'));
        if valid {
            Ok(Self(tag.to_owned()))
        } else {
            Err(InvalidTag(tag.to_owned()))
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A daemon the machine talks to over a socket the launcher listens on
/// and passes in at start (systemd: socket activation, fd 3).
#[derive(Debug, PartialEq, Eq)]
pub struct Helper {
    /// The daemon's name, which names its units.
    pub program: &'static str,
    /// Where the daemon's options are documented, as a URI.
    pub documentation: &'static str,
    pub tag: Tag,
    pub binary: String,
    pub args: Vec<Arg>,
    /// Host paths the daemon serves; the launcher orders it after their
    /// mounts.
    pub requires_mounts: Vec<String>,
}

/// A guest shutdown asked for over SSH, before the backend's own
/// channel is used.
#[derive(Debug, PartialEq, Eq)]
pub struct SshPoweroff {
    pub private_key: String,
    pub vsock_cid: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    /// The main process's name, which names its units and its machine
    /// registration.
    pub program: &'static str,
    pub binary: String,
    pub args: Vec<Arg>,
    pub helpers: Vec<Helper>,
    /// Device nodes or groups the process may open. These borrow the
    /// `DeviceAllow=` spelling, the one place a plan uses a launcher's
    /// terms; a second launcher would map them.
    pub device_allow: Vec<&'static str>,
    /// Whether the process pins guest memory, for an assigned device, so
    /// that no locked-memory limit may bind it.
    pub lock_memory: bool,
    /// The guest's vsock CID and its UUID, for the machine registration.
    pub vsock_cid: Option<u64>,
    pub uuid: Option<String>,
    pub ssh_poweroff: Option<SshPoweroff>,
    /// The control socket the launcher's stop command talks to.
    pub control_socket: String,
}

/// Paths the launcher owns that a backend must embed in command lines.
pub trait Paths {
    /// The per-machine runtime directory of the main process, named
    /// after it.
    fn runtime_dir(&self, machine: &UnitName, program: &str) -> String;
    /// The socket the launcher listens on for a helper of the machine.
    fn helper_socket(&self, machine: &UnitName, program: &str, tag: &Tag) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags() {
        assert_eq!("home".parse::<Tag>().unwrap().to_string(), "home");
        assert!("lib.modules_1-2".parse::<Tag>().is_ok());
        for bad in ["", "a b", "a/b", "a@b", "a:b"] {
            assert!(bad.parse::<Tag>().is_err(), "{bad:?}");
        }
    }
}
