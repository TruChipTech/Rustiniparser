//! # rustiniparser
//!
//! Author: Anand <truchipinfo@gmail.com>
//! Version: V1.0.1
//!
//! A tiny **in-memory INI store**. You feed it INI text, it parses everything
//! into a [`Document`], and then you:
//!
//! - **read values back** converted to the type you want — string, int,
//!   float, bool;
//! - **modify** existing fields;
//! - **add** new entries (and remove them).
//!
//! It uses `std` collections so a document grows as needed and keys and values
//! may be of any length. Optional [`Limits`] give fixed-capacity behaviour when
//! you want it.
//!
//! ## At a glance
//!
//! ```
//! use rustiniparser::Document;
//!
//! let text = "\
//! [network]
//! host = example.com
//! port = 8080
//! enabled = yes
//! ";
//!
//! let mut doc = Document::new();
//! doc.load(text).unwrap();
//!
//! assert_eq!(doc.get_string("network", "host", "localhost"), "example.com");
//! assert_eq!(doc.get_int("network", "port", 80), 8080);
//! assert!(doc.get_bool("network", "enabled", false));
//!
//! doc.set_int("network", "port", 9090);          // modify existing
//! doc.set("network", "gateway", "10.0.0.1");     // add new
//! ```
//!
//! ## Accepted INI syntax
//!
//! - `[sections]`, plus a **global section** (`""`) for keys before any header
//! - `key = value` and `key : value`
//! - `;` and `#` comment lines, and inline comments after a value
//! - single (`'...'`) and double (`"..."`) quoted values
//! - a few escapes inside double quotes: `\\ \" \n \r \t`
//! - empty values (`key =`)
//! - LF, CRLF and lone-CR line endings
//!
//! Lookups are **case-sensitive** for section and key names.

// The pure-Rust core below contains no `unsafe`. The only `unsafe` in the
// crate lives in the C-ABI layer (`ffi` module), which is required to expose
// the C/C++ entry points and is annotated there.

use std::fmt;

pub mod ffi;

/// Library version string.
pub const VERSION: &str = "V1.0.1";

/// Result / error codes for parsing and mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Malformed line in the input.
    Syntax,
    /// A section/key/value exceeded a configured limit (unused unless limits
    /// are set via [`Document::with_limits`]).
    TooLong,
    /// The document is at its configured entry capacity.
    Full,
    /// Requested section/key does not exist.
    NotFound,
}

impl Error {
    /// Human-readable description of the error.
    pub fn as_str(&self) -> &'static str {
        match self {
            Error::Syntax => "syntax error",
            Error::TooLong => "value too long for storage",
            Error::Full => "document is full",
            Error::NotFound => "key not found",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// One stored key/value pair. Treat as opaque; iterate with
/// [`Document::entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub section: String,
    pub key: String,
    pub value: String,
}

/// Optional compile-time-style capacity limits.
///
/// By default a [`Document`] has no limits at all and grows freely. Set limits
/// with [`Document::with_limits`] when you want fixed-capacity behaviour that
/// rejects oversized input.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum number of key/value pairs (`None` = unlimited).
    pub max_entries: Option<usize>,
    /// Max section-name length in bytes (`None` = unlimited).
    pub max_section: Option<usize>,
    /// Max key length in bytes (`None` = unlimited).
    pub max_key: Option<usize>,
    /// Max value length in bytes (`None` = unlimited).
    pub max_value: Option<usize>,
}

impl Limits {
    /// A conservative fixed capacity: 64 entries; section/key up to 63 bytes,
    /// value up to 127 bytes.
    pub const COMPACT: Limits = Limits {
        max_entries: Some(64),
        max_section: Some(63),
        max_key: Some(63),
        max_value: Some(127),
    };
}

#[allow(clippy::derivable_impls)]
impl Default for Limits {
    /// No limits — a document grows freely. Use [`Limits::COMPACT`] for a
    /// fixed capacity.
    fn default() -> Self {
        Limits {
            max_entries: None,
            max_section: None,
            max_key: None,
            max_value: None,
        }
    }
}

/// An in-memory INI document: an ordered collection of [`Entry`] values.
#[derive(Debug, Clone, Default)]
pub struct Document {
    entries: Vec<Entry>,
    limits: Limits,
}

impl Document {
    /// Create an empty document with no limits.
    pub fn new() -> Self {
        Document {
            entries: Vec::new(),
            limits: Limits::default(),
        }
    }

