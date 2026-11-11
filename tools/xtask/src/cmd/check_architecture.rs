//! `xtask check-architecture` — architecture boundary checks (card 1.E1).
//!
//! Implements doc 01 §5 (the CI gate **G5** of doc 10 §4.6) against the
//! declarative manifest `boundaries.toml` at the workspace root (the doc's
//! proposed in-tree path is overridden by the card: the manifest sits next to
//! the workspace manifest it describes; Lead-ratified deviation).
//!
//! # Checks (doc 01 §5.2, adapted per the 1.E1 card)
//!
//! | # | Name               | What it proves |
//! |---|--------------------|----------------|
//! | 1 | `edges`            | every actual dependency is allowed by `boundaries.toml` (allowlist semantics: unknown crate = error, missing allowed-edge = fine), internal edges point downward in the layer stack or are whitelisted same-layer pairs (§5.2 steps 2-4, ARCH-2) |
//! | 2 | `cycles`           | no dependency cycles over normal **and** dev edges (§5.2 step 5, ARCH-8) — cargo rejects normal-edge cycles itself, dev-edge cycles only die here |
//! | 3 | `unsafe-lints`     | every member manifest carries `[lints] workspace = true` and the root denies `unsafe_code` (§5.2 step 7 via doc 10 §3.1/§3.2 — the workspace lint makes `unsafe` uncompilable; this is the CI-visible structural proof, per the card) |
//! | 4 | `static-mut`       | no `static mut` in any member `src/` tree (§5.2 step 6, ARCH-8) |
//! | 5 | `testkit-direction`| `testkit` appears in no `[dependencies]`/`[build-dependencies]` table — dev-only (MP §3.7, ARCH-7) |
//!
//! # Design decisions (documented per the card)
//!
//! * **Manifests are read directly, not via `cargo metadata`.** xtask is
//!   std-only (C5: no `toml`/`serde_json` crates), so both `boundaries.toml`
//!   and every `Cargo.toml` go through the hand-rolled TOML-subset reader
//!   below. Direct reads also keep the check hermetic: no cargo subprocess,
//!   no registry access, and deliberately-broken fixture workspaces (dev-edge
//!   cycles, missing lints) are checkable even though cargo could not build
//!   them. The workspace member set is discovered from `[workspace] members`
//!   globs exactly like cargo does.
//! * **`static mut` scan posture (honest):** a comment/string-aware text scan
//!   of source files. False negatives: `static mut` synthesized by external
//!   proc-macros is invisible (macro-generated code is not in our sources);
//!   a `'"'` char literal can temporarily confuse the string tracker.
//!   False positives: none known — comments, doc comments, string literals
//!   and raw strings are stripped before matching. The `deny(unsafe_code)`
//!   lint already makes `static mut` *unusable* workspace-wide; this scan
//!   catches the declaration itself (belt-and-braces, doc 01 §5.2 step 6).
//! * **Layer ordering applies to normal `[dependencies]` edges only.**
//!   Dev edges are exempt: doc 01's own design has L0 `media-types`
//!   dev-depend on the layer-less dev crate `testkit` (§1.4 layer table).
//! * **Reporting ownership:** a normal edge into a dev-only crate (`testkit`)
//!   is reported ONLY by check 5 (ARCH-7), and self-edges only by check 2 —
//!   one root cause, one rule, one fix hint, no double reporting.
//! * **Parse errors are tool errors (exit 2)**, distinct from violations
//!   (exit 1): a manifest the checker cannot read is "the tool cannot
//!   verify", not "the tree is fine".
//!
//! # Report
//!
//! Human summary (gate.rs conventions: `=== LUMEN ... REPORT ===`, per-check
//! PASS/FAIL lines, `result:` verdict), a machine summary line
//! (`architecture: result=... <check>=<status> ...`), and the JSON artifact
//! `target/architecture-report.json` (doc 01 §5.2 step 8; schema
//! `architecture-report/1` `[PROPOSAL]` — doc 10 §7.2's `evidence/1`
//! envelope is command-shaped and does not fit static analysis; the Lead may
//! unify later). Exit 0 all-pass / 1 any violation / 2 xtask error.
//!
//! # Flags
//!
//! * `--root <dir>` — check the workspace at `<dir>` instead of walking up
//!   from the current directory. Exists for the fixture-based acceptance
//!   proof (`tools/xtask/tests/check_architecture.rs`); the real workspace
//!   needs no flag.
//! * `--json` — additionally print the JSON report to stdout.
//!
//! # Known v0 limitations (honest register)
//!
//! * No `deny_categories` table (doc 01 §5.1 sketch): v1 allowlists external
//!   crates by *name* per crate; cargo-deny (card 1.E5-R) owns license,
//!   source and advisory policy.
//! * No `allow_unsafe` / `unsafe` token scan (doc 01 §5.2 step 7's platform
//!   crates): no L5 crate exists yet; the lint-inheritance check plus the
//!   root `deny(unsafe_code)` is the v1 posture. Add when `platform-*`
//!   lands.
//! * The TOML subset rejects multi-line inline tables and dotted keys —
//!   loudly, with file:line, never silently. Multi-line ARRAYS parse
//!   (W25 CI-repair: member manifests ship them).
//! * Dependency renames (`foo = { package = "bar" }`) are keyed by the local
//!   name `foo`, matching what the manifest advertises.
//!
//! # Ownership
//!
//! Owned by card **1.E1**. The dispatch table (`main.rs`/`cmd/mod.rs`) is
//! frozen; only this module and `boundaries.toml` belong to this card.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::XtaskError;

// ---------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------

const USAGE: &str = "\
Usage: cargo xtask check-architecture [--root <dir>] [--json]

Checks the Lumen workspace against boundaries.toml (doc 01 §5; CI gate G5
per doc 10 §4.6):

  1  edges              dependency allowlist + layer ordering
                        (doc 01 §5.2 steps 2-4, ARCH-2)
  2  cycles             no cycles over normal AND dev edges
                        (doc 01 §5.2 step 5, ARCH-8)
  3  unsafe-lints       every member inherits [lints] workspace = true and
                        the root denies unsafe_code (doc 10 §3.1-3.2)
  4  static-mut         no `static mut` in member src/ trees
                        (doc 01 §5.2 step 6, ARCH-8)
  5  testkit-direction  testkit only ever in [dev-dependencies]
                        (MP §3.7, ARCH-7)

Reads the Cargo.toml manifests directly (no cargo subprocess, no network;
the xtask is std-only by C5) and writes the JSON artifact to
target/architecture-report.json (doc 01 §5.2 step 8).

  --root <dir>  check the workspace rooted at <dir> instead of walking up
                from the current directory (used by the fixture-based
                acceptance proof in tools/xtask/tests/check_architecture.rs)
  --json        additionally print the JSON report to stdout

Exit code: 0 all checks pass | 1 any violation | 2 xtask error.";

/// Where the JSON report is written, relative to the checked root.
const REPORT_REL: &str = "target/architecture-report.json";

/// The five checks, in report order (doc 01 §5.2 / the 1.E1 card).
const CHECK_NAMES: [&str; 5] = [
    "edges",
    "cycles",
    "unsafe-lints",
    "static-mut",
    "testkit-direction",
];

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Entry point for `cargo xtask check-architecture`.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut root_override: Option<PathBuf> = None;
    let mut json_to_stdout = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "--json" => json_to_stdout = true,
            "--root" => {
                i += 1;
                let Some(dir) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--root requires a directory argument\n{USAGE}"
                    )));
                };
                root_override = Some(PathBuf::from(dir));
            }
            other => {
                return Err(XtaskError::Usage(format!(
                    "unexpected argument `{other}`\n{USAGE}"
                )));
            }
        }
        i += 1;
    }

    let root = match root_override {
        Some(dir) => dir,
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| XtaskError::io("determining current directory", e))?;
            find_workspace_root(&cwd)?
        }
    };
    let root = root
        .canonicalize()
        .map_err(|e| XtaskError::io(format!("resolving root {}", root.display()), e))?;

    let workspace = load_workspace(&root)?;
    let boundaries = load_boundaries(&root)?;
    let checks = run_checks(&workspace, &boundaries, &root);
    let report = Report {
        root: root.display().to_string(),
        members: workspace.members.len(),
        declared: boundaries.crates.len(),
        checks,
    };
    let json = report.to_json();
    let report_path = write_json_report(&root, &json)?;
    report.print_human(&report_path);
    if json_to_stdout {
        println!("{json}");
    }
    if report.result().is_pass() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

// ---------------------------------------------------------------------------
// TOML subset reader (std-only; C5 — xtask takes no dependencies)
// ---------------------------------------------------------------------------

/// A parsed value in the checker's TOML subset.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TValue {
    Str(String),
    Int(i64),
    Bool(bool),
    StrArray(Vec<String>),
    /// A value shape the subset does not interpret (inline tables, floats,
    /// dates, arrays with non-string elements, ...). Stored raw so that keys
    /// we do not consume never fail the parse.
    Opaque(String),
}

impl TValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            TValue::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            TValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    fn as_int(&self) -> Option<i64> {
        match self {
            TValue::Int(i) => Some(*i),
            _ => None,
        }
    }
    fn as_str_array(&self) -> Option<&[String]> {
        match self {
            TValue::StrArray(v) => Some(v),
            _ => None,
        }
    }
}

/// One `[table.header]` (or `[[array.of.tables]]`) and its entries, in
/// source order. Header paths are kept flat; `[a.b]` becomes `["a","b"]`.
#[derive(Debug)]
struct Table {
    path: Vec<String>,
    entries: Vec<(String, TValue)>,
}

