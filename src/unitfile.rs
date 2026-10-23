// SPDX-License-Identifier: copyleft-next-0.3.1

//! systemd unit-file syntax, kept as ordered lines (`systemd.syntax(7)`).
//!
//! systemd's own parser is not a public API, so this module owns the
//! rules that matter here: `#` and `;` start comment lines, which are
//! skipped even inside a continuation, a trailing backslash continues
//! the line, keys are split from values at the first `=`, and sections
//! keep their lines in file order. What an empty value means is the
//! consumer's decision.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// One `Key=Value` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: String,
    /// Number of the physical line the entry starts on, from 1.
    pub line: usize,
}

/// One `[Name]` header and the lines under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub entries: Vec<Entry>,
    /// Number of the physical line the header is on, from 1.
    pub line: usize,
    /// The file the section is in, the `.qemu` file or a drop-in.
    pub path: PathBuf,
}

impl Section {
    /// A line of this section, as a diagnostic names it.
    pub fn at(&self, line: usize) -> String {
        format!("{}: line {line}", self.path.display())
    }
}

/// A parsed file: its sections in file order, repeated names kept apart.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitFile {
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    MalformedHeader(String),
    MissingEquals(String),
    EmptyKey,
    KeyBeforeSection(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.kind {
            ParseErrorKind::MalformedHeader(text) => {
                write!(f, "malformed section header {text:?}")
            }
            ParseErrorKind::MissingEquals(text) => write!(f, "expected KEY=VALUE, got {text:?}"),
            ParseErrorKind::EmptyKey => f.write_str("empty key"),
            ParseErrorKind::KeyBeforeSection(key) => {
                write!(f, "{key}= before any section header")
            }
        }
    }
}

impl Error for ParseError {}

/// Logical lines after continuation handling, each with the number of
/// its first physical line. As in systemd, the continuing backslash
/// becomes a space and the lines are joined with their whitespace.
fn logical_lines(text: &str) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    let mut pending: Option<(usize, String)> = None;
    for (number, raw) in (1..).zip(text.lines()) {
        if raw.trim_start_matches([' ', '\t']).starts_with(['#', ';']) {
            continue;
        }
        let (start, mut text) = pending.take().unwrap_or((number, String::new()));
        text.push_str(raw);
        // An even run of backslashes is escaped ones, not a continuation.
        let backslashes = text.len() - text.trim_end_matches('\\').len();
        if backslashes % 2 == 1 {
            text.pop();
            text.push(' ');
            pending = Some((start, text));
        } else {
            lines.push((start, text));
        }
    }
    lines.extend(pending);
    lines
}

/// A file read from no path, as the tests write one.
impl FromStr for UnitFile {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text, Path::new(""))
    }
}

impl UnitFile {
    pub fn parse(text: &str, path: &Path) -> Result<Self, ParseError> {
        let mut file = Self::default();
        for (line, logical) in logical_lines(text) {
            let logical = logical.trim();
            if logical.is_empty() {
                continue;
            }
            let fail = |kind| ParseError { line, kind };
            if let Some(rest) = logical.strip_prefix('[') {
                let name = rest
                    .strip_suffix(']')
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| fail(ParseErrorKind::MalformedHeader(logical.to_owned())))?;
                file.sections.push(Section {
                    name: name.to_owned(),
                    entries: Vec::new(),
                    line,
                    path: path.to_owned(),
                });
                continue;
            }
            let (key, value) = logical
                .split_once('=')
                .ok_or_else(|| fail(ParseErrorKind::MissingEquals(logical.to_owned())))?;
            let key = key.trim();
            if key.is_empty() {
                return Err(fail(ParseErrorKind::EmptyKey));
            }
            let section = file
                .sections
                .last_mut()
                .ok_or_else(|| fail(ParseErrorKind::KeyBeforeSection(key.to_owned())))?;
            section.entries.push(Entry {
                key: key.to_owned(),
                value: value.trim().to_owned(),
                line,
            });
        }
        Ok(file)
    }

    /// Every section with this name, in file order.
    pub fn sections_named<'a, 'n>(
        &'a self,
        name: &'n str,
    ) -> impl Iterator<Item = &'a Section> + use<'a, 'n> {
        self.sections.iter().filter(move |s| s.name == name)
    }

    /// All entries of all sections with this name, in file order.
    pub fn entries<'a, 'n>(
        &'a self,
        name: &'n str,
    ) -> impl Iterator<Item = &'a Entry> + use<'a, 'n> {
        self.sections_named(name).flat_map(|s| s.entries.iter())
    }

    /// The last value assigned to `key` in sections named `section`,
    /// which is the one systemd would keep for a scalar setting.
    pub fn last(&self, section: &str, key: &str) -> Option<&str> {
        self.entries(section)
            .filter(|e| e.key == key)
            .last()
            .map(|e| e.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> UnitFile {
        text.parse().expect("valid unit file")
    }

    #[test]
    fn sections_keep_line_order_and_repeat() {
        let f = parse("[A]\nx=1\ny=2\nx=3\n[B]\nz=\n[A]\nx=4\n");
        let lines: Vec<_> = f
            .entries("A")
            .map(|e| format!("{}={}", e.key, e.value))
            .collect();
        assert_eq!(lines, ["x=1", "y=2", "x=3", "x=4"]);
        assert_eq!(f.last("A", "x"), Some("4"));
        assert_eq!(f.last("B", "z"), Some(""));
        assert_eq!(f.sections_named("A").count(), 2);
    }

    #[test]
    fn continuation_skips_comment_lines() {
        let f = parse("[S]\nappend=console=ttyS0 \\\n# a comment\n  root=/dev/vda \\\n  rw\n");
        assert_eq!(
            f.last("S", "append"),
            Some("console=ttyS0    root=/dev/vda    rw")
        );
        assert_eq!(f.entries("S").next().map(|e| e.line), Some(2));
    }

    #[test]
    fn escaped_backslash_ends_the_line() {
        let f = parse("[S]\nappend=a\\\\\nm=2\n");
        assert_eq!(f.last("S", "append"), Some("a\\\\"));
        assert_eq!(f.last("S", "m"), Some("2"));
    }

    #[test]
    fn comment_lines_never_continue() {
        let f = parse("[S]\nm=1\n# note \\\nnographic=\n");
        let keys: Vec<_> = f.entries("S").map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["m", "nographic"]);
    }

    #[test]
    fn sections_name_their_file() {
        let f = UnitFile::parse("\n[S]\nx=1\n", Path::new("/d/10-a.conf")).unwrap();
        let section = &f.sections[0];
        assert_eq!(section.at(section.entries[0].line), "/d/10-a.conf: line 3");
    }

    #[test]
    fn whitespace_and_comments() {
        let f = parse("  ; c\n#c\n [S] \n  key = value with = sign  \n");
        assert_eq!(f.last("S", "key"), Some("value with = sign"));
    }

    #[test]
    fn errors_name_the_line() {
        let err = "[S]\n\nnovalue\n".parse::<UnitFile>().unwrap_err();
        assert_eq!(err.line, 3);
        assert_eq!(
            err.kind,
            ParseErrorKind::MissingEquals("novalue".to_owned())
        );
        assert!("key=value\n".parse::<UnitFile>().is_err());
        assert!("[S\n".parse::<UnitFile>().is_err());
        assert!("[]\n".parse::<UnitFile>().is_err());
        assert!("[S]\n=v\n".parse::<UnitFile>().is_err());
    }
}