    /// Create an empty document that enforces the given [`Limits`].
    pub fn with_limits(limits: Limits) -> Self {
        Document {
            entries: Vec::new(),
            limits,
        }
    }

    /// Parse a fresh document from a string in one step.
    pub fn parse(data: &str) -> Result<Self> {
        let mut doc = Document::new();
        doc.load(data)?;
        Ok(doc)
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the document holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Reset the document to empty (keeps the configured limits).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Iterate over all entries in insertion order.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    // ----------------------------------------------------------------- //
    // Loading
    // ----------------------------------------------------------------- //

    /// Parse an in-memory INI buffer, appending its entries to this document.
    ///
    /// Call on a fresh document, or again to merge another buffer in. Returns
    /// the first [`Error`] encountered.
    pub fn load(&mut self, data: &str) -> Result<()> {
        let mut section = String::new();
        for raw in split_lines(data) {
            self.handle_line(raw, &mut section)?;
        }
        Ok(())
    }

    fn handle_line(&mut self, raw: &str, section: &mut String) -> Result<()> {
        let trimmed = trim(raw);

        // Blank line or full-line comment.
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            return Ok(());
        }

        // Section header.
        if let Some(rest) = trimmed.strip_prefix('[') {
            let close = rest.find(']').ok_or(Error::Syntax)?;
            let name = trim(&rest[..close]);
            check_len(name.len(), self.limits.max_section)?;
            section.clear();
            section.push_str(name);
            return Ok(());
        }

        // key = value  /  key : value
        let sep = trimmed
            .find(['=', ':'])
            .ok_or(Error::Syntax)?;
        let key = trim(&trimmed[..sep]);
        if key.is_empty() {
            return Err(Error::Syntax);
        }
        let value = clean_value(&trimmed[sep + 1..]);

        // `section` is a separate local buffer, not part of `self`, so it can
        // be borrowed while `self` is mutated — no per-line clone needed.
        self.set_checked(section, key, &value)
    }

    // ----------------------------------------------------------------- //
    // Getters
    // ----------------------------------------------------------------- //

    fn find(&self, section: &str, key: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.section == section && e.key == key)
    }

    fn find_mut(&mut self, section: &str, key: &str) -> Option<&mut Entry> {
        self.entries
            .iter_mut()
            .find(|e| e.section == section && e.key == key)
    }

    /// Whether `section` / `key` exists. Use `""` for the global section.
    pub fn has(&self, section: &str, key: &str) -> bool {
        self.find(section, key).is_some()
    }

    /// Get the raw string value, or `fallback` if absent.
    pub fn get_string<'a>(&'a self, section: &str, key: &str, fallback: &'a str) -> &'a str {
        self.find(section, key)
            .map(|e| e.value.as_str())
            .unwrap_or(fallback)
    }

    /// Get the value as an `i64` (decimal, or `0x`-prefixed hex).
    ///
    /// Returns `fallback` if the key is absent or the value is unconvertible.
    pub fn get_int(&self, section: &str, key: &str, fallback: i64) -> i64 {
        match self.find(section, key) {
            Some(e) => parse_int(&e.value).unwrap_or(fallback),
            None => fallback,
        }
    }

    /// Get the value as an `f64` (supports an `e` exponent).
    pub fn get_double(&self, section: &str, key: &str, fallback: f64) -> f64 {
        match self.find(section, key) {
            Some(e) => e.value.trim().parse::<f64>().unwrap_or(fallback),
            None => fallback,
        }
    }

    /// Get the value as a boolean.
    ///
    /// Recognises (case-insensitive) `true/false`, `yes/no`, `on/off`, `1/0`.
    /// Returns `fallback` if absent or unrecognised.
    pub fn get_bool(&self, section: &str, key: &str, fallback: bool) -> bool {
        match self.find(section, key) {
            Some(e) => parse_bool(&e.value).unwrap_or(fallback),
            None => fallback,
        }
    }

    // ----------------------------------------------------------------- //
    // Setters
    // ----------------------------------------------------------------- //

    /// Set `key` in `section` to a string value (add or overwrite).
    ///
    /// Returns [`Error::Full`] / [`Error::TooLong`] only when limits are
    /// configured (see [`Document::with_limits`]); the default document never
    /// fails here.
    pub fn set(&mut self, section: &str, key: &str, value: &str) -> Result<()> {
        self.set_checked(section, key, value)
    }

    fn set_checked(&mut self, section: &str, key: &str, value: &str) -> Result<()> {
        check_len(value.len(), self.limits.max_value)?;

        if let Some(e) = self.find_mut(section, key) {
            e.value.clear();
            e.value.push_str(value);
            return Ok(());
        }

        check_len(section.len(), self.limits.max_section)?;
        check_len(key.len(), self.limits.max_key)?;
        if let Some(max) = self.limits.max_entries {
            if self.entries.len() >= max {
                return Err(Error::Full);
            }
        }
        self.entries.push(Entry {
            section: section.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        });
        Ok(())
    }

    /// Set `key` to an integer value (formatted as decimal).
    pub fn set_int(&mut self, section: &str, key: &str, value: i64) -> Result<()> {
        self.set(section, key, &value.to_string())
    }

    /// Set `key` to `"true"` or `"false"`.
    pub fn set_bool(&mut self, section: &str, key: &str, value: bool) -> Result<()> {
        self.set(section, key, if value { "true" } else { "false" })
    }

    /// Remove `key` from `section`. Returns [`Error::NotFound`] if absent.
    pub fn remove(&mut self, section: &str, key: &str) -> Result<()> {
        if let Some(idx) = self
            .entries
            .iter()
            .position(|e| e.section == section && e.key == key)
        {
            self.entries.remove(idx);
            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }
}