/// A parsed TOML-subset document.
#[derive(Debug)]
struct TomlDoc {
    tables: Vec<Table>,
}

/// Parse failure with the 1-based line where it was detected.
struct TomlError {
    line: usize,
    message: String,
}

impl TomlError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        TomlError {
            line,
            message: message.into(),
        }
    }
}

/// Parse `text` (the contents of a TOML file) into the subset.
///
/// Supported: `[a.b.c]` headers (bare and quoted segments), `[[array]]`
/// headers, `key = value` with string / integer / boolean / single- or
/// multi-line string-array / opaque values, `#` comments, trailing
/// comments. Rejected loudly: duplicate plain tables, duplicate keys,
/// dotted keys, multi-line inline tables, unterminated strings or
/// arrays, unsupported escapes. (Multi-line arrays became supported in
/// the W25 CI-repair pass — the codecs/web tree-shake lane landed them
/// in member manifests and the reader had to follow reality.)
fn parse_toml(text: &str) -> Result<TomlDoc, TomlError> {
    let chars: Vec<char> = {
        let mut c = text.chars().collect::<Vec<char>>();
        // A UTF-8 BOM would poison the first header/key match.
        if c.first() == Some(&'\u{feff}') {
            c.remove(0);
        }
        c
    };
    let mut p = Parser {
        chars,
        pos: 0,
        line: 1,
    };
    let mut doc = TomlDoc { tables: Vec::new() };
    let mut root = Table {
        path: Vec::new(),
        entries: Vec::new(),
    };
    let mut current: Option<usize> = None;
    loop {
        p.skip_blank()?;
        if p.eof() {
            break;
        }
        if p.peek() == Some('[') {
            p.bump();
            let array_of_tables = p.peek() == Some('[');
            if array_of_tables {
                p.bump();
            }
            let path = p.parse_header_path()?;
            if p.peek() != Some(']') {
                return Err(p.error("table header is missing its closing `]`"));
            }
            p.bump();
            if array_of_tables {
                if p.peek() != Some(']') {
                    return Err(p.error("array-of-tables header is missing its second `]`"));
                }
                p.bump();
            }
            p.finish_line()?;
            if path.is_empty() {
                return Err(p.error("empty table header"));
            }
            if !array_of_tables && doc.tables.iter().any(|t| t.path == path) {
                return Err(p.error(format!("duplicate table header `[{}]`", path.join("."))));
            }
            doc.tables.push(Table {
                path,
                entries: Vec::new(),
            });
            current = Some(doc.tables.len() - 1);
        } else {
            let key = p.parse_key()?;
            p.skip_inline_ws();
            if p.peek() == Some('.') {
                return Err(p.error(
                    "dotted keys are not supported by this reader — use [table.sub] headers",
                ));
            }
            if p.peek() != Some('=') {
                return Err(p.error(format!("expected `=` after key `{key}`")));
            }
            p.bump();
            p.skip_inline_ws();
            let value = p.parse_value()?;
            p.finish_line()?;
            let table = match current {
                Some(idx) => match doc.tables.get_mut(idx) {
                    Some(t) => t,
                    None => return Err(p.error("internal parser error: no current table")),
                },
                None => &mut root,
            };
            if table.entries.iter().any(|(k, _)| *k == key) {
                return Err(p.error(format!(
                    "duplicate key `{key}` in [{}]",
                    table.path.join(".")
                )));
            }
            table.entries.push((key, value));
        }
    }
    if !root.entries.is_empty() {
        doc.tables.insert(0, root);
    }
    Ok(doc)
}

/// Character-level cursor with line tracking.
struct Parser {
    chars: Vec<char>,
    pos: usize,
    line: usize,
}

impl Parser {
    fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Advance one character, counting lines on `\n`.
    fn bump(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
            }
        }
    }

    fn error(&self, message: impl Into<String>) -> TomlError {
        TomlError::new(self.line, message)
    }

    /// Skip spaces, tabs, newlines and whole-line comments.
    fn skip_blank(&mut self) -> Result<(), TomlError> {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => self.bump(),
                Some('#') => {
                    while !matches!(self.peek(), None | Some('\n')) {
                        self.bump();
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// Skip spaces and tabs only.
    fn skip_inline_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.bump();
        }
    }

    /// After a value or header: only inline whitespace and an optional
    /// trailing comment may remain on the line.
    fn finish_line(&mut self) -> Result<(), TomlError> {
        self.skip_inline_ws();
        if self.peek() == Some('#') {
            while !matches!(self.peek(), None | Some('\n')) {
                self.bump();
            }
        }
        match self.peek() {
            None | Some('\n') => Ok(()),
            Some(c) => Err(self.error(format!("unexpected `{c}` after value"))),
        }
    }

    /// Parse a table-header path: `a.b.'c-d'."e"` — segments of bare keys
    /// or quoted strings, dot-separated.
    fn parse_header_path(&mut self) -> Result<Vec<String>, TomlError> {
        let mut path = Vec::new();
        loop {
            self.skip_inline_ws();
            let segment = match self.peek() {
                Some('"') => self.parse_basic_string()?,
                Some('\'') => self.parse_literal_string()?,
                _ => self.parse_bare_key()?,
            };
            if segment.is_empty() {
                return Err(self.error("empty segment in table header"));
            }
            path.push(segment);
            self.skip_inline_ws();
            if self.peek() == Some('.') {
                self.bump();
                continue;
            }
            return Ok(path);
        }
    }

    /// Parse a key: a quoted string or a non-empty bare key.
    fn parse_key(&mut self) -> Result<String, TomlError> {
        match self.peek() {
            Some('"') => self.parse_basic_string(),
            Some('\'') => self.parse_literal_string(),
            _ => {
                let key = self.parse_bare_key()?;
                if key.is_empty() {
                    return Err(self.error("expected a key"));
                }
                Ok(key)
            }
        }
    }

    /// Consume a run of TOML bare-key characters `[A-Za-z0-9_-]`.
    fn parse_bare_key(&mut self) -> Result<String, TomlError> {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                out.push(c);
                self.bump();
            } else {
                break;
            }
        }
        Ok(out)
    }

    /// Parse a `"..."` basic string with the common escapes.
    fn parse_basic_string(&mut self) -> Result<String, TomlError> {
        self.bump(); // opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    return Err(
                        self.error("unterminated string (multi-line strings are unsupported)")
                    );
                }
                Some('"') => {
                    self.bump();
                    return Ok(out);
                }
                Some('\\') => {
                    self.bump();
                    let escaped = match self.peek() {
                        Some('"') => '"',
                        Some('\\') => '\\',
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('r') => '\r',
                        Some(other) => {
                            return Err(
                                self.error(format!("unsupported escape `\\{other}` in string"))
                            );
                        }
                        None => return Err(self.error("unterminated escape in string")),
                    };
                    out.push(escaped);
                    self.bump();
                }
                Some(c) => {
                    out.push(c);
                    self.bump();
                }
            }
        }
    }

    /// Parse a `'...'` literal string (no escapes).
    fn parse_literal_string(&mut self) -> Result<String, TomlError> {
        self.bump(); // opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => return Err(self.error("unterminated literal string")),
                Some('\'') => {
                    self.bump();
                    return Ok(out);
                }
                Some(c) => {
                    out.push(c);
                    self.bump();
                }
            }
        }
    }

    /// Parse a value on the current line.
    fn parse_value(&mut self) -> Result<TValue, TomlError> {
        match self.peek() {
            None => Err(self.error("expected a value")),
            Some('"') => Ok(TValue::Str(self.parse_basic_string()?)),
            Some('\'') => Ok(TValue::Str(self.parse_literal_string()?)),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_inline_table(),
            Some('t') | Some('f') => self.parse_bool(),
            Some(c) if c == '-' || c == '+' || c.is_ascii_digit() => self.parse_number(),
            Some(_) => self.parse_opaque_token(),
        }
    }

    /// Parse `true` / `false` (a following identifier character means the
    /// value is an unknown token — keep it opaque instead of failing).
    fn parse_bool(&mut self) -> Result<TValue, TomlError> {
        let word = if self.starts_with_word("true") {
            "true"
        } else if self.starts_with_word("false") {
            "false"
        } else {
            return self.parse_opaque_token();
        };
        for _ in 0..word.len() {
            self.bump();
        }
        Ok(TValue::Bool(word == "true"))
    }

    /// True if `word` starts at the cursor and is not followed by another
    /// identifier character.
    fn starts_with_word(&self, word: &str) -> bool {
        let mut idx = self.pos;
        for expected in word.chars() {
            match self.chars.get(idx) {
                Some(c) if *c == expected => idx += 1,
                _ => return false,
            }
        }
        !self
            .chars
            .get(idx)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
    }

    /// Parse an integer, or keep an unparseable numeric-looking token
    /// (floats, dates) opaque.
    fn parse_number(&mut self) -> Result<TValue, TomlError> {
        let mut token = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '+' | '_' | '.' | ':') {
                token.push(c);
                self.bump();
            } else {
                break;
            }
        }
        let cleaned = token.replace('_', "");
        match cleaned.parse::<i64>() {
            Ok(n) => Ok(TValue::Int(n)),
            Err(_) => Ok(TValue::Opaque(token)),
        }
    }

    /// Parse a string array. Single- or multi-line: newlines and whole-line
    /// comments between elements are skipped (TOML 1.0 allows them). Any
    /// non-string element shape degrades the whole array to an opaque value.
    fn parse_array(&mut self) -> Result<TValue, TomlError> {
        let start = self.pos;
        self.bump(); // '['
        let mut items: Vec<String> = Vec::new();
        loop {
            self.skip_blank()?; // spans newlines + comments inside the array
            match self.peek() {
                None => {
                    return Err(self.error("unterminated array"));
                }
                Some(']') => {
                    self.bump();
                    return Ok(TValue::StrArray(items));
                }
                Some(',') => {
                    self.bump();
                }
                Some('"') => {
                    let s = self.parse_basic_string()?;
                    items.push(s);
                }
                Some(_) => {
                    // Non-string element: rewind and take the array opaque.
                    self.pos = start;
                    return self.parse_opaque_array();
                }
            }
        }
    }

    /// Consume an array of unknown element shapes as an opaque value.
    /// Newlines inside the brackets are preserved in the opaque text
    /// (multi-line arrays, W25); EOF before the closing bracket errors.
    fn parse_opaque_array(&mut self) -> Result<TValue, TomlError> {
        self.bump(); // '['
        let mut depth = 1usize;
        let mut out = String::from("[");
        while depth > 0 {
            match self.peek() {
                None => {
                    return Err(self.error("unterminated array"));
                }
                Some('\n') => {
                    out.push('\n');
                    self.bump();
                }
                Some('[') => {
                    depth += 1;
                    out.push('[');
                    self.bump();
                }
                Some(']') => {
                    depth -= 1;
                    out.push(']');
                    self.bump();
                }
                Some('"') => {
                    let s = self.parse_basic_string()?;
                    out.push('"');
                    out.push_str(&s);
                    out.push('"');
                }
                Some(c) => {
                    out.push(c);
                    self.bump();
                }
            }
        }
        Ok(TValue::Opaque(out))
    }

    /// Consume a single-line `{ ... }` inline table as an opaque value
    /// (dependency entries only need their KEY, not their value).
    fn parse_inline_table(&mut self) -> Result<TValue, TomlError> {
        self.bump(); // '{'
        let mut depth = 1usize;
        let mut out = String::from("{");
        while depth > 0 {
            match self.peek() {
                None | Some('\n') => {
                    return Err(
                        self.error("multi-line inline tables are not supported by this reader")
                    );
                }
                Some('{') => {
                    depth += 1;
                    out.push('{');
                    self.bump();
                }
                Some('}') => {
                    depth -= 1;
                    out.push('}');
                    self.bump();
                }
                Some('"') => {
                    let s = self.parse_basic_string()?;
                    out.push('"');
                    out.push_str(&s);
                    out.push('"');
                }
                Some(c) => {
                    out.push(c);
                    self.bump();
                }
            }
        }
        Ok(TValue::Opaque(out))
    }

    /// Consume an unknown single-token value (barewords, ...) to the end of
    /// the line, kept opaque.
    fn parse_opaque_token(&mut self) -> Result<TValue, TomlError> {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if matches!(c, '\n' | '#' | ',' | ']' | '}') {
                break;
            }
            out.push(c);
            self.bump();
        }
        Ok(TValue::Opaque(out.trim_end().to_string()))
    }
}

