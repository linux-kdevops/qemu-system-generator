// SPDX-License-Identifier: copyleft-next-0.3.1

//! Where `.qemu` files live and how one file and its drop-ins are found.
//! The search path mirrors Quadlet's (`podman-systemd.unit(5)`), with
//! `qemu/systemd/` in place of `containers/systemd/`.

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const SUFFIX: &str = "qemu";
const UNIT_DIRS_ENV: &str = "QEMU_SYSTEM_GENERATOR_UNIT_DIRS";

/// The service manager generated for (`systemd.generator(7)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    System,
    User,
}

/// What the search path depends on, read from the process once so that
/// the rest of the module is a pure function of it.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Environment {
    /// `$SYSTEMD_SCOPE` (`systemd.generator(7)`).
    pub scope: Option<Scope>,
    pub unit_dirs: Option<OsString>,
    pub xdg_runtime_dir: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub uid: u32,
}

impl Environment {
    pub fn from_process() -> Self {
        Self {
            scope: match env::var("SYSTEMD_SCOPE").as_deref() {
                Ok("system") => Some(Scope::System),
                Ok("user") => Some(Scope::User),
                _ => None,
            },
            unit_dirs: env::var_os(UNIT_DIRS_ENV),
            xdg_runtime_dir: env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from),
            xdg_config_home: env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            home: env::var_os("HOME").map(PathBuf::from),
            // The owner of /proc/self is the effective user id, which
            // the kernel sets from the process's credentials (proc(5)).
            uid: fs::metadata("/proc/self").map_or(0, |m| m.uid()),
        }
    }
}

/// The directories searched for `.qemu` files, highest precedence first.
pub fn unit_dirs(scope: Scope, env: &Environment) -> Vec<PathBuf> {
    if let Some(list) = &env.unit_dirs {
        return env::split_paths(list)
            .filter(|d| !d.as_os_str().is_empty())
            .collect();
    }
    let relative = Path::new(SUFFIX).join("systemd");
    match scope {
        Scope::System => ["/run", "/etc", "/usr/share"]
            .iter()
            .map(|root| Path::new(root).join(&relative))
            .collect(),
        Scope::User => {
            let uid = env.uid.to_string();
            let runtime = env.xdg_runtime_dir.as_ref().map(|d| d.join(&relative));
            let config = env
                .xdg_config_home
                .clone()
                .or_else(|| env.home.as_ref().map(|h| h.join(".config")))
                .map(|d| d.join(&relative));
            let shared = ["/etc", "/usr/share"].into_iter().flat_map(|root| {
                let users = Path::new(root).join(&relative).join("users");
                [users.join(&uid), users]
            });
            runtime.into_iter().chain(config).chain(shared).collect()
        }
    }
}

/// The stem of a `.qemu` file, which becomes the instance of the
/// generated units: the characters systemd allows in a unit name
/// (`unit-name.c`), without `@` so that it cannot read as a template.
#[derive(Debug, PartialEq, Eq)]
pub struct UnitName(String);

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidUnitName(pub String);

impl fmt::Display for InvalidUnitName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} is not a unit name: letters, digits, ':', '_', '.' and '-' only",
            self.0
        )
    }
}

impl Error for InvalidUnitName {}

impl FromStr for UnitName {
    type Err = InvalidUnitName;

    fn from_str(stem: &str) -> Result<Self, Self::Err> {
        let valid = !stem.is_empty()
            && stem
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '.' | '-'));
        if valid {
            Ok(Self(stem.to_owned()))
        } else {
            Err(InvalidUnitName(stem.to_owned()))
        }
    }
}

impl UnitName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnitName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One `.qemu` file to generate from, with its drop-ins in the order
/// they apply.
#[derive(Debug, PartialEq, Eq)]
pub struct Source {
    pub name: UnitName,
    pub path: PathBuf,
    pub dropins: Vec<PathBuf>,
}

/// A name systemd skips in a drop-in directory (`FOREACH_DIRENT()` in
/// `src/basic/conf-files.c`): of the rules of `hidden_or_backup_file()`
/// (`src/basic/path-util.c`), the leading dot is the only one a `.qemu`,
/// `.qemu.d` or `.conf` name can meet. systemd loads a unit whose name
/// starts with a dot; a `.qemu` is skipped too so that an editor's lock
/// file, `.#name.qemu`, is not a machine.
fn hidden(file_name: &str) -> bool {
    file_name.starts_with('.')
}

fn unit_stem(file_name: &str) -> Option<&str> {
    file_name
        .strip_suffix(SUFFIX)
        .and_then(|s| s.strip_suffix('.'))
}