// ----------------------------------------------------------------------- //
// Free-standing parsing helpers.
// ----------------------------------------------------------------------- //

fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{0c}' | '\u{0b}')
}

/// Trim leading/trailing INI whitespace (space, tab, form-feed, vertical-tab).
fn trim(s: &str) -> &str {
    s.trim_matches(is_space)
}

fn check_len(len: usize, limit: Option<usize>) -> Result<()> {
    match limit {
        Some(max) if len > max => Err(Error::TooLong),
        _ => Ok(()),
    }
}

/// A lazy iterator over the lines of `data`, splitting on LF, CRLF and lone-CR
/// without allocating.
struct Lines<'a> {
    data: &'a str,
    pos: usize,
}

fn split_lines(data: &str) -> Lines<'_> {
    Lines { data, pos: 0 }
}

impl<'a> Iterator for Lines<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let bytes = self.data.as_bytes();
        if self.pos >= bytes.len() {
            return None;
        }
        let start = self.pos;
        let mut i = start;
        while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
            i += 1;
        }
        let line = &self.data[start..i];
        // Advance past the terminator (handles LF, CRLF and lone-CR).
        if i < bytes.len() {
            if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
        }
        self.pos = i;
        Some(line)
    }
}

/// Strip quotes / inline comments from a raw value, returning a clean owned
/// string.
fn clean_value(raw: &str) -> String {
    let src = raw.trim_start_matches(is_space);
    let mut chars = src.chars().peekable();

    match chars.peek() {
        Some(&q @ '"') | Some(&q @ '\'') => {
            chars.next(); // consume opening quote
            let mut out = String::new();
            while let Some(c) = chars.next() {
                if c == q {
                    break;
                }
                if q == '"' && c == '\\' {
                    match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some('\\') => out.push('\\'),
                        Some('"') => out.push('"'),
                        Some('\'') => out.push('\''),
                        Some(other) => out.push(other),
                        None => break,
                    }
                } else {
                    out.push(c);
                }
            }
            out
        }
        _ => {
            // Unquoted: cut at an inline `;`/`#` that follows whitespace (or
            // starts the value), then trim trailing whitespace.
            let mut out = String::new();
            let mut prev_space = true; // beginning counts as preceding space
            for c in src.chars() {
                if (c == ';' || c == '#') && prev_space {
                    break;
                }
                prev_space = is_space(c);
                out.push(c);
            }
            while out.ends_with(is_space) {
                out.pop();
            }
            out
        }
    }
}