impl TomlDoc {
    /// All entries across tables whose path is exactly `path`.
    fn entries_at(&self, path: &[&str]) -> Vec<&(String, TValue)> {
        self.tables
            .iter()
            .filter(|t| table_path_matches(&t.path, path))
            .flat_map(|t| t.entries.iter())
            .collect()
    }

    /// First value for `key` under the table at `path`.
    fn get(&self, path: &[&str], key: &str) -> Option<&TValue> {
        self.entries_at(path)
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Whether a table with exactly this path exists (test helper).
    #[cfg(test)]
    fn has_table(&self, path: &[&str]) -> bool {
        self.tables
            .iter()
            .any(|t| table_path_matches(&t.path, path))
    }
}

/// Compare a parsed table path against a query path of `&str` segments.
fn table_path_matches(parsed: &[String], query: &[&str]) -> bool {
    parsed.len() == query.len() && parsed.iter().zip(query).all(|(a, b)| a == b)
}

// ---------------------------------------------------------------------------
// Workspace + boundaries models
// ---------------------------------------------------------------------------

/// One dependency-table entry of a member manifest.
#[derive(Debug, Clone)]
struct DepEntry {
    /// Dependency name as written (the table key).
    name: String,
    /// Which table it came from: "dependencies", "dev-dependencies" or
    /// "build-dependencies" (the header's last segment).
    table: String,
}

impl DepEntry {
    fn is_dev(&self) -> bool {
        self.table == "dev-dependencies"
    }
}

/// One workspace member, as the checker sees it.
#[derive(Debug)]
struct Member {
    /// `[package] name`.
    name: String,
    /// Manifest path relative to the checked root, forward slashes.
    manifest_rel: String,
    /// Absolute member directory.
    dir: PathBuf,
    /// All dependency entries (normal + build + dev), in manifest order.
    deps: Vec<DepEntry>,
    /// `[lints] workspace = true` present.
    lints_inherits: bool,
    /// A `[lints.<x>]` override sub-table is present.
    lints_override: bool,
}

/// The checked workspace: members + root lint posture.
#[derive(Debug)]
struct Workspace {
    /// Sorted by name (deterministic reports).
    members: Vec<Member>,
    /// Root `[workspace.lints.rust] unsafe_code = "deny"` present.
    root_denies_unsafe: bool,
}

/// One `[crates.<name>]` rule from `boundaries.toml`.
#[derive(Debug)]
struct CrateRule {
    /// Doc 01 §1.4 layer, when the crate is in the layer stack.
    layer: Option<i64>,
    internal_deps: Vec<String>,
    dev_deps: Vec<String>,
    external_allowlist: Vec<String>,
    external_dev_allowlist: Vec<String>,
}

/// The parsed `boundaries.toml`.
#[derive(Debug)]
struct Boundaries {
    crates: BTreeMap<String, CrateRule>,
    /// Whitelisted same-layer pairs ("from", "to"), doc 01 §1.4.
    same_layer_pairs: BTreeSet<(String, String)>,
    /// Crates allowed only in [dev-dependencies] (MP §3.7 / ARCH-7).
    dev_only: BTreeSet<String>,
    /// Members exempt from the `static mut` scan, doc 01 §5.2 step 6.
    static_mut_allow: BTreeSet<String>,
}

// ---------------------------------------------------------------------------
// Workspace root discovery (mirrors gate.rs — its helpers are private)
// ---------------------------------------------------------------------------

/// Walk up from `start` to the nearest `Cargo.toml` declaring `[workspace]`.
fn find_workspace_root(start: &Path) -> Result<PathBuf, XtaskError> {
    let mut dir: Option<&Path> = Some(start);
    while let Some(d) = dir {
        let manifest = d.join("Cargo.toml");
        if manifest.is_file() && manifest_declares_workspace(&manifest) {
            return Ok(d.to_path_buf());
        }
        dir = d.parent();
    }
    Err(XtaskError::internal(format!(
        "no Cargo.toml with [workspace] found in {} or any parent — \
         run `cargo xtask check-architecture` from inside the Lumen workspace",
        start.display()
    )))
}

/// True if the manifest has a `[workspace]` section (line-start match;
/// `[workspace.lints]` coexists with it in the root manifest, while member
/// manifests only carry `[lints] workspace = true`).
fn manifest_declares_workspace(path: &Path) -> bool {
    match std::fs::read_to_string(path) {
        Ok(contents) => contents
            .lines()
            .any(|line| line.trim_start().starts_with("[workspace]")),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Loading: workspace members
// ---------------------------------------------------------------------------

/// Read the root manifest and every member manifest.
fn load_workspace(root: &Path) -> Result<Workspace, XtaskError> {
    let root_manifest_path = root.join("Cargo.toml");
    let root_text = std::fs::read_to_string(&root_manifest_path)
        .map_err(|e| XtaskError::io("reading the workspace root manifest", e))?;
    let root_doc = parse_manifest(&root_manifest_path, &root_text, "the root manifest")?;

    let members_value = root_doc
        .get(&["workspace"], "members")
        .ok_or_else(|| {
            XtaskError::internal(format!(
                "{} declares no [workspace] members — not a workspace root",
                root_manifest_path.display()
            ))
        })?
        .clone();
    let member_patterns = members_value.as_str_array().ok_or_else(|| {
        XtaskError::internal(
            "[workspace] members must be an array of path strings in the root manifest".to_string(),
        )
    })?;

    let exclude_patterns: Vec<String> = match root_doc.get(&["workspace"], "exclude") {
        None => Vec::new(),
        Some(v) => match v.as_str_array() {
            Some(patterns) => patterns.to_vec(),
            None => {
                return Err(XtaskError::internal(
                    "[workspace] exclude must be an array of path strings in the root manifest"
                        .to_string(),
                ));
            }
        },
    };

    let member_dirs = expand_member_patterns(root, member_patterns, &exclude_patterns)?;

    let mut members: Vec<Member> = Vec::new();
    for dir in member_dirs {
        let manifest_path = dir.join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest_path).map_err(|e| {
            XtaskError::io(
                format!("reading member manifest {}", manifest_path.display()),
                e,
            )
        })?;
        let doc = parse_manifest(&manifest_path, &text, "a member manifest")?;

        let name = doc
            .get(&["package"], "name")
            .and_then(TValue::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                XtaskError::internal(format!(
                    "{} has no [package] name — every workspace member needs one",
                    manifest_path.display()
                ))
            })?;

        let mut deps: Vec<DepEntry> = Vec::new();
        for table in &doc.tables {
            let Some(first) = table.path.first() else {
                continue; // root-level keys carry no dependencies
            };
            if first == "workspace" {
                // Workspace-level tables ([workspace.lints...] and any
                // [workspace.dependencies] inheritance declarations) are not
                // this member's own dependency edges.
                continue;
            }
            let Some(last) = table.path.last() else {
                continue;
            };
            let table_kind = match last.as_str() {
                "dependencies" | "build-dependencies" | "dev-dependencies" => last.as_str(),
                _ => continue,
            };
            for (key, _) in &table.entries {
                if !deps.iter().any(|d| d.name == *key && d.table == table_kind) {
                    deps.push(DepEntry {
                        name: key.clone(),
                        table: table_kind.to_string(),
                    });
                }
            }
        }

        let lints_inherits = root_bool(&doc, &["lints"], "workspace");
        let lints_override = doc
            .tables
            .iter()
            .any(|t| t.path.first().is_some_and(|s| s == "lints") && t.path.len() > 1);

        members.push(Member {
            manifest_rel: rel_path(root, &manifest_path),
            dir,
            deps,
            lints_inherits,
            lints_override,
            name,
        });
    }

    // Duplicate package names would confuse every check that keys by name.
    for pair in members.windows(2) {
        if let [a, b] = pair
            && a.name == b.name
        {
            return Err(XtaskError::internal(format!(
                "duplicate package name `{}` ({} and {})",
                a.name, a.manifest_rel, b.manifest_rel
            )));
        }
    }

    let root_denies_unsafe = root_doc.get(&["workspace", "lints", "rust"], "unsafe_code")
        == Some(&TValue::Str("deny".to_string()));

    Ok(Workspace {
        members,
        root_denies_unsafe,
    })
}

/// Parse manifest text, converting parse failures into a contextual
/// `XtaskError::Internal` that names the file and 1-based line.
fn parse_manifest(path: &Path, text: &str, what: &str) -> Result<TomlDoc, XtaskError> {
    parse_toml(text).map_err(|e| {
        XtaskError::internal(format!(
            "parsing {} ({}): line {}: {}",
            path.display(),
            what,
            e.line,
            e.message
        ))
    })
}

/// Read an optional boolean under `[path] key` (root manifest helper).
fn root_bool(doc: &TomlDoc, path: &[&str], key: &str) -> bool {
    doc.get(path, key).and_then(TValue::as_bool) == Some(true)
}

/// Expand `[workspace] members` patterns (and subtract `exclude`) into the
/// sorted set of member directories, each verified to carry a Cargo.toml.
///
/// Supported pattern shape: literal paths, or a single `*` in the LAST path
/// component (cargo's `crates/*` convention). Anything fancier fails loudly
/// so the checker never silently misses a member.
fn expand_member_patterns(
    root: &Path,
    patterns: &[String],
    exclude: &[String],
) -> Result<Vec<PathBuf>, XtaskError> {
    let mut excluded: BTreeSet<PathBuf> = BTreeSet::new();
    for pat in exclude {
        if pat.contains('*') {
            return Err(XtaskError::internal(format!(
                "glob patterns in [workspace] exclude are not supported (`{pat}`) — \
                 use literal paths"
            )));
        }
        let p = root.join(pat.trim_end_matches('/'));
        if p.is_dir() {
            excluded.insert(p);
        }
    }

    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for pat in patterns {
        match pat.split_once('*') {
            None => {
                dirs.insert(root.join(pat.trim_end_matches('/')));
            }
            Some((before, after)) => {
                if before.contains('*') || after.contains('*') {
                    return Err(XtaskError::internal(format!(
                        "member pattern `{pat}`: only one `*` is supported"
                    )));
                }
                if after.contains('/') {
                    return Err(XtaskError::internal(format!(
                        "member pattern `{pat}`: the `*` must be in the last path component"
                    )));
                }
                let (parent_str, name_prefix) = match before.rsplit_once('/') {
                    Some((parent, prefix)) => (parent, prefix),
                    None => ("", before),
                };
                let parent = if parent_str.is_empty() {
                    root.to_path_buf()
                } else {
                    root.join(parent_str)
                };
                let entries = std::fs::read_dir(&parent).map_err(|e| {
                    XtaskError::io(
                        format!(
                            "expanding member pattern `{pat}` under {}",
                            parent.display()
                        ),
                        e,
                    )
                })?;
                for entry in entries {
                    let entry = entry.map_err(|e| {
                        XtaskError::io(
                            format!(
                                "expanding member pattern `{pat}` under {}",
                                parent.display()
                            ),
                            e,
                        )
                    })?;
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let fname = entry.file_name().to_string_lossy().into_owned();
                    if fname.starts_with(name_prefix)
                        && fname.ends_with(after)
                        && fname.len() >= name_prefix.len() + after.len()
                    {
                        dirs.insert(entry.path());
                    }
                }
            }
        }
    }

    let mut out: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        if excluded.contains(&dir) {
            continue;
        }
        if !dir.join("Cargo.toml").is_file() {
            return Err(XtaskError::internal(format!(
                "workspace member {} has no Cargo.toml",
                rel_path(root, &dir)
            )));
        }
        out.push(dir);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Loading: boundaries.toml
// ---------------------------------------------------------------------------

/// Parse `boundaries.toml` from the checked root.
fn load_boundaries(root: &Path) -> Result<Boundaries, XtaskError> {
    let path = root.join("boundaries.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| XtaskError::io("reading boundaries.toml (doc 01 §5.1)", e))?;
    let doc = parse_manifest(&path, &text, "boundaries.toml (doc 01 §5.1)")?;

    match doc.get(&["manifest"], "version") {
        Some(TValue::Int(1)) => {}
        Some(other) => {
            return Err(XtaskError::internal(format!(
                "boundaries.toml [manifest] version must be the integer 1, got {other:?}"
            )));
        }
        None => {
            return Err(XtaskError::internal(
                "boundaries.toml has no [manifest] version (expected version = 1)".to_string(),
            ));
        }
    }

    let rules_keys: BTreeSet<&str> = doc
        .entries_at(&["rules"])
        .into_iter()
        .map(|(k, _)| k.as_str())
        .collect();
    for key in &rules_keys {
        if !matches!(*key, "same_layer_pairs" | "dev_only" | "static_mut_allow") {
            return Err(XtaskError::internal(format!(
                "unknown key `{key}` in boundaries.toml [rules] \
                 (allowed: same_layer_pairs, dev_only, static_mut_allow)"
            )));
        }
    }

    let mut same_layer_pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for pair in str_array(&doc, &["rules"], "same_layer_pairs", &[])? {
        let Some((from, to)) = pair.split_once("->") else {
            return Err(XtaskError::internal(format!(
                "same_layer_pairs entries must look like \"from->to\", got `{pair}`"
            )));
        };
        let (from, to) = (from.trim(), to.trim());
        if from.is_empty() || to.is_empty() {
            return Err(XtaskError::internal(format!(
                "same_layer_pairs entry `{pair}` has an empty side"
            )));
        }
        same_layer_pairs.insert((from.to_string(), to.to_string()));
    }
    let dev_only: BTreeSet<String> = str_array(&doc, &["rules"], "dev_only", &[])?
        .into_iter()
        .collect();
    let static_mut_allow: BTreeSet<String> = str_array(&doc, &["rules"], "static_mut_allow", &[])?
        .into_iter()
        .collect();

    let mut crates: BTreeMap<String, CrateRule> = BTreeMap::new();
    for table in &doc.tables {
        match table.path.first().map(String::as_str) {
            Some("manifest") | Some("rules") => continue,
            Some("crates") => {
                if table.path.len() == 1 {
                    if !table.entries.is_empty() {
                        return Err(XtaskError::internal(
                            "entries directly under [crates] are not allowed — \
                             use [crates.<name>] sub-tables"
                                .to_string(),
                        ));
                    }
                    continue;
                }
                if table.path.len() != 2 {
                    return Err(XtaskError::internal(format!(
                        "boundaries.toml table [{}] is not a [crates.<name>] section",
                        table.path.join(".")
                    )));
                }
                let name = match table.path.get(1) {
                    Some(n) => n.clone(),
                    None => continue,
                };
                let rule = crates.entry(name.clone()).or_insert_with(|| CrateRule {
                    layer: None,
                    internal_deps: Vec::new(),
                    dev_deps: Vec::new(),
                    external_allowlist: Vec::new(),
                    external_dev_allowlist: Vec::new(),
                });
                for (key, value) in &table.entries {
                    match key.as_str() {
                        "layer" => {
                            rule.layer = value.as_int();
                        }
                        "internal_deps" => {
                            rule.internal_deps = str_array_value(value, "internal_deps")?;
                        }
                        "dev_deps" => {
                            rule.dev_deps = str_array_value(value, "dev_deps")?;
                        }
                        "external_allowlist" => {
                            rule.external_allowlist = str_array_value(value, "external_allowlist")?;
                        }
                        "external_dev_allowlist" => {
                            rule.external_dev_allowlist =
                                str_array_value(value, "external_dev_allowlist")?;
                        }
                        other => {
                            return Err(XtaskError::internal(format!(
                                "unknown key `{other}` in boundaries.toml [crates.{name}] \
                                 (allowed: layer, internal_deps, dev_deps, external_allowlist, \
                                 external_dev_allowlist)"
                            )));
                        }
                    }
                }
            }
            _ => {
                return Err(XtaskError::internal(format!(
                    "unknown table [{}] in boundaries.toml \
                     (allowed: [manifest], [rules], [crates.<name>])",
                    table.path.join(".")
                )));
            }
        }
    }

    Ok(Boundaries {
        crates,
        same_layer_pairs,
        dev_only,
        static_mut_allow,
    })
}

/// Read an optional string-array key from a table (default `fallback`).
fn str_array(
    doc: &TomlDoc,
    path: &[&str],
    key: &str,
    fallback: &[&str],
) -> Result<Vec<String>, XtaskError> {
    match doc.get(path, key) {
        None => Ok(fallback.iter().map(|s| s.to_string()).collect()),
        Some(v) => str_array_value(v, key),
    }
}

/// Interpret one value as a string array, with a contextual error otherwise.
fn str_array_value(value: &TValue, key: &str) -> Result<Vec<String>, XtaskError> {
    value.as_str_array().map(|v| v.to_vec()).ok_or_else(|| {
        XtaskError::internal(format!(
            "boundaries.toml key `{key}` must be an array of strings"
        ))
    })
}

// ---------------------------------------------------------------------------
// The five checks (doc 01 §5.2, per the 1.E1 card)
// ---------------------------------------------------------------------------

/// One rule violation: what broke, where, and the fix.
#[derive(Debug, Clone)]
struct Violation {
    /// Short rule tag for the report (EDGE, LAYER, ...).
    rule: &'static str,
    /// What and where.
    message: String,
    /// One-line fix hint.
    fix: String,
}

/// The outcome of one named check.
#[derive(Debug)]
struct CheckOutcome {
    /// 1-based check number (report order).
    id: usize,
    /// Check name (report + machine line).
    name: &'static str,
    violations: Vec<Violation>,
    /// Human-readable stats for the PASS line (also shown on FAIL).
    note: String,
}

impl CheckOutcome {
    fn is_pass(&self) -> bool {
        self.violations.is_empty()
    }

    fn status_str(&self) -> &'static str {
        if self.is_pass() { "pass" } else { "fail" }
    }
}

/// Run all five checks against the loaded workspace + boundaries.
fn run_checks(ws: &Workspace, b: &Boundaries, root: &Path) -> Vec<CheckOutcome> {
    vec![
        check_edges(ws, b),
        check_cycles(ws),
        check_lints(ws),
        match check_static_mut(ws, b, root) {
            Ok(outcome) => outcome,
            Err(err) => CheckOutcome {
                id: 4,
                name: CHECK_NAMES[3],
                violations: vec![Violation {
                    rule: "STATIC-MUT",
                    message: format!("scan failed: {err}"),
                    fix: "fix the scanner error above (exit 2 has details)".to_string(),
                }],
                note: "scan error".to_string(),
            },
        },
        check_testkit_direction(ws, b),
    ]
}

/// Check 1 — edge allowlist + layer ordering (doc 01 §5.2 steps 2-4).
fn check_edges(ws: &Workspace, b: &Boundaries) -> CheckOutcome {
    let mut violations: Vec<Violation> = Vec::new();
    let member_names: BTreeSet<&str> = ws.members.iter().map(|m| m.name.as_str()).collect();
    let mut n_internal = 0usize;
    let mut n_dev = 0usize;
    let mut n_external = 0usize;

    for m in &ws.members {
        let Some(rule) = b.crates.get(&m.name) else {
            violations.push(Violation {
                rule: "MANIFEST",
                message: format!(
                    "workspace crate `{}` ({}) is not declared in boundaries.toml \
                     (doc 01 §5.1)",
                    m.name, m.manifest_rel
                ),
                fix: "add a [crates.<name>] section in the same change that adds the \
                      crate (doc 01 §6.1 gate 3)"
                    .to_string(),
            });
            continue;
        };
        for d in &m.deps {
            if !member_names.contains(d.name.as_str()) {
                // External (registry) dependency.
                if d.is_dev() {
                    if !rule.external_dev_allowlist.contains(&d.name) {
                        violations.push(Violation {
                            rule: "EXTERNAL",
                            message: format!(
                                "{} [{}]: dev-dependency `{}` is not in \
                                 [crates.{}].external_dev_allowlist (doc 01 §5.2 step 4)",
                                m.manifest_rel, d.table, d.name, m.name
                            ),
                            fix: "remove it, or allowlist it in boundaries.toml after the \
                                  doc 11 dependency audit"
                                .to_string(),
                        });
                    }
                } else {
                    n_external += 1;
                    if !rule.external_allowlist.contains(&d.name) {
                        violations.push(Violation {
                            rule: "EXTERNAL",
                            message: format!(
                                "{} [{}]: external crate `{}` is not in \
                                 [crates.{}].external_allowlist (doc 01 §5.2 step 4)",
                                m.manifest_rel, d.table, d.name, m.name
                            ),
                            fix: "remove it, or allowlist it in boundaries.toml after the \
                                  doc 11 dependency audit"
                                .to_string(),
                        });
                    }
                }
                continue;
            }
            // Internal (workspace) dependency.
            if b.dev_only.contains(d.name.as_str()) && !d.is_dev() {
                // Check 5 owns normal edges into dev-only crates (ARCH-7).
                continue;
            }
            if d.name == m.name {
                // Check 2 owns self-edges (cycles).
                continue;
            }
            if d.is_dev() {
                n_dev += 1;
                if !rule.dev_deps.contains(&d.name) {
                    violations.push(Violation {
                        rule: "EDGE",
                        message: format!(
                            "{} [{}]: `{}` dev-depends on `{}`, which is not in \
                             [crates.{}].dev_deps (doc 01 §5.2 step 3)",
                            m.manifest_rel, d.table, m.name, d.name, m.name
                        ),
                        fix: "remove the dev-dependency, or extend [crates.<name>].dev_deps \
                              in boundaries.toml with Lead review (doc 01 §6.3)"
                            .to_string(),
                    });
                }
                // Layer ordering intentionally does not apply to dev edges:
                // doc 01's design has L0 media-types dev-depend on testkit.
                continue;
            }
            n_internal += 1;
            if !rule.internal_deps.contains(&d.name) {
                violations.push(Violation {
                    rule: "EDGE",
                    message: format!(
                        "{} [{}]: `{}` depends on `{}`, which is not in \
                         [crates.{}].internal_deps (doc 01 §5.2 step 3, ARCH-2)",
                        m.manifest_rel, d.table, m.name, d.name, m.name
                    ),
                    fix: "remove the dependency, or extend \
                          [crates.<name>].internal_deps in boundaries.toml \
                          with Lead review (doc 01 §6.3)"
                        .to_string(),
                });
                continue;
            }
            let target_layer = b.crates.get(&d.name).and_then(|target| target.layer);
            if let (Some(from_layer), Some(to_layer)) = (rule.layer, target_layer) {
                if from_layer < to_layer {
                    violations.push(Violation {
                        rule: "LAYER",
                        message: format!(
                            "internal edge `{}` -> `{}` points upward (layer {} -> {}); \
                             every edge must point downward or be a whitelisted same-layer \
                             pair (doc 01 §1.4, §5.2 step 2)",
                            m.name, d.name, from_layer, to_layer
                        ),
                        fix: "invert the dependency, or reassign layers with Lead review \
                              in boundaries.toml"
                            .to_string(),
                    });
                } else if from_layer == to_layer
                    && !b
                        .same_layer_pairs
                        .contains(&(m.name.clone(), d.name.clone()))
                {
                    violations.push(Violation {
                        rule: "LAYER",
                        message: format!(
                            "internal edge `{}` -> `{}` is same-layer ({} -> {}) but the \
                             pair is not whitelisted (doc 01 §1.4 invariant 1)",
                            m.name, d.name, from_layer, to_layer
                        ),
                        fix: "add \"from->to\" to [rules].same_layer_pairs with Lead \
                              review, or remove the edge"
                            .to_string(),
                    });
                }
            }
        }
    }

    for name in b.crates.keys() {
        if !member_names.contains(name.as_str()) {
            violations.push(Violation {
                rule: "MANIFEST",
                message: format!(
                    "boundaries.toml declares [crates.{name}] but no workspace crate \
                     named `{name}` exists"
                ),
                fix: "remove the stale entry".to_string(),
            });
        }
    }

    CheckOutcome {
        id: 1,
        name: CHECK_NAMES[0],
        note: format!(
            "{n_internal} internal edges, {n_dev} dev edges, {n_external} external deps — \
             all checked against boundaries.toml"
        ),
        violations,
    }
}

/// Check 2 — cycle scan over normal AND dev edges (doc 01 §5.2 step 5).
fn check_cycles(ws: &Workspace) -> CheckOutcome {
    let member_names: BTreeSet<&str> = ws.members.iter().map(|m| m.name.as_str()).collect();
    let mut adjacency: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut edge_count = 0usize;
    for m in &ws.members {
        let targets = adjacency.entry(m.name.as_str()).or_default();
        for d in &m.deps {
            if member_names.contains(d.name.as_str()) {
                edge_count += 1;
                targets.insert(d.name.as_str());
            }
        }
    }

    let mut color: BTreeMap<&str, u8> = BTreeMap::new(); // 0 white, 1 gray, 2 black
    let mut path: Vec<&str> = Vec::new();
    let mut cycles: Vec<String> = Vec::new();
    let mut seen: BTreeSet<BTreeSet<&str>> = BTreeSet::new();
    for name in adjacency.keys().copied() {
        dfs_cycles(
            name,
            &adjacency,
            &mut color,
            &mut path,
            &mut cycles,
            &mut seen,
        );
    }

    let violations: Vec<Violation> = cycles
        .iter()
        .map(|cycle| Violation {
            rule: "CYCLE",
            message: format!(
                "dependency cycle over normal+dev edges: {cycle} \
                 (doc 01 §5.2 step 5, ARCH-8)"
            ),
            fix: "break the cycle — dev-dependencies participate in the graph cargo \
                  will not reject for you"
                .to_string(),
        })
        .collect();

    CheckOutcome {
        id: 2,
        name: CHECK_NAMES[1],
        note: format!(
            "{} nodes, {} internal edges (normal + dev), {} cycles",
            adjacency.len(),
            edge_count,
            violations.len()
        ),
        violations,
    }
}

/// Depth-first three-color search recording each distinct cycle once
/// (deduplicated by the set of crates on it).
fn dfs_cycles<'a>(
    node: &'a str,
    adjacency: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    color: &mut BTreeMap<&'a str, u8>,
    path: &mut Vec<&'a str>,
    cycles: &mut Vec<String>,
    seen: &mut BTreeSet<BTreeSet<&'a str>>,
) {
    match color.get(node) {
        Some(1) => {
            // Back edge: node is on the current path — extract the cycle.
            if let Some(start) = path.iter().position(|n| *n == node) {
                let members: BTreeSet<&str> = path[start..].iter().copied().collect();
                if seen.insert(members) {
                    let mut rendered = path[start..].join(" -> ");
                    rendered.push_str(" -> ");
                    rendered.push_str(node);
                    cycles.push(rendered);
                }
            }
        }
        Some(2) => {}
        _ => {
            color.insert(node, 1);
            path.push(node);
            if let Some(targets) = adjacency.get(node) {
                for next in targets {
                    dfs_cycles(next, adjacency, color, path, cycles, seen);
                }
            }
            path.pop();
            color.insert(node, 2);
        }
    }
}

/// Check 3 — lint inheritance (doc 10 §3.1/§3.2 via doc 01 §5.2 step 7).
fn check_lints(ws: &Workspace) -> CheckOutcome {
    let mut violations: Vec<Violation> = Vec::new();
    if !ws.root_denies_unsafe {
        violations.push(Violation {
            rule: "UNSAFE",
            message: "the root manifest does not deny unsafe_code: expected \
                      [workspace.lints.rust] unsafe_code = \"deny\" (doc 10 §3.1, MP §9)"
                .to_string(),
            fix: "restore the deny in the root [workspace.lints.rust] table \
                  (Lead-owned manifest)"
                .to_string(),
        });
    }
    let mut inheriting = 0usize;
    for m in &ws.members {
        if m.lints_inherits && !m.lints_override {
            inheriting += 1;
            continue;
        }
        let detail = if m.lints_override {
            "it carries a [lints.<x>] override table instead"
        } else {
            "it has no `[lints]` table with `workspace = true`"
        };
        violations.push(Violation {
            rule: "UNSAFE",
            message: format!(
                "{} does not inherit the workspace lint table: {detail} \
                 (doc 10 §3.2 — no overrides, no lowering)",
                m.manifest_rel
            ),
            fix: "restore `[lints]` + `workspace = true` in the crate manifest".to_string(),
        });
    }
    CheckOutcome {
        id: 3,
        name: CHECK_NAMES[2],
        note: format!(
            "{}/{} crates inherit [lints] workspace = true; root {} unsafe_code",
            inheriting,
            ws.members.len(),
            if ws.root_denies_unsafe {
                "denies"
            } else {
                "DOES NOT deny"
            }
        ),
        violations,
    }
}

/// Check 4 — `static mut` scan over member `src/` trees
/// (doc 01 §5.2 step 6, ARCH-8).
fn check_static_mut(ws: &Workspace, b: &Boundaries, root: &Path) -> Result<CheckOutcome, String> {
    let mut violations: Vec<Violation> = Vec::new();
    let mut scanned_files = 0usize;
    let mut scanned_trees = 0usize;
    for m in &ws.members {
        if b.static_mut_allow.contains(&m.name) {
            continue;
        }
        let src = m.dir.join("src");
        if !src.is_dir() {
            continue;
        }
        scanned_trees += 1;
        let mut files: Vec<PathBuf> = Vec::new();
        collect_rs_files(&src, &mut files)
            .map_err(|e| format!("walking {} for the static-mut scan: {e}", src.display()))?;
        for file in files {
            let bytes =
                std::fs::read(&file).map_err(|e| format!("reading {}: {e}", file.display()))?;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            scanned_files += 1;
            let stripped = strip_strings_and_comments(&text);
            for line in static_mut_lines(&stripped) {
                violations.push(Violation {
                    rule: "STATIC-MUT",
                    message: format!(
                        "{}:{}: `static mut` global mutable state \
                         (doc 01 §5.2 step 6, ARCH-8)",
                        rel_path(root, &file),
                        line
                    ),
                    fix: "use owned state passed explicitly, or get a Lead-approved \
                          [rules].static_mut_allow entry"
                        .to_string(),
                });
            }
        }
    }
    Ok(CheckOutcome {
        id: 4,
        name: CHECK_NAMES[3],
        note: format!(
            "0 occurrences wanted; scanned {scanned_files} files across \
             {scanned_trees} src trees"
        ),
        violations,
    })
}

/// Check 5 — dev-only crates never in [dependencies] (MP §3.7, ARCH-7).
fn check_testkit_direction(ws: &Workspace, b: &Boundaries) -> CheckOutcome {
    let mut violations: Vec<Violation> = Vec::new();
    for m in &ws.members {
        for d in &m.deps {
            if !d.is_dev() && b.dev_only.contains(d.name.as_str()) {
                violations.push(Violation {
                    rule: "TESTKIT-DIRECTION",
                    message: format!(
                        "{} [{}]: dev-only crate `{}` must not be a normal dependency \
                         (MP §3.7, doc 01 §1.3 ARCH-7)",
                        m.manifest_rel, d.table, d.name
                    ),
                    fix: format!("move `{}` to [dev-dependencies]", d.name),
                });
            }
        }
    }
    let dev_only_list: Vec<&str> = b.dev_only.iter().map(String::as_str).collect();
    CheckOutcome {
        id: 5,
        name: CHECK_NAMES[4],
        note: format!(
            "dev-only crates [{}] appear in [dev-dependencies] tables only",
            dev_only_list.join(", ")
        ),
        violations,
    }
}

// ---------------------------------------------------------------------------
// `static mut` source scanner
// ---------------------------------------------------------------------------

/// Recursively collect `.rs` files under `dir` (sorted, deterministic).
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .collect::<Result<Vec<std::fs::DirEntry>, std::io::Error>>()?
        .into_iter()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            collect_rs_files(&entry, out)?;
        } else if entry.extension().is_some_and(|e| e == "rs") {
            out.push(entry);
        }
    }
    Ok(())
}