/// The stem a drop-in directory applies to: `""` for the type directory
/// `qemu.d/`, `foo-` for `foo-.qemu.d/`, `foo` for `foo.qemu.d/`.
fn dropin_stem(dir_name: &str) -> Option<&str> {
    let base = dir_name.strip_suffix(".d")?;
    if base == SUFFIX {
        return Some("");
    }
    unit_stem(base)
}

/// Find every `.qemu` file on the search path. The first directory that
/// holds a name wins. Unreadable directories and unusable names are
/// reported through `warn` and skipped.
pub fn discover(dirs: &[PathBuf], mut warn: impl FnMut(String)) -> Vec<Source> {
    let mut files: BTreeMap<String, PathBuf> = BTreeMap::new();
    // One map per search directory: the stem a drop-in directory applies
    // to, and its path.
    let mut dropin_dirs: Vec<BTreeMap<String, PathBuf>> = Vec::new();
    for dir in dirs {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                warn(format!("{}: {err}", dir.display()));
                continue;
            }
        };
        let mut found = BTreeMap::new();
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str().filter(|n| !hidden(n)) else {
                continue;
            };
            let path = entry.path();
            if let Some(stem) = unit_stem(file_name) {
                if path.is_file() {
                    files.entry(stem.to_owned()).or_insert(path);
                } else if !path.is_dir() {
                    warn(format!("{}: not a regular file, ignoring", path.display()));
                }
            } else if path.is_dir()
                && let Some(stem) = dropin_stem(file_name)
            {
                found.insert(stem.to_owned(), path);
            }
        }
        dropin_dirs.push(found);
    }

    files
        .into_iter()
        .filter_map(|(stem, path)| match stem.parse::<UnitName>() {
            Ok(name) => {
                let dropins = collect_dropins(&name, &dropin_dirs);
                Some(Source {
                    name,
                    path,
                    dropins,
                })
            }
            Err(err) => {
                warn(format!("{}: {err}, ignoring", path.display()));
                None
            }
        })
        .collect()
}

/// The name-specific drop-in stems of a unit, most specific first: its
/// own name, then each dash-truncated prefix. The type directory is the
/// empty stem.
fn dropin_levels(name: &UnitName) -> impl Iterator<Item = String> + '_ {
    let stem = name.as_str();
    let prefixes = stem
        .char_indices()
        .rev()
        .filter(|&(_, c)| c == '-')
        .map(|(i, _)| stem[..=i].to_owned());
    std::iter::once(stem.to_owned()).chain(prefixes)
}

