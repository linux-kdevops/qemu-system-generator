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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg(pub Vec<String>);

impl<const N: usize> From<[String; N]> for Arg {
    fn from(words: [String; N]) -> Self {
        Self(words.into())
    }
}

/// The mount tag of a share, which also names the helper that serves it
/// and so must fit in a unit name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tag(String);

#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Helper {
    /// The daemon's name, which names its units.
    pub program: &'static str,
    pub tag: Tag,
    pub binary: String,
    pub args: Vec<Arg>,
    /// Host paths the daemon serves; the launcher orders it after their
    /// mounts.
    pub requires_mounts: Vec<String>,
}

/// A guest shutdown asked for over SSH, before the backend's own
/// channel is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshPoweroff {
    pub private_key: String,
    pub vsock_cid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub binary: String,
    pub args: Vec<Arg>,
    pub helpers: Vec<Helper>,
    /// Device nodes or groups the process may open, in the
    /// `DeviceAllow=` spelling.
    pub device_allow: Vec<&'static str>,
    /// Locked memory the process needs, in MiB, when it pins guest
    /// memory for an assigned device.
    pub memlock_mib: Option<u64>,
    /// The guest's vsock CID and its UUID, for the machine registration.
    pub vsock_cid: Option<u64>,
    pub uuid: Option<String>,
    pub ssh_poweroff: Option<SshPoweroff>,
    /// The control socket the launcher's stop command talks to.
    pub control_socket: String,
}

/// Paths the launcher owns that a backend must embed in command lines.
pub trait Paths {
    /// The per-machine runtime directory of the main process.
    fn runtime_dir(&self, machine: &UnitName) -> String;
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