/// Replace comment and string-literal contents with spaces (newlines are
/// preserved 1:1 so line numbers stay accurate). Handles `//`, nested
/// `/* */`, `"..."` with escapes and `r#"..."#` raw strings. Char literals
/// and lifetimes are not special-cased — a `'` never opens a stripped
/// region (documented false-negative: a `'"'` char literal).
fn strip_strings_and_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;
    // State: (line comment | block depth | in string | raw-string hashes).
    let mut in_line_comment = false;
    let mut block_depth = 0usize;
    let mut in_string = false;
    let mut raw_hashes: Option<usize> = None;
    while i < chars.len() {
        let c = chars[i];
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
                out.push('\n');
            } else {
                out.push(' ');
            }
            i += 1;
        } else if block_depth > 0 {
            if c == '/' && chars.get(i + 1) == Some(&'*') {
                block_depth += 1;
                out.push(' ');
                out.push(' ');
                i += 2;
            } else if c == '*' && chars.get(i + 1) == Some(&'/') {
                block_depth -= 1;
                out.push(' ');
                out.push(' ');
                i += 2;
            } else {
                out.push(if c == '\n' { '\n' } else { ' ' });
                i += 1;
            }
        } else if in_string {
            match c {
                '\\' => {
                    // Keep line-continuations as newlines for line counting.
                    if chars.get(i + 1) == Some(&'\n') {
                        out.push(' ');
                        out.push('\n');
                    } else {
                        out.push(' ');
                        out.push(' ');
                    }
                    i += 2;
                }
                '"' => {
                    in_string = false;
                    out.push(' ');
                    i += 1;
                }
                '\n' => {
                    out.push('\n');
                    i += 1;
                }
                _ => {
                    out.push(' ');
                    i += 1;
                }
            }
        } else if let Some(hashes) = raw_hashes {
            if c == '"' {
                let mut j = i + 1;
                let mut closing = true;
                for _ in 0..hashes {
                    match chars.get(j) {
                        Some('#') => j += 1,
                        _ => {
                            closing = false;
                            break;
                        }
                    }
                }
                if closing {
                    raw_hashes = None;
                    out.push(' ');
                    i = j;
                    continue;
                }
            }
            out.push(if c == '\n' { '\n' } else { ' ' });
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            in_line_comment = true;
            out.push(' ');
            out.push(' ');
            i += 2;
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            block_depth = 1;
            out.push(' ');
            out.push(' ');
            i += 2;
        } else if c == '"' {
            in_string = true;
            out.push(' ');
            i += 1;
        } else if c == 'r' {
            // Raw string r"..." / r#"..."# (a raw identifier r#foo does not
            // match: no quote follows the hashes).
            let mut j = i + 1;
            let mut hashes = 0usize;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                raw_hashes = Some(hashes);
                out.push(' ');
                i = j + 1;
            } else {
                out.push(c);
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Rust identifier continuation character.
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// 1-based line numbers of every `static mut` occurrence in already-stripped
/// source text (word-boundary match, whitespace — including newlines —
/// between the two keywords).
fn static_mut_lines(stripped: &str) -> Vec<usize> {
    const KEYWORD: &str = "static";
    let mut lines = Vec::new();
    let mut from = 0usize;
    while let Some(offset) = stripped[from..].find(KEYWORD) {
        let at = from + offset;
        from = at + KEYWORD.len();
        // Word boundary before "static".
        let before_ok = stripped[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !is_ident_char(c));
        if !before_ok {
            continue;
        }
        // Whitespace run, then "mut" with its own word boundary.
        let rest = &stripped[from..];
        let ws_len = rest
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
        let after = &rest[ws_len..];
        if !after.starts_with("mut") {
            continue;
        }
        let mut_end = from + ws_len + 3;
        let mut_ok = stripped[mut_end..]
            .chars()
            .next()
            .is_none_or(|c| !is_ident_char(c));
        if !mut_ok {
            continue;
        }
        let line = stripped[..at].matches('\n').count() + 1;
        lines.push(line);
    }
    lines
}

// ---------------------------------------------------------------------------
// Report (human + JSON)
// ---------------------------------------------------------------------------

/// The full check run, ready to render.
#[derive(Debug)]
struct Report {
    root: String,
    members: usize,
    declared: usize,
    checks: Vec<CheckOutcome>,
}

impl Report {
    fn failed_checks(&self) -> usize {
        self.checks.iter().filter(|c| !c.is_pass()).count()
    }

    fn result(&self) -> Verdict {
        if self.failed_checks() == 0 {
            Verdict::Pass
        } else {
            Verdict::Fail
        }
    }

    /// The machine summary line (single line, grep-friendly).
    fn machine_line(&self) -> String {
        let mut line = format!("architecture: result={}", self.result().as_str());
        for (idx, check) in self.checks.iter().enumerate() {
            line.push(' ');
            line.push_str(CHECK_NAMES.get(idx).unwrap_or(&"check"));
            line.push('=');
            line.push_str(check.status_str());
        }
        line
    }

    fn print_human(&self, report_path: &Path) {
        println!("=== LUMEN ARCHITECTURE REPORT ===");
        println!("root:       {}", self.root);
        println!(
            "members:    {} workspace crates, {} declared in boundaries.toml",
            self.members, self.declared
        );
        println!("spec:       doc 01 §5.2 / gate G5 (doc 10 §4.6); manifest version 1");
        println!();
        for check in &self.checks {
            println!(
                "{}  {}",
                check_line(check.id, check.name),
                if check.is_pass() {
                    format!("PASS   ({})", check.note)
                } else {
                    format!(
                        "FAIL   ({} violation{})",
                        check.violations.len(),
                        if check.violations.len() == 1 { "" } else { "s" }
                    )
                }
            );
            for v in &check.violations {
                println!("     [{}] {}", v.rule, v.message);
                println!("       fix: {}", v.fix);
            }
        }
        println!();
        match self.result() {
            Verdict::Pass => println!("result: PASS (5 of 5 checks)"),
            Verdict::Fail => println!(
                "result: FAILURE ({} of 5 checks failed)",
                self.failed_checks()
            ),
        }
        println!("{}", self.machine_line());
        println!("full log: {}", rel_path(Path::new(&self.root), report_path));
    }

    /// The JSON artifact (doc 01 §5.2 step 8), schema `architecture-report/1`.
    fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str("  \"schema\": \"architecture-report/1\",\n");
        out.push_str(&format!("  \"generated_unix\": {},\n", unix_now()));
        out.push_str(&format!("  \"root\": \"{}\",\n", json_escape(&self.root)));
        out.push_str(&format!("  \"members\": {},\n", self.members));
        out.push_str(&format!("  \"declared\": {},\n", self.declared));
        out.push_str("  \"checks\": [\n");
        for (idx, check) in self.checks.iter().enumerate() {
            out.push_str("    {\n");
            out.push_str(&format!("      \"id\": {},\n", check.id));
            out.push_str(&format!(
                "      \"name\": \"{}\",\n",
                json_escape(check.name)
            ));
            out.push_str(&format!("      \"status\": \"{}\",\n", check.status_str()));
            out.push_str(&format!(
                "      \"note\": \"{}\",\n",
                json_escape(&check.note)
            ));
            out.push_str("      \"violations\": [");
            if check.violations.is_empty() {
                out.push(']');
            } else {
                out.push('\n');
                for (v_idx, v) in check.violations.iter().enumerate() {
                    out.push_str("        {\n");
                    out.push_str(&format!(
                        "          \"rule\": \"{}\",\n",
                        json_escape(v.rule)
                    ));
                    out.push_str(&format!(
                        "          \"message\": \"{}\",\n",
                        json_escape(&v.message)
                    ));
                    out.push_str(&format!("          \"fix\": \"{}\"\n", json_escape(&v.fix)));
                    out.push_str("        }");
                    out.push(if v_idx + 1 == check.violations.len() {
                        '\n'
                    } else {
                        ','
                    });
                }
                out.push_str("      ]");
            }
            out.push_str("\n    }");
            if idx + 1 == self.checks.len() {
                out.push('\n');
            } else {
                out.push_str(",\n");
            }
        }
        out.push_str("  ],\n");
        out.push_str(&format!("  \"result\": \"{}\",\n", self.result().as_str()));
        out.push_str(&format!(
            "  \"summary\": \"{}\"\n",
            json_escape(&self.machine_line())
        ));
        out.push_str("}\n");
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Pass,
    Fail,
}

impl Verdict {
    fn as_str(self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
        }
    }

    fn is_pass(self) -> bool {
        self == Verdict::Pass
    }
}

