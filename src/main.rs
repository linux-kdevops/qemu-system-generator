// SPDX-License-Identifier: copyleft-next-0.3.1

//! qemu-system-generator: a systemd generator (`systemd.generator(7)`)
//! that reads `.qemu` files and writes the units that run them.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
// Unit text names its directives and specifiers in prose, and every
// public item here is internal to one binary.
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::must_use_candidate
)]

mod backend;
mod launcher;
mod plan;
mod search;
mod unitfile;

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Write as _};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use backend::qemu::DecodeError;
use launcher::systemd::{GENERATOR, Output, SystemdPaths};
use search::{Environment, Scope, Source};
use unitfile::{ParseError, UnitFile};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
Usage: qemu-system-generator [--user|--system] [--dryrun] [NORMAL_DIR [EARLY_DIR LATE_DIR]]
       qemu-system-generator --version
       qemu-system-generator --help

Reads .qemu files from the search path (see qemu-systemd.unit(5)) and
writes systemd units into NORMAL_DIR. With --dryrun the units are printed
instead. The scope comes from --user/--system, else $SYSTEMD_SCOPE, else
the effective user id.";

/// Exit status of a usage error, as `EX_USAGE` would be but kept small
/// so the service manager's log shows it at a glance.
const EXIT_USAGE: u8 = 2;

fn log(message: impl fmt::Display) {
    eprintln!("{GENERATOR}: {message}");
}

/// An error and each of its causes, which its `Display` leaves out.
fn log_error(err: &dyn Error) {
    let mut message = err.to_string();
    let mut cause = err.source();
    while let Some(err) = cause {
        message = format!("{message}: {err}");
        cause = err.source();
    }
    log(message);
}

enum Command {
    Generate(Options),
    Help,
    Version,
}

/// Where the units go: the normal-priority directory, or standard
/// output for `--dryrun`.
#[derive(Debug, PartialEq, Eq)]
enum Sink {
    Dir(PathBuf),
    Stdout,
}

struct Options {
    scope: Option<Scope>,
    sink: Sink,
}

#[derive(Debug, PartialEq, Eq)]
enum UsageError {
    UnknownOption(String),
    DirectoryCount(usize),
    NoOutputDir,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOption(flag) => write!(f, "unknown option {flag}"),
            Self::DirectoryCount(n) => write!(f, "expected one or three directories, got {n}"),
            Self::NoOutputDir => f.write_str("no output directory"),
        }
    }
}

impl Error for UsageError {}

/// The generator protocol: zero, one or three directories, of which
/// only the normal-priority one is written to.
fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let mut scope = None;
    let mut dryrun = false;
    let mut dirs: Vec<PathBuf> = Vec::new();
    for arg in args {
        match arg.to_str() {
            Some("--help") => return Ok(Command::Help),
            Some("--version") => return Ok(Command::Version),
            Some("--user") => scope = Some(Scope::User),
            Some("--system") => scope = Some(Scope::System),
            Some("--dryrun") => dryrun = true,
            Some(flag) if flag.starts_with('-') => {
                return Err(UsageError::UnknownOption(flag.to_owned()));
            }
            _ => dirs.push(PathBuf::from(arg)),
        }
    }
    if !matches!(dirs.len(), 0 | 1 | 3) {
        return Err(UsageError::DirectoryCount(dirs.len()));
    }
    let sink = match dirs.into_iter().next() {
        _ if dryrun => Sink::Stdout,
        Some(dir) => Sink::Dir(dir),
        None => return Err(UsageError::NoOutputDir),
    };
    Ok(Command::Generate(Options { scope, sink }))
}

