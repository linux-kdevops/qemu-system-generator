// SPDX-License-Identifier: copyleft-next-0.3.1

//! The members of a JSON object, read as QEMU reads an option written as
//! JSON (`qobject_from_json()`): RFC 8259 with single-quoted strings and
//! a `\'` escape (`qobject/json-lexer.c`). The decoder looks up a few
//! members, so each value is kept as written and a string is decoded on
//! request. Text that QEMU refuses may still be read; QEMU reports it.

/// QEMU refuses deeper nesting (`qobject/json-streamer.c`).
const MAX_NESTING: usize = 1024;

/// The members of the one object `text` holds, each value as written.
pub fn members(text: &str) -> Option<Vec<(String, &str)>> {
    let mut scanner = Scanner::new(text);
    scanner.skip_space();
    let members = scanner.object()?;
    scanner.skip_space();
    scanner.at_end().then_some(members)
}

/// The string a value written as a JSON string holds.
pub fn string(value: &str) -> Option<String> {
    let mut scanner = Scanner::new(value);
    let string = scanner.string()?;
    scanner.at_end().then_some(string)
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    depth: usize,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            pos: 0,
            depth: 0,
        }
    }

    fn at_end(&self) -> bool {
        self.pos == self.text.len()
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat(&mut self, c: char) -> bool {
        let found = self.peek() == Some(c);
        if found {
            self.pos += c.len_utf8();
        }
        found
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.pos += 1;
        }
    }

    /// Comma-separated items between `open` and `close`, each read by
    /// `item`.
    fn items(
        &mut self,
        open: char,
        close: char,
        mut item: impl FnMut(&mut Self) -> Option<()>,
    ) -> Option<()> {
        if self.depth == MAX_NESTING || !self.eat(open) {
            return None;
        }
        self.depth += 1;
        self.skip_space();
        if !self.eat(close) {
            loop {
                self.skip_space();
                item(self)?;
                self.skip_space();
                if self.eat(close) {
                    break;
                }
                if !self.eat(',') {
                    return None;
                }
            }
        }
        self.depth -= 1;
        Some(())
    }

    fn object(&mut self) -> Option<Vec<(String, &'a str)>> {
        let mut members = Vec::new();
        self.items('{', '}', |s| {
            let key = s.string()?;
            s.skip_space();
            if !s.eat(':') {
                return None;
            }
            s.skip_space();
            members.push((key, s.value()?));
            Some(())
        })?;
        Some(members)
    }

    fn value(&mut self) -> Option<&'a str> {
        let start = self.pos;
        match self.peek()? {
            '{' => {
                self.object()?;
            }
            '[' => self.items('[', ']', |s| s.value().map(drop))?,
            '"' | '\'' => {
                self.string()?;
            }
            _ => {
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
                {
                    self.pos += 1;
                }
                if self.pos == start {
                    return None;
                }
            }
        }
        Some(&self.text[start..self.pos])
    }

    fn string(&mut self) -> Option<String> {
        let quote = self.next().filter(|c| matches!(c, '"' | '\''))?;
        let mut string = String::new();
        loop {
            let c = match self.next()? {
                c if c == quote => return Some(string),
                '\\' => match self.next()? {
                    c @ ('"' | '\'' | '\\' | '/') => c,
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'u' => self.unicode_escape()?,
                    _ => return None,
                },
                c if c < ' ' => return None,
                c => c,
            };
            string.push(c);
        }
    }

    /// The character of a `\u` escape, of two for a surrogate pair.
    fn unicode_escape(&mut self) -> Option<char> {
        let high = self.hex4()?;
        if !(0xD800..0xDC00).contains(&high) {
            return char::from_u32(high);
        }
        if !(self.eat('\\') && self.eat('u')) {
            return None;
        }
        let low = self.hex4()?;
        if !(0xDC00..0xE000).contains(&low) {
            return None;
        }
        char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00))
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits = self.text.get(self.pos..self.pos + 4)?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        self.pos += 4;
        u32::from_str_radix(digits, 16).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_members() {
        let found = members(
            r#" {"driver": "vfio-user-pci", 'socket': {"type": "unix", "path": "/a,b}"}, "x": [1, true, null, -2.5e3, []], "guest-cid":42} "#,
        )
        .unwrap();
        let keys: Vec<_> = found.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["driver", "socket", "x", "guest-cid"]);
        assert_eq!(found[0].1, r#""vfio-user-pci""#);
        assert_eq!(found[1].1, r#"{"type": "unix", "path": "/a,b}"}"#);
        assert_eq!(found[3].1, "42");
        assert_eq!(members("{ }"), Some(vec![]));
    }

    #[test]
    fn not_one_object() {
        for bad in [
            "",
            "[]",
            "{",
            r#"{"a"}"#,
            r#"{"a":}"#,
            r#"{"a":1,}"#,
            r#"{"a":1} x"#,
            "{a:1}",
            r#"{"a":[1 2]}"#,
        ] {
            assert_eq!(members(bad), None, "{bad}");
        }
        let nested = |depth: usize| {
            format!(
                r#"{{"a":{}{}}}"#,
                "[".repeat(depth - 1),
                "]".repeat(depth - 1)
            )
        };
        assert!(members(&nested(MAX_NESTING)).is_some());
        assert_eq!(members(&nested(MAX_NESTING + 1)), None);
    }

    #[test]
    fn strings() {
        assert_eq!(string(r#""vfio-pci""#).as_deref(), Some("vfio-pci"));
        assert_eq!(string(r#"'it\'s "x"'"#).as_deref(), Some(r#"it's "x""#));
        assert_eq!(string(r#""A\n\/😀""#).as_deref(), Some("A\n/\u{1f600}"));
        for bad in [
            r#""a"#,
            r#""\x""#,
            r#""\ud83d""#,
            r#""\ude00""#,
            r#""\u+041""#,
            "\"a\u{1}\"",
            r#""a"b"#,
            "42",
        ] {
            assert_eq!(string(bad), None, "{bad}");
        }
    }
}