/// The drop-ins that apply to a unit, in the order they are appended: by
/// file name, where the first directory to hold a name wins. The
/// directories come in systemd's order (`unit_file_find_dropin_paths()`
/// in `src/shared/dropin.c`): for each search directory its
/// name-specific levels, most specific first, then the type directory of
/// every search directory.
fn collect_dropins(name: &UnitName, dropin_dirs: &[BTreeMap<String, PathBuf>]) -> Vec<PathBuf> {
    let levels: Vec<String> = dropin_levels(name).collect();
    let ordered = dropin_dirs
        .iter()
        .flat_map(|found| levels.iter().filter_map(|level| found.get(level)))
        .chain(dropin_dirs.iter().filter_map(|found| found.get("")));
    let mut chosen: BTreeMap<String, PathBuf> = BTreeMap::new();
    for entry in ordered
        .filter_map(|d| fs::read_dir(d).ok())
        .flatten()
        .flatten()
    {
        let path = entry.path();
        let Some(file_name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !hidden(n))
        else {
            continue;
        };
        // The suffix is matched byte for byte, as systemd matches it
        // (endswith() in src/basic/conf-files.c).
        if path.extension().is_some_and(|ext| ext == "conf") && path.is_file() {
            chosen.entry(file_name.to_owned()).or_insert(path);
        }
    }
    chosen.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_names() {
        assert_eq!("nixos".parse::<UnitName>().unwrap().as_str(), "nixos");
        assert!("fstests-1.x_y:z".parse::<UnitName>().is_ok());
        for bad in ["", "fstests@", "fstests@xfs", "bad name", "a/b"] {
            assert!(bad.parse::<UnitName>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn stems() {
        assert_eq!(unit_stem("nixos.qemu"), Some("nixos"));
        assert_eq!(unit_stem("nixos.qemu.d"), None);
        assert_eq!(unit_stem("qemu"), None);
        assert_eq!(dropin_stem("qemu.d"), Some(""));
        assert_eq!(dropin_stem("nixos.qemu.d"), Some("nixos"));
        assert_eq!(dropin_stem("fstests-.qemu.d"), Some("fstests-"));
        assert_eq!(dropin_stem("nixos.qemu"), None);
    }

    #[test]
    fn levels() {
        let name: UnitName = "fstests-xfs-1".parse().unwrap();
        let levels: Vec<_> = dropin_levels(&name).collect();
        assert_eq!(levels, ["fstests-xfs-1", "fstests-xfs-", "fstests-"]);
    }

    #[test]
    fn search_path() {
        let env = Environment {
            unit_dirs: Some("/a:/b:".into()),
            ..Environment::default()
        };
        assert_eq!(
            unit_dirs(Scope::System, &env),
            [PathBuf::from("/a"), PathBuf::from("/b")]
        );

        let env = Environment::default();
        assert_eq!(
            unit_dirs(Scope::System, &env),
            [
                PathBuf::from("/run/qemu/systemd"),
                PathBuf::from("/etc/qemu/systemd"),
                PathBuf::from("/usr/share/qemu/systemd"),
            ]
        );

        let env = Environment {
            xdg_runtime_dir: Some("/run/user/1000".into()),
            home: Some("/home/me".into()),
            uid: 1000,
            ..Environment::default()
        };
        assert_eq!(
            unit_dirs(Scope::User, &env),
            [
                PathBuf::from("/run/user/1000/qemu/systemd"),
                PathBuf::from("/home/me/.config/qemu/systemd"),
                PathBuf::from("/etc/qemu/systemd/users/1000"),
                PathBuf::from("/etc/qemu/systemd/users"),
                PathBuf::from("/usr/share/qemu/systemd/users/1000"),
                PathBuf::from("/usr/share/qemu/systemd/users"),
            ]
        );

        let env = Environment {
            xdg_config_home: Some("/cfg".into()),
            home: Some("/home/me".into()),
            ..Environment::default()
        };
        assert_eq!(
            unit_dirs(Scope::User, &env)[0],
            PathBuf::from("/cfg/qemu/systemd")
        );
    }

    #[test]
    fn discovery_and_dropins() {
        let tmp = env::temp_dir().join(format!(
            "qemu-system-generator-search-{}",
            std::process::id()
        ));
        let high = tmp.join("high");
        let low = tmp.join("low");
        for dir in [&high, &low] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(low.join("nixos.qemu"), "[QEMU]\nm=1\n").unwrap();
        fs::write(high.join("nixos.qemu"), "[QEMU]\nm=2\n").unwrap();
        fs::write(low.join("fstests-xfs.qemu"), "[QEMU]\n").unwrap();
        fs::write(low.join("bad@name.qemu"), "[QEMU]\n").unwrap();
        fs::create_dir_all(low.join("fstests-.qemu.d")).unwrap();
        fs::write(low.join("fstests-.qemu.d/10-a.conf"), "").unwrap();
        fs::create_dir_all(low.join("qemu.d")).unwrap();
        fs::write(low.join("qemu.d/10-a.conf"), "").unwrap();
        fs::write(low.join("qemu.d/00-z.conf"), "").unwrap();
        fs::write(low.join("qemu.d/README"), "").unwrap();
        fs::write(low.join("qemu.d/.30-hidden.conf"), "").unwrap();
        fs::write(low.join(".hidden.qemu"), "[QEMU]\n").unwrap();
        fs::create_dir_all(high.join("nixos.qemu.d")).unwrap();
        fs::write(high.join("nixos.qemu.d/20-b.conf"), "").unwrap();
        fs::create_dir_all(high.join("fstests-.qemu.d")).unwrap();
        fs::write(high.join("fstests-.qemu.d/10-a.conf"), "").unwrap();
        fs::create_dir_all(low.join("fstests-xfs.qemu.d")).unwrap();
        fs::write(low.join("fstests-xfs.qemu.d/10-a.conf"), "").unwrap();

        let mut warnings = Vec::new();
        let sources = discover(&[high.clone(), low.clone()], |w| warnings.push(w));
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("bad@name.qemu"), "{warnings:?}");
        let names: Vec<_> = sources.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["fstests-xfs", "nixos"]);

        let nixos = &sources[1];
        assert_eq!(nixos.path, high.join("nixos.qemu"));
        assert_eq!(
            nixos.dropins,
            [
                low.join("qemu.d/00-z.conf"),
                low.join("qemu.d/10-a.conf"),
                high.join("nixos.qemu.d/20-b.conf"),
            ]
        );

        let xfs = &sources[0];
        assert_eq!(xfs.path, low.join("fstests-xfs.qemu"));
        // 10-a.conf is at three places: the higher search directory wins
        // over the more specific level, and both over qemu.d/.
        assert_eq!(
            xfs.dropins,
            [
                low.join("qemu.d/00-z.conf"),
                high.join("fstests-.qemu.d/10-a.conf"),
            ]
        );

        fs::remove_dir_all(&tmp).unwrap();
    }
}