#[derive(Debug)]
enum GenerateError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: ParseError,
    },
    Decode {
        path: PathBuf,
        source: DecodeError,
    },
    Write {
        path: PathBuf,
        source: io::Error,
    },
    Conflict {
        path: PathBuf,
        unit: PathBuf,
        first: PathBuf,
    },
    Stdout(io::Error),
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, .. }
            | Self::Write { path, .. }
            | Self::Parse { path, .. }
            | Self::Decode { path, .. } => write!(f, "{}", path.display()),
            Self::Stdout(_) => f.write_str("standard output"),
            Self::Conflict { path, unit, first } => write!(
                f,
                "{}: {} is already generated from {}",
                path.display(),
                unit.display(),
                first.display()
            ),
        }
    }
}

impl Error for GenerateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } | Self::Stdout(source) => {
                Some(source)
            }
            Self::Parse { source, .. } => Some(source),
            Self::Decode { source, .. } => Some(source),
            Self::Conflict { .. } => None,
        }
    }
}

fn read_unit_file(path: &Path) -> Result<UnitFile, GenerateError> {
    let text = fs::read_to_string(path).map_err(|source| GenerateError::Read {
        path: path.to_owned(),
        source,
    })?;
    UnitFile::parse(&text, path).map_err(|source| GenerateError::Parse {
        path: path.to_owned(),
        source,
    })
}

/// The file with its drop-ins' sections appended, in the order they
/// apply.
fn read_source(source: &Source) -> Result<UnitFile, GenerateError> {
    let mut file = read_unit_file(&source.path)?;
    for dropin in &source.dropins {
        file.sections.extend(read_unit_file(dropin)?.sections);
    }
    Ok(file)
}

/// Sections that neither the backend nor the launcher reads.
fn unknown_sections(file: &UnitFile) -> impl Iterator<Item = String> + '_ {
    file.sections
        .iter()
        .filter(|s| {
            let name = s.name.as_str();
            !backend::qemu::SECTIONS.contains(&name) && !launcher::systemd::SECTIONS.contains(&name)
        })
        .map(|s| format!("{}: unknown section [{}], ignoring", s.at(s.line), s.name))
}

fn generate(source: &Source, scope: Scope) -> Result<Output, GenerateError> {
    let file = read_source(source)?;
    unknown_sections(&file).for_each(log);
    let decoded = backend::qemu::decode(&source.name, &file, &SystemdPaths).map_err(|source_| {
        GenerateError::Decode {
            path: source_.path().unwrap_or(&source.path).to_owned(),
            source: source_,
        }
    })?;
    for warning in &decoded.warnings {
        log(warning);
    }
    Ok(launcher::systemd::emit(
        source,
        &file,
        &decoded.plan,
        scope,
        log,
    ))
}

/// Record the units of `output` as generated from `source`, unless an
/// earlier file generated one of them: `a-b.qemu` with tag `c` and
/// `a.qemu` with tag `b-c` both name `virtiofsd@a-b-c`.
fn claim(
    claimed: &mut BTreeMap<PathBuf, PathBuf>,
    source: &Source,
    output: &Output,
) -> Result<(), GenerateError> {
    if let Some((unit, first)) = output
        .files
        .iter()
        .find_map(|f| Some((&f.path, claimed.get(&f.path)?)))
    {
        return Err(GenerateError::Conflict {
            path: source.path.clone(),
            unit: unit.clone(),
            first: first.clone(),
        });
    }
    for file in &output.files {
        claimed.insert(file.path.clone(), source.path.clone());
    }
    Ok(())
}

fn write_output(dest: &Path, output: &Output) -> Result<(), GenerateError> {
    let failed = |path: &Path, source| GenerateError::Write {
        path: path.to_owned(),
        source,
    };
    for file in &output.files {
        let path = dest.join(&file.path);
        fs::write(&path, &file.text).map_err(|e| failed(&path, e))?;
    }
    for link in &output.links {
        let path = dest.join(&link.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| failed(parent, e))?;
        }
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(failed(&path, e)),
        }
        symlink(&link.target, &path).map_err(|e| failed(&path, e))?;
    }
    Ok(())
}

