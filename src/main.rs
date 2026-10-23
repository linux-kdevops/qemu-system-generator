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

enum Command {
    Generate(Options),
    Help,
    Version,
}

struct Options {
    scope: Option<Scope>,
    dryrun: bool,
    output_dir: Option<PathBuf>,
}

#[derive(Debug)]
struct UsageError(String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for UsageError {}

/// The generator protocol: zero, one or three directories, of which
/// only the normal-priority one is written to.
fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let mut options = Options {
        scope: None,
        dryrun: false,
        output_dir: None,
    };
    let mut dirs: Vec<PathBuf> = Vec::new();
    for arg in args {
        match arg.to_str() {
            Some("--help") => return Ok(Command::Help),
            Some("--version") => return Ok(Command::Version),
            Some("--user") => options.scope = Some(Scope::User),
            Some("--system") => options.scope = Some(Scope::System),
            Some("--dryrun") => options.dryrun = true,
            Some(flag) if flag.starts_with('-') => {
                return Err(UsageError(format!("unknown option {flag}")));
            }
            _ => dirs.push(PathBuf::from(arg)),
        }
    }
    if !matches!(dirs.len(), 0 | 1 | 3) {
        return Err(UsageError("expected one or three directories".to_owned()));
    }
    options.output_dir = dirs.into_iter().next();
    if options.output_dir.is_none() && !options.dryrun {
        return Err(UsageError("no output directory".to_owned()));
    }
    Ok(Command::Generate(options))
}

#[derive(Debug)]
enum GenerateError {
    Read { path: PathBuf, source: io::Error },
    Parse { path: PathBuf, source: ParseError },
    Decode { path: PathBuf, source: DecodeError },
    Write { path: PathBuf, source: io::Error },
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } | Self::Write { path, source } => {
                write!(f, "{}: {source}", path.display())
            }
            Self::Parse { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Decode { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl Error for GenerateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Decode { source, .. } => Some(source),
        }
    }
}

fn read_unit_file(path: &Path) -> Result<UnitFile, GenerateError> {
    let text = fs::read_to_string(path).map_err(|source| GenerateError::Read {
        path: path.to_owned(),
        source,
    })?;
    text.parse().map_err(|source| GenerateError::Parse {
        path: path.to_owned(),
        source,
    })
}

/// The file with its drop-ins appended, in the order they apply.
fn read_source(source: &Source) -> Result<UnitFile, GenerateError> {
    let mut file = read_unit_file(&source.path)?;
    for dropin in &source.dropins {
        file.extend(read_unit_file(dropin)?);
    }
    Ok(file)
}

fn generate(source: &Source, scope: Scope) -> Result<Output, GenerateError> {
    let file = read_source(source)?;
    let decoded = backend::qemu::decode(&source.name, &file, &SystemdPaths).map_err(|source_| {
        GenerateError::Decode {
            path: source.path.clone(),
            source: source_,
        }
    })?;
    for warning in &decoded.warnings {
        log(format_args!("{}: {warning}", source.path.display()));
    }
    Ok(launcher::systemd::emit(
        source,
        &file,
        &decoded.plan,
        scope,
        |warning| log(format_args!("{}: {warning}", source.path.display())),
    ))
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
    requested
        .or_else(|| env::var("SYSTEMD_SCOPE").ok()?.parse().ok())
        .unwrap_or(if env.uid == 0 {
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
    for source in &sources {
        let result = generate(source, scope).and_then(|output| match &options.output_dir {
            Some(dir) if !options.dryrun => write_output(dir, &output),
            _ => print_output(&output, &mut stdout).map_err(|source_| GenerateError::Write {
                path: PathBuf::from("<stdout>"),
                source: source_,
            }),
        });
        if let Err(err) = result {
            log(err);
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
        assert!(o.dryrun && o.output_dir.is_none());
        let Ok(Command::Generate(o)) = parse_args(args(&["/n", "/e", "/l"])) else {
            panic!("three directories are the generator protocol");
        };
        assert_eq!(o.output_dir, Some(PathBuf::from("/n")));
        assert!(parse_args(args(&[])).is_err());
        assert!(parse_args(args(&["/a", "/b"])).is_err());
        assert!(parse_args(args(&["--bogus", "/a"])).is_err());
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
        if env::var_os("SYSTEMD_SCOPE").is_none() {
            assert_eq!(resolve_scope(None, &root), Scope::System);
            assert_eq!(resolve_scope(None, &user), Scope::User);
        }
    }
}