/// Parse a decimal or `0x`-hex integer with optional sign, ignoring trailing
/// junk; returns `None` if no digits were consumed.
fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim_start_matches(is_space);
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }

    let mut val: i64 = 0;
    let mut any = false;
    if i + 1 < bytes.len() && bytes[i] == b'0' && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
        i += 2;
        while i < bytes.len() {
            let d = match bytes[i] {
                b'0'..=b'9' => (bytes[i] - b'0') as i64,
                b'a'..=b'f' => (bytes[i] - b'a' + 10) as i64,
                b'A'..=b'F' => (bytes[i] - b'A' + 10) as i64,
                _ => break,
            };
            val = val.wrapping_mul(16).wrapping_add(d);
            any = true;
            i += 1;
        }
    } else {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
            any = true;
            i += 1;
        }
    }

    if !any {
        return None;
    }
    Some(if neg { -val } else { val })
}

fn parse_bool(s: &str) -> Option<bool> {
    let t = s.trim();
    if t.eq_ignore_ascii_case("true")
        || t.eq_ignore_ascii_case("yes")
        || t.eq_ignore_ascii_case("on")
        || t == "1"
    {
        Some(true)
    } else if t.eq_ignore_ascii_case("false")
        || t.eq_ignore_ascii_case("no")
        || t.eq_ignore_ascii_case("off")
        || t == "0"
    {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        "; global config\n",
        "app_name = Rustiniparser\n",
        "[network]\n",
        "host = \"example.com\"   ; inline comment\n",
        "port = 8080\n",
        "ratio = 0.25\n",
        "mask = 0xFF\n",
        "[server]\n",
        "enabled = yes\n",
        "password = 'p@ss #word'\n",
    );

    #[test]
    fn load_and_get() {
        let doc = Document::parse(SAMPLE).unwrap();
        assert_eq!(doc.get_string("", "app_name", "?"), "Rustiniparser");
        assert_eq!(doc.get_string("network", "host", "?"), "example.com");
        assert_eq!(doc.get_string("server", "password", "?"), "p@ss #word");
        assert_eq!(doc.get_int("network", "port", -1), 8080);
        assert_eq!(doc.get_int("network", "mask", -1), 255);
        let r = doc.get_double("network", "ratio", 0.0);
        assert!(r > 0.249 && r < 0.251);
        assert!(doc.get_bool("server", "enabled", false));
        assert_eq!(doc.get_int("network", "missing", 42), 42);
        assert_eq!(doc.get_string("nope", "x", "def"), "def");
        assert!(doc.has("network", "host"));
        assert!(!doc.has("network", "nope"));
    }

    #[test]
    fn modify() {
        let mut doc = Document::parse(SAMPLE).unwrap();
        let before = doc.len();
        doc.set_int("network", "port", 9090).unwrap();
        assert_eq!(doc.get_int("network", "port", 0), 9090);
        assert_eq!(doc.len(), before);
        doc.set_bool("server", "enabled", false).unwrap();
        assert!(!doc.get_bool("server", "enabled", true));
    }

    #[test]
    fn add() {
        let mut doc = Document::parse(SAMPLE).unwrap();
        let before = doc.len();
        doc.set("network", "gateway", "10.0.0.1").unwrap();
        assert_eq!(doc.get_string("network", "gateway", "?"), "10.0.0.1");
        doc.set_int("limits", "max", 100).unwrap();
        assert_eq!(doc.get_int("limits", "max", 0), 100);
        assert_eq!(doc.len(), before + 2);
    }

    #[test]
    fn remove() {
        let mut doc = Document::parse(SAMPLE).unwrap();
        assert_eq!(doc.remove("network", "port"), Ok(()));
        assert!(!doc.has("network", "port"));
        assert!(doc.has("network", "host"));
        assert_eq!(doc.remove("network", "port"), Err(Error::NotFound));
    }

    #[test]
    fn errors() {
        assert_eq!(Document::parse("[unterminated\n").err(), Some(Error::Syntax));
        assert_eq!(Document::parse("novalue\n").err(), Some(Error::Syntax));
    }

    #[test]
    fn limits_enforced() {
        let mut doc = Document::with_limits(Limits {
            max_entries: Some(1),
            ..Limits::default()
        });
        doc.set("", "a", "1").unwrap();
        assert_eq!(doc.set("", "b", "2"), Err(Error::Full));
        // overwriting the existing key still works at capacity
        assert_eq!(doc.set("", "a", "2"), Ok(()));
    }

    #[test]
    fn line_endings() {
        let doc = Document::parse("a=1\r\nb=2\rc=3").unwrap();
        assert_eq!(doc.get_int("", "a", 0), 1);
        assert_eq!(doc.get_int("", "b", 0), 2);
        assert_eq!(doc.get_int("", "c", 0), 3);
    }
}