/// The dry-run format: each file after a `---path---` line, each
/// symlink as `---path -> target---`.
fn print_output(output: &Output, out: &mut impl io::Write) -> io::Result<()> {
    for file in &output.files {
        write!(out, "---{}---\n{}", file.path.display(), file.text)?;
    }
    for link in &output.links {
        writeln!(
            out,
            "---{} -> {}---",
            link.path.display(),
            link.target.display()
        )?;
    }
    Ok(())
}

fn resolve_scope(requested: Option<Scope>, env: &Environment) -> Scope {
    requested.or(env.scope).unwrap_or(if env.uid == 0 {
        Scope::System
    } else {
        Scope::User
    })
}

fn run(options: &Options) -> ExitCode {
    let env = Environment::from_process();
    let scope = resolve_scope(options.scope, &env);
    let dirs = search::unit_dirs(scope, &env);
    let sources = search::discover(&dirs, log);
    let stdout = io::stdout();
    let mut stdout = io::BufWriter::new(stdout.lock());
    let mut failed = false;
    let mut claimed = BTreeMap::new();
    for source in &sources {
        let result = generate(source, scope)
            .and_then(|output| claim(&mut claimed, source, &output).map(|()| output))
            .and_then(|output| match &options.sink {
                Sink::Dir(dir) => write_output(dir, &output),
                Sink::Stdout => print_output(&output, &mut stdout).map_err(GenerateError::Stdout),
            });
        if let Err(err) = result {
            log_error(&err);
            failed = true;
        }
    }
    if let Err(err) = stdout.flush() {
        log(err);
        failed = true;
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    match parse_args(env::args_os().skip(1)) {
        Ok(Command::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("{GENERATOR} {VERSION}");
            ExitCode::SUCCESS
        }
        Ok(Command::Generate(options)) => run(&options),
        Err(err) => {
            log(err);
            eprintln!("{USAGE}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn argument_forms() {
        assert!(matches!(parse_args(args(&["--help"])), Ok(Command::Help)));
        assert!(matches!(
            parse_args(args(&["--version"])),
            Ok(Command::Version)
        ));
        let Ok(Command::Generate(o)) = parse_args(args(&["--user", "--dryrun"])) else {
            panic!("dryrun needs no directory");
        };
        assert_eq!(o.scope, Some(Scope::User));
        assert_eq!(o.sink, Sink::Stdout);
        let Ok(Command::Generate(o)) = parse_args(args(&["/n", "/e", "/l"])) else {
            panic!("three directories are the generator protocol");
        };
        assert_eq!(o.sink, Sink::Dir(PathBuf::from("/n")));
        assert!(matches!(
            parse_args(args(&[])),
            Err(UsageError::NoOutputDir)
        ));
        assert!(matches!(
            parse_args(args(&["/a", "/b"])),
            Err(UsageError::DirectoryCount(2))
        ));
        assert!(matches!(
            parse_args(args(&["--bogus", "/a"])),
            Err(UsageError::UnknownOption(_))
        ));
    }

    #[test]
    fn unknown_sections_are_named() {
        let file: UnitFile = "[Bogus]\nx=1\n[QEMU]\n[Service]\n[VM]\n".parse().unwrap();
        let unknown: Vec<_> = unknown_sections(&file).collect();
        assert_eq!(unknown, [": line 1: unknown section [Bogus], ignoring"]);
    }

    #[test]
    fn scope_falls_back_to_the_user_id() {
        let root = Environment {
            uid: 0,
            ..Environment::default()
        };
        let user = Environment {
            uid: 1000,
            ..Environment::default()
        };
        assert_eq!(resolve_scope(Some(Scope::User), &root), Scope::User);
        assert_eq!(resolve_scope(None, &root), Scope::System);
        assert_eq!(resolve_scope(None, &user), Scope::User);
        let scoped = Environment {
            scope: Some(Scope::User),
            ..root
        };
        assert_eq!(resolve_scope(None, &scoped), Scope::User);
    }
}