/// `1  edges ...........` — gate.rs-style padded check label.
fn check_line(id: usize, name: &str) -> String {
    let mut line = format!("{id} {name}");
    while line.chars().count() < 24 {
        line.push('.');
    }
    line
}

/// Write the JSON report under `<root>/target/`, returning its path.
fn write_json_report(root: &Path, json: &str) -> Result<PathBuf, XtaskError> {
    let dir = root.join("target");
    std::fs::create_dir_all(&dir)
        .map_err(|e| XtaskError::io("creating target/ for the architecture report", e))?;
    let path = root.join(REPORT_REL);
    std::fs::write(&path, json).map_err(|e| XtaskError::io(format!("writing {REPORT_REL}"), e))?;
    Ok(path)
}

/// Escape a string for JSON (RFC 8259): quotes, backslash, \b \f \n \r \t,
/// other control chars as \u00XX. Non-ASCII passes through (UTF-8 output).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Whole seconds since the Unix epoch (0 if the clock is before 1970).
fn unix_now() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    }
}

/// Path relative to the checked root, forward slashes; absolute display
/// fallback when the path is not under the root.
fn rel_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => path.display().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Unit tests (fixtures run the real binary in tests/check_architecture.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- TOML subset reader ------------------------------------------------

    #[test]
    fn toml_parses_the_boundaries_shape() {
        let text = "\
# comment
[manifest]
version = 1

[rules]
same_layer_pairs = [\"a->b\"]
dev_only = [\"testkit\"]
static_mut_allow = []

[crates.a]
layer = 1
internal_deps = [\"b\", \"c\"]
dev_deps = []
external_allowlist = [\"serde\"]
external_dev_allowlist = []
";
        let doc = match parse_toml(text) {
            Ok(doc) => doc,
            Err(e) => unreachable!("parse failed at line {}: {}", e.line, e.message),
        };
        assert_eq!(doc.get(&["manifest"], "version"), Some(&TValue::Int(1)));
        assert_eq!(
            doc.get(&["rules"], "dev_only"),
            Some(&TValue::StrArray(vec!["testkit".to_string()]))
        );
        assert_eq!(
            doc.get(&["crates", "a"], "internal_deps"),
            Some(&TValue::StrArray(vec!["b".to_string(), "c".to_string()]))
        );
        assert_eq!(doc.get(&["crates", "a"], "layer"), Some(&TValue::Int(1)));
    }

    #[test]
    fn toml_parses_a_crate_manifest_shape() {
        let text = "\
[package]
name = \"alpha\"
version = \"0.1.0\"
edition = \"2024\"
publish = false

[lints]
workspace = true

[dependencies]
beta = { path = \"../beta\", version = \"0.1\" }
serde = \"1\"

[dev-dependencies]
testkit = { path = \"../testkit\" }

[[bin]]
name = \"alpha-gen\"
path = \"src/bin/gen.rs\"

[workspace.lints.rust]
unsafe_code = \"deny\"
";
        let doc = match parse_toml(text) {
            Ok(doc) => doc,
            Err(e) => unreachable!("parse failed at line {}: {}", e.line, e.message),
        };
        assert_eq!(
            doc.get(&["package"], "name"),
            Some(&TValue::Str("alpha".to_string()))
        );
        assert_eq!(doc.get(&["lints"], "workspace"), Some(&TValue::Bool(true)));
        // Dependency KEYS are reachable even though the values are inline
        // tables (opaque).
        let dep_keys: Vec<String> = doc
            .entries_at(&["dependencies"])
            .into_iter()
            .map(|(k, _)| k.clone())
            .collect();
        assert_eq!(dep_keys, vec!["beta".to_string(), "serde".to_string()]);
        assert!(doc.has_table(&["bin"]));
        assert!(doc.has_table(&["workspace", "lints", "rust"]));
    }

    #[test]
    fn toml_rejects_duplicate_tables_and_keys() {
        let dup_table = "[a]\nx = 1\n[a]\ny = 2\n";
        match parse_toml(dup_table) {
            Ok(_) => unreachable!("duplicate table must fail"),
            Err(e) => assert!(e.message.contains("duplicate table"), "{}", e.message),
        }
        let dup_key = "[a]\nx = 1\nx = 2\n";
        match parse_toml(dup_key) {
            Ok(_) => unreachable!("duplicate key must fail"),
            Err(e) => assert!(e.message.contains("duplicate key"), "{}", e.message),
        }
    }

    #[test]
    fn toml_accepts_multiline_arrays_and_rejects_dotted_keys_and_bad_escapes() {
        // Multi-line string arrays PARSE since the W25 CI-repair pass
        // (the codecs/web tree-shake lane ships them in member
        // manifests): elements may span lines, with comments between.
        let doc = match parse_toml("[a]\nx = [\"one\",\n # a comment\n \"two\",\n]\n") {
            Ok(doc) => doc,
            Err(e) => unreachable!("multi-line array must parse: {}", e.message),
        };
        assert_eq!(
            doc.get(&["a"], "x"),
            Some(&TValue::StrArray(vec![
                "one".to_string(),
                "two".to_string()
            ]))
        );
        // An unterminated array still fails loudly.
        match parse_toml("[a]\nx = [\"one\",\n \"two\"\n") {
            Ok(_) => unreachable!("unterminated array must fail"),
            Err(e) => assert!(e.message.contains("unterminated array"), "{}", e.message),
        }
        match parse_toml("[a]\nx.y = 1\n") {
            Ok(_) => unreachable!("dotted key must fail"),
            Err(e) => assert!(e.message.contains("dotted keys"), "{}", e.message),
        }
        match parse_toml("[a]\nx = \"bad \\q escape\"\n") {
            Ok(_) => unreachable!("unsupported escape must fail"),
            Err(e) => assert!(e.message.contains("unsupported escape"), "{}", e.message),
        }
    }

    #[test]
    fn toml_handles_quoted_header_segments_and_arrays_of_tables() {
        let text = "\
[target.'cfg(windows)'.dependencies]
winapi = \"0.3\"

[[bench]]
name = \"b1\"

[[bench]]
name = \"b2\"
";
        let doc = match parse_toml(text) {
            Ok(doc) => doc,
            Err(e) => unreachable!("parse failed at line {}: {}", e.line, e.message),
        };
        let dep_keys: Vec<String> = doc
            .entries_at(&["target", "cfg(windows)", "dependencies"])
            .into_iter()
            .map(|(k, _)| k.clone())
            .collect();
        assert_eq!(dep_keys, vec!["winapi".to_string()]);
        // Two [[bench]] tables coexist.
        assert_eq!(
            doc.tables
                .iter()
                .filter(|t| t.path == vec!["bench".to_string()])
                .count(),
            2
        );
    }

    // --- static-mut scanner -------------------------------------------------

    #[test]
    fn stripper_removes_comments_and_strings_but_keeps_code() {
        let src = "// static mut in a line comment\n\
                   /* static mut in\n a block /* nested */ comment */\n\
                   let s = \"static mut in a string\";\n\
                   let r = r#\"static mut in a raw string\"#;\n\
                   pub static mut COUNTER: u32 = 0;\n\
                   let doc = \"line1 \\\n line2\";\n";
        let stripped = strip_strings_and_comments(src);
        // Newlines preserved 1:1.
        assert_eq!(
            src.matches('\n').count(),
            stripped.matches('\n').count(),
            "newline count must be preserved for line numbers"
        );
        let lines = static_mut_lines(&stripped);
        assert_eq!(lines, vec![6], "only the real declaration on line 6");
    }

    #[test]
    fn static_mut_matcher_requires_word_boundaries_and_whitespace() {
        // `static\nmut` across lines still matches.
        let a = "static\nmut X: u8 = 0;\n";
        assert_eq!(static_mut_lines(a), vec![1]);
        // Identifier-adjacent text must not match.
        assert!(static_mut_lines("staticx mut Y;").is_empty());
        assert!(static_mut_lines("astatic mut Y;").is_empty());
        assert!(static_mut_lines("static mutx: u8 = 0;").is_empty());
        // Plain (immutable) statics must not match.
        assert!(static_mut_lines("static X: u8 = 0;").is_empty());
        assert!(static_mut_lines("pub static MUT: u8 = 0;").is_empty());
    }

    // --- cycle detection ----------------------------------------------------

    #[test]
    fn cycle_detection_finds_dev_edge_cycles_and_self_loops() {
        let mut adjacency: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        adjacency.insert("a", BTreeSet::from(["b"]));
        adjacency.insert("b", BTreeSet::from(["a"]));
        let mut color = BTreeMap::new();
        let mut path = Vec::new();
        let mut cycles = Vec::new();
        let mut seen = BTreeSet::new();
        for node in adjacency.keys() {
            dfs_cycles(
                node,
                &adjacency,
                &mut color,
                &mut path,
                &mut cycles,
                &mut seen,
            );
        }
        assert_eq!(cycles.len(), 1, "one distinct cycle: {cycles:?}");
        assert!(
            cycles[0].starts_with("a -> b -> a") || cycles[0].starts_with("b -> a -> b"),
            "cycle path rendered: {}",
            cycles[0]
        );

        let mut self_adj: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        self_adj.insert("solo", BTreeSet::from(["solo"]));
        let mut color = BTreeMap::new();
        let mut path = Vec::new();
        let mut cycles = Vec::new();
        let mut seen = BTreeSet::new();
        dfs_cycles(
            "solo",
            &self_adj,
            &mut color,
            &mut path,
            &mut cycles,
            &mut seen,
        );
        assert_eq!(cycles, vec!["solo -> solo".to_string()]);
    }

    #[test]
    fn cycle_detection_is_quiet_on_a_dag() {
        let mut adjacency: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        adjacency.insert("a", BTreeSet::from(["b"]));
        adjacency.insert("b", BTreeSet::from(["c"]));
        adjacency.insert("c", BTreeSet::from([]));
        let mut color = BTreeMap::new();
        let mut path = Vec::new();
        let mut cycles = Vec::new();
        let mut seen = BTreeSet::new();
        for node in adjacency.keys() {
            dfs_cycles(
                node,
                &adjacency,
                &mut color,
                &mut path,
                &mut cycles,
                &mut seen,
            );
        }
        assert!(cycles.is_empty());
    }

    // --- report rendering ---------------------------------------------------

    #[test]
    fn machine_line_lists_every_check_status() {
        let report = Report {
            root: "/tmp/x".to_string(),
            members: 2,
            declared: 2,
            checks: vec![
                CheckOutcome {
                    id: 1,
                    name: CHECK_NAMES[0],
                    violations: vec![Violation {
                        rule: "EDGE",
                        message: "m".to_string(),
                        fix: "f".to_string(),
                    }],
                    note: "n".to_string(),
                },
                CheckOutcome {
                    id: 2,
                    name: CHECK_NAMES[1],
                    violations: Vec::new(),
                    note: "n".to_string(),
                },
                CheckOutcome {
                    id: 3,
                    name: CHECK_NAMES[2],
                    violations: Vec::new(),
                    note: "n".to_string(),
                },
                CheckOutcome {
                    id: 4,
                    name: CHECK_NAMES[3],
                    violations: Vec::new(),
                    note: "n".to_string(),
                },
                CheckOutcome {
                    id: 5,
                    name: CHECK_NAMES[4],
                    violations: Vec::new(),
                    note: "n".to_string(),
                },
            ],
        };
        assert_eq!(
            report.machine_line(),
            "architecture: result=fail edges=fail cycles=pass unsafe-lints=pass \
             static-mut=pass testkit-direction=pass"
        );
        assert_eq!(report.failed_checks(), 1);
    }

    #[test]
    fn json_report_escapes_and_carries_violations() {
        let report = Report {
            root: "/tmp/x\"y".to_string(),
            members: 1,
            declared: 1,
            checks: vec![
                CheckOutcome {
                    id: 1,
                    name: CHECK_NAMES[0],
                    violations: Vec::new(),
                    note: "clean".to_string(),
                },
                CheckOutcome {
                    id: 2,
                    name: CHECK_NAMES[1],
                    violations: vec![Violation {
                        rule: "CYCLE",
                        message: "a -> b -> a\nsecond line".to_string(),
                        fix: "break it".to_string(),
                    }],
                    note: "1 cycle".to_string(),
                },
                CheckOutcome {
                    id: 3,
                    name: CHECK_NAMES[2],
                    violations: Vec::new(),
                    note: "ok".to_string(),
                },
                CheckOutcome {
                    id: 4,
                    name: CHECK_NAMES[3],
                    violations: Vec::new(),
                    note: "ok".to_string(),
                },
                CheckOutcome {
                    id: 5,
                    name: CHECK_NAMES[4],
                    violations: Vec::new(),
                    note: "ok".to_string(),
                },
            ],
        };
        let json = report.to_json();
        assert!(
            json.contains("\"schema\": \"architecture-report/1\""),
            "{json}"
        );
        assert!(json.contains("\"root\": \"/tmp/x\\\"y\""), "{json}");
        assert!(json.contains("\"status\": \"fail\""), "{json}");
        assert!(json.contains("\\nsecond line"), "{json}");
        assert!(json.contains("\"result\": \"fail\""), "{json}");
    }

    #[test]
    fn toml_subset_supports_bom_and_trailing_comments() {
        let text = "\u{feff}[a]\nkey = \"value\" # trailing comment\n";
        let doc = match parse_toml(text) {
            Ok(doc) => doc,
            Err(e) => unreachable!("parse failed at line {}: {}", e.line, e.message),
        };
        assert_eq!(
            doc.get(&["a"], "key"),
            Some(&TValue::Str("value".to_string()))
        );
    }
}
