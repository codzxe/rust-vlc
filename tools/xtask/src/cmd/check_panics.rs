//! `xtask check-panics` — the panic-surface gate (card 7-e, audit F-5
//! of `docs/audits/error-surface-20261023.md`).
//!
//! # Why this exists (the verified clippy blind spot)
//!
//! The audit's §0 controlled experiment proved that the workspace's
//! denied-lint block (`unwrap_used`, `expect_used`, `panic`, `todo`,
//! `unimplemented` — all `deny` at the root manifest) does **not**
//! cover `unreachable!`: a throwaway crate with the same five denied
//! lints compiled `unreachable!("provably dead")` with **no
//! diagnostic**, and the house convention
//! `unwrap_or_else(|e| unreachable!("{e}"))` hides the `unwrap` from
//! `unwrap_used` entirely. The 27 non-test `unreachable!` sites the
//! audit counted therefore passed the gate by lint blind spot, not by
//! proof. This check encodes the audit's classification as CI: a
//! grep-based scan over the nine engine crates' `src/` trees that
//! fails on any panic-family form outside the sanctioned zones.
//!
//! # The scanned forms (audit §0's raw sweep, verbatim)
//!
//! `.unwrap(` · `.expect(` · `panic!(` · `unreachable!(` · `todo!(` ·
//! `unimplemented!(` — matched on a string/comment-stripped copy of
//! the source, so doc examples and prose never fire. (Like the audit's
//! grep, the method forms require the `.` and an adjacent `(`;
//! `Option::unwrap(f)`-style free calls and spaced `unwrap (` shapes
//! are outside both nets.)
//!
//! # The sanctioned zones (audit §0's classification)
//!
//! 1. **Test context** — anything under a `#[cfg(test)]` /
//!    `#[cfg(any(test, …))]` / `#[cfg(all(…, test, …))]` / `#[test]`
//!    attribute's item (tracked by brace regions), any file pulled in
//!    by a file-level `#[cfg(test)] mod name;` declaration, and any
//!    file whose own `#![cfg(test)]` makes it wholly test-gated.
//! 2. **The executable-contract modules** — `input/src/contract.rs`
//!    and `media-probe/src/contract.rs` (audit §1.8/§1.9; F-2 keeps
//!    them out of release builds — the scan allows them by path
//!    regardless of feature state).
//! 3. **Doc examples** — comments are stripped before matching.
//! 4. **Audit-cited DOC-COMMENTED INVARIANT sites** — the explicit
//!    [`CITED_INVARIANTS`] table below (path + message fragment, both
//!    must match the offending line). No F-item retired these; adding
//!    an entry is a deliberate, reviewable edit to this file.
//!
//! # Scope and limits (honest)
//!
//! - Only the nine engine crates' `src/` trees (the audit's domain);
//!   integration tests under `tests/` are test context by definition.
//!   Extending the scope is a one-line change to [`SCANNED_CRATES`].
//! - The classifier is lexical, not semantic: a `#[cfg(test)]` on a
//!    statement inside a function body (nonexistent in this tree)
//!    would not open a region; `cfg(not(test))` code is correctly
//!    treated as non-test. Direct-indexing panics (audit §4) are out
//!   of scope.
//! - `assert!`/`assert_eq!` (the assert family) are NOT scanned — the
//!   audit kept them informational; the two contract modules carry
//!   all 43 non-test uses.
//!
//! ```text
//! Usage: cargo xtask check-panics [--root <dir>]
//!
//!   --root <dir>  check the workspace rooted at <dir> instead of walking up
//!                 from the current directory (used by the fixture-based
//!                 acceptance proof in tools/xtask/tests/check_panics.rs)
//!
//! Exit code: 0 no violations | 1 violations found | 2 xtask error.
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::XtaskError;

/// The nine engine crates of the audit's scope (§0): their `src/`
/// trees are the scan domain. A missing one is an error, never a
/// silent scope shrink.
const SCANNED_CRATES: [&str; 9] = [
    "playback-engine",
    "media-clock",
    "packet-queue",
    "demux",
    "codecs",
    "audio-engine",
    "video-engine",
    "input",
    "media-probe",
];

/// The sanctioned executable-contract modules (audit §1.8/§1.9, fix
/// F-2): test/conformance-only surface, feature-gated out of release
/// builds. Paths are relative to the workspace root.
const CONTRACT_MODULES: [&str; 2] = [
    "crates/input/src/contract.rs",
    "crates/media-probe/src/contract.rs",
];

/// Audit-cited DOC-COMMENTED INVARIANT sites no F-item retired. An
/// entry is `(path, message fragment)`: the fragment must appear on
/// the same RAW (unstripped) source line as the panic-family match —
/// the message lives in a string literal, which the stripped scan
/// copy blanks, so the citation check reads the original line. Editing
/// the message or moving the site re-reviews the entry — the allowance
/// cannot drift into a blanket file pass. The audit's 28 non-test
/// matches were 1 expect (F-1) + 27 `unreachable!` (24 in the F-2
/// contract modules); these three are the residual, each provably
/// dead, each cited below.
const CITED_INVARIANTS: [(&str, &str); 3] = [
    (
        // Audit §1.6: a just-constructed SharedBytes has refcount 1 by
        // Arc semantics, so get_mut is Some — Rust's own type invariant,
        // stated in the message itself.
        "crates/audio-engine/src/pipeline.rs",
        "fresh SharedBytes has refcount 1",
    ),
    (
        // The same SharedBytes refcount-1 invariant, same slot-rotation
        // cold path, in the PCM decoder's convert-slot arena: `from_vec`
        // moves a fresh allocation in (no interning/caching anywhere in
        // the type — audit §1.6's rationale transfers verbatim), so
        // `Arc::get_mut` on the just-built handle is `Some`.
        "crates/codecs/src/audio/pcm/decoder.rs",
        "fresh SharedBytes has refcount 1",
    ),
    (
        // `retrying()` is `matches!(self.phase, SourcePhase::Waiting{..})`
        // (recovery.rs:444-446) and the caller early-returns unless it
        // holds, so the `_` arm of the immediately following phase match
        // is pinned dead — the arm's own comment states the pin. Cited at
        // the F-5 gate's introduction (card 7-e): the audit counted this
        // among the 3 residual non-test sites (27 − 24 contract-gated)
        // but printed no per-site row; this entry is the reviewable act
        // that makes that explicit.
        "crates/playback-engine/src/recovery.rs",
        "retrying implies Waiting",
    ),
];

/// The panic-family forms scanned (audit §0's raw sweep, verbatim).
const FORMS: [&str; 6] = [
    ".unwrap(",
    ".expect(",
    "panic!(",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
];

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Entry point for `cargo xtask check-panics`.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut root_override: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
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

    let outcome = scan_workspace(&root)?;
    outcome.print(&root);
    if outcome.violations.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

/// The check's plain-text usage (mirrors the module docs).
const USAGE: &str = "\
Usage: cargo xtask check-panics [--root <dir>]

  Scans the nine engine crates' src/ trees (audit F-5 scope) for the
  panic family (.unwrap / .expect / panic! / unreachable! / todo! /
  unimplemented!) outside #[cfg(test)] regions, test-only mod files,
  the two executable-contract modules, doc examples (comments are
  stripped) and the audit-cited invariant table.

  --root <dir>  check the workspace rooted at <dir> instead of walking up

Exit code: 0 no violations | 1 violations found | 2 xtask error.";

// ---------------------------------------------------------------------------
// Workspace scan
// ---------------------------------------------------------------------------

/// One panic-family match found in stripped (comment-free) source.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Match {
    /// 1-based line number.
    line: usize,
    /// The matched form, verbatim (e.g. `unreachable!(`).
    form: &'static str,
    /// The stripped line, trimmed (evidence for the report).
    snippet: String,
}

/// The classification of an allowed match (for the summary counts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Allowance {
    /// Inside a `#[cfg(test)]`-style region in the same file.
    TestRegion,
    /// One of the two executable-contract modules.
    ContractModule,
    /// An audit-cited invariant site (path + fragment).
    CitedInvariant,
}

impl Allowance {
    fn label(self) -> &'static str {
        match self {
            Allowance::TestRegion => "cfg-test region",
            Allowance::ContractModule => "contract module",
            Allowance::CitedInvariant => "cited invariant",
        }
    }
}

/// The whole-workspace outcome.
struct Outcome {
    files_scanned: usize,
    test_regions: usize,
    test_mod_files: usize,
    allowed: Vec<(String, Match, Allowance)>,
    violations: Vec<(String, Match)>,
}

impl Outcome {
    fn print(&self, root: &Path) {
        println!(
            "check-panics: {} files scanned across {} crates",
            self.files_scanned,
            SCANNED_CRATES.len()
        );
        println!(
            "  test context: {} cfg-test regions, {} file-level test-only mod files",
            self.test_regions, self.test_mod_files
        );
        for (path, m, why) in &self.allowed {
            println!(
                "  allowed ({}): {}:{}: {} — {}",
                why.label(),
                rel(root, path),
                m.line,
                m.form.trim_start_matches('.'),
                m.snippet
            );
        }
        if self.violations.is_empty() {
            println!("check-panics: PASS — 0 violations (audit F-5)");
        } else {
            println!(
                "check-panics: FAIL — {} violation(s) outside the sanctioned zones:",
                self.violations.len()
            );
            for (path, m) in &self.violations {
                println!(
                    "  {}:{}: {} in non-test code — {}",
                    rel(root, path),
                    m.line,
                    m.form.trim_start_matches('.'),
                    m.snippet
                );
            }
            println!(
                "  fix: return a typed error, or move the site into a #[cfg(test)] \
                 region; a provably-dead production invariant belongs in \
                 CITED_INVARIANTS (tools/xtask/src/cmd/check_panics.rs) with an \
                 audit citation — a reviewable act, never a silent pass"
            );
        }
    }
}

/// `path` relative to `root` when possible (evidence formatting).
fn rel(root: &Path, path: &str) -> String {
    Path::new(path)
        .strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.to_string())
}

/// Scan the nine crates under `root`.
fn scan_workspace(root: &Path) -> Result<Outcome, XtaskError> {
    let mut files: Vec<PathBuf> = Vec::new();
    for crate_name in SCANNED_CRATES {
        let src = root.join("crates").join(crate_name).join("src");
        if !src.is_dir() {
            return Err(XtaskError::internal(format!(
                "scanned crate `{crate_name}` has no src/ tree at {} — the audit \
                 scope must not silently shrink (rename? update SCANNED_CRATES)",
                src.display()
            )));
        }
        collect_rs_files(&src, &mut files)
            .map_err(|e| XtaskError::io(format!("walking {}", src.display()), e))?;
    }
    files.sort();

    // Pass 1: scan every file; collect the file-level test-mod decls.
    let mut scans: Vec<(PathBuf, FileScan)> = Vec::with_capacity(files.len());
    for file in &files {
        let src = std::fs::read_to_string(file)
            .map_err(|e| XtaskError::io(format!("reading {}", file.display()), e))?;
        scans.push((file.clone(), scan_source(&src, file)));
    }
    let test_mod_files: Vec<PathBuf> = scans
        .iter()
        .flat_map(|(_, scan)| scan.mod_files.iter().cloned())
        .collect();

    // Pass 2: classify.
    let mut outcome = Outcome {
        files_scanned: scans.len(),
        test_regions: scans.iter().map(|(_, s)| s.test_regions.len()).sum(),
        test_mod_files: test_mod_files.len(),
        allowed: Vec::new(),
        violations: Vec::new(),
    };
    for (file, scan) in &scans {
        let file_str = file.display().to_string();
        let rel_file = rel(root, &file_str);
        let is_contract_module = CONTRACT_MODULES.contains(&rel_file.as_str());
        let is_test_mod_file = test_mod_files.contains(file);
        for m in find_matches(&scan.stripped) {
            if scan.whole_file_test || is_test_mod_file {
                // Whole-file test context — the file itself is the
                // zone (counted in the summary, not per-match).
                continue;
            }
            if is_contract_module {
                outcome
                    .allowed
                    .push((file_str.clone(), m, Allowance::ContractModule));
                continue;
            }
            if scan
                .test_regions
                .iter()
                .any(|(start, end)| m.line >= *start && m.line <= *end)
            {
                outcome
                    .allowed
                    .push((file_str.clone(), m, Allowance::TestRegion));
                continue;
            }
            // Cited invariants match on the RAW line: the message
            // fragment is a string literal, blanked in `stripped`.
            if scan
                .raw_lines
                .get(m.line - 1)
                .is_some_and(|raw_line| {
                    CITED_INVARIANTS.iter().any(|(path, fragment)| {
                        *path == rel_file && raw_line.contains(fragment)
                    })
                })
            {
                outcome
                    .allowed
                    .push((file_str.clone(), m, Allowance::CitedInvariant));
                continue;
            }
            outcome.violations.push((file_str.clone(), m));
        }
    }
    Ok(outcome)
}

// ---------------------------------------------------------------------------
// Source scanner: strip strings/comments, track cfg-test regions
// ---------------------------------------------------------------------------

/// One file's classification inputs.
struct FileScan {
    /// The source with every string/char/comment character (except
    /// newlines) replaced by a space — char-for-char aligned with the
    /// input, so line numbers are trivial and matches never fire on
    /// doc examples or string contents.
    stripped: String,
    /// The raw (unstripped) source lines — [`CITED_INVARIANTS`]
    /// fragments live in string literals, which `stripped` blanks.
    raw_lines: Vec<String>,
    /// 1-based inclusive line ranges covered by a test-context item.
    test_regions: Vec<(usize, usize)>,
    /// Files pulled in by file-level `#[cfg(test)] mod name;` decls.
    mod_files: Vec<PathBuf>,
    /// The file's own `#![cfg(test)]` makes it wholly test context.
    whole_file_test: bool,
}

/// The lexer modes.
#[derive(PartialEq)]
enum Mode {
    Normal,
    LineComment,
    BlockComment,
    Str,
    Char,
    /// A raw string with `n` leading `#`s.
    RawStr(usize),
}

/// Scan one source file.
fn scan_source(src: &str, file: &Path) -> FileScan {
    let chars: Vec<char> = src.chars().collect();
    let mut stripped = String::with_capacity(src.len());
    let mut test_regions: Vec<(usize, usize)> = Vec::new();
    let mut mod_files: Vec<PathBuf> = Vec::new();
    let mut whole_file_test = false;

    let mut mode = Mode::Normal;
    // Block-comment nesting depth (Rust nests them).
    let mut block_depth = 0usize;
    let mut i = 0usize;
    // Brace depth in Normal mode.
    let mut depth: i64 = 0;
    // Open test regions: (depth before the region's `{`, start line).
    let mut open_regions: Vec<(i64, usize)> = Vec::new();
    // A test-context attribute was just seen; the next `{` (region) or
    // `;` (file-level mod decl) resolves it.
    let mut test_attr_pending = false;
    // Normal-mode text since the pending attribute closed, with
    // whitespace collapsed (bounded — `pub mod name` is short).
    let mut since_attr = String::new();

    while i < chars.len() {
        let c = chars[i];
        match mode {
            Mode::LineComment => {
                if c == '\n' {
                    mode = Mode::Normal;
                }
                stripped.push(if c == '\n' { '\n' } else { ' ' });
            }
            Mode::BlockComment => {
                if c == '/' && chars.get(i + 1) == Some(&'*') {
                    block_depth += 1;
                    stripped.push(' ');
                    stripped.push(' ');
                    i += 2;
                    continue;
                }
                if c == '*' && chars.get(i + 1) == Some(&'/') {
                    block_depth -= 1;
                    stripped.push(' ');
                    stripped.push(' ');
                    i += 2;
                    if block_depth == 0 {
                        mode = Mode::Normal;
                    }
                    continue;
                }
                stripped.push(if c == '\n' { '\n' } else { ' ' });
            }
            Mode::Str => {
                if c == '\\' && i + 1 < chars.len() {
                    stripped.push(' ');
                    let next = chars[i + 1];
                    // A string-continuation newline must stay a
                    // newline (line alignment).
                    stripped.push(if next == '\n' { '\n' } else { ' ' });
                    i += 2;
                    continue;
                }
                if c == '"' {
                    mode = Mode::Normal;
                }
                stripped.push(if c == '\n' { '\n' } else { ' ' });
            }
            Mode::Char => {
                if c == '\\' && i + 1 < chars.len() {
                    stripped.push(' ');
                    stripped.push(' ');
                    i += 2;
                    continue;
                }
                if c == '\'' {
                    mode = Mode::Normal;
                }
                stripped.push(' ');
            }
            Mode::RawStr(hashes) => {
                if c == '"' {
                    let mut following = 0usize;
                    while following < hashes && chars.get(i + 1 + following) == Some(&'#') {
                        following += 1;
                    }
                    if following == hashes {
                        // The closing quote plus its `#`s.
                        for _ in 0..=hashes {
                            stripped.push(' ');
                        }
                        i += 1 + hashes;
                        mode = Mode::Normal;
                        continue;
                    }
                }
                stripped.push(if c == '\n' { '\n' } else { ' ' });
            }
            Mode::Normal => {
                // Comments.
                if c == '/' && chars.get(i + 1) == Some(&'/') {
                    stripped.push(' ');
                    stripped.push(' ');
                    i += 2;
                    mode = Mode::LineComment;
                    continue;
                }
                if c == '/' && chars.get(i + 1) == Some(&'*') {
                    stripped.push(' ');
                    stripped.push(' ');
                    i += 2;
                    block_depth = 1;
                    mode = Mode::BlockComment;
                    continue;
                }
                // Strings.
                if c == '"' {
                    stripped.push(' ');
                    mode = Mode::Str;
                    i += 1;
                    continue;
                }
                // Raw strings: `r"…"` / `r#"…"#` (not an identifier
                // ending in `r`).
                if c == 'r' && looks_like_raw_string(&chars, i) {
                    let mut j = i + 1;
                    let mut hashes = 0usize;
                    while chars.get(j) == Some(&'#') {
                        hashes += 1;
                        j += 1;
                    }
                    // chars[j] == '"'
                    for _ in i..=j {
                        stripped.push(' ');
                    }
                    mode = Mode::RawStr(hashes);
                    i = j + 1;
                    continue;
                }
                // Char literals vs lifetimes: `'x'`, `'\n'`, `'\u{1F}'`
                // are chars; `'a` followed by a non-quote is a
                // lifetime and stays a normal char.
                if c == '\'' && char_literal_len(&chars, i).is_some() {
                    stripped.push(' ');
                    mode = Mode::Char;
                    i += 1;
                    continue;
                }
                // Attributes: `#[…]` / `#![…]`.
                if c == '#' {
                    let mut j = i + 1;
                    let mut inner = false;
                    if chars.get(j) == Some(&'!') {
                        inner = true;
                        j += 1;
                    }
                    if chars.get(j) == Some(&'[') {
                        if let Some((text, span)) = capture_attribute(&chars, j) {
                            if is_test_attribute(&text) {
                                if inner {
                                    whole_file_test = true;
                                } else {
                                    test_attr_pending = true;
                                    since_attr.clear();
                                }
                            }
                            // Blank `#`, the optional `!`, and the
                            // whole `[…]` span (char-aligned).
                            let total = 1 + usize::from(inner) + span;
                            for _ in 0..total {
                                stripped.push(' ');
                            }
                            i += total;
                            continue;
                        }
                    }
                    stripped.push('#');
                    i += 1;
                    continue;
                }
                // Braces: track depth; close any test region that
                // returns to its opening depth.
                if c == '{' {
                    if test_attr_pending {
                        let line = 1 + stripped.matches('\n').count();
                        open_regions.push((depth, line));
                        test_attr_pending = false;
                        since_attr.clear();
                    }
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if open_regions
                        .last()
                        .is_some_and(|(open_depth, _)| *open_depth == depth)
                    {
                        if let Some((_, start)) = open_regions.pop() {
                            let end = 1 + stripped.matches('\n').count();
                            test_regions.push((start, end));
                        }
                    }
                } else if c == ';' && test_attr_pending {
                    // A pending test attribute resolved without a
                    // body: a file-level `mod name;` pulls a whole
                    // file into test context.
                    let tokens = since_attr.split_whitespace().collect::<Vec<_>>();
                    if tokens.first() == Some(&"mod") && tokens.len() >= 2 {
                        resolve_mod_file(file, tokens[1], &mut mod_files);
                    }
                    test_attr_pending = false;
                    since_attr.clear();
                } else if test_attr_pending {
                    if c.is_whitespace() {
                        if !since_attr.is_empty() && !since_attr.ends_with(' ') {
                            since_attr.push(' ');
                        }
                    } else if since_attr.len() < 96 {
                        since_attr.push(c);
                    }
                }
                stripped.push(c);
            }
        }
        i += 1;
    }
    FileScan {
        stripped,
        raw_lines: src.lines().map(str::to_string).collect(),
        test_regions,
        mod_files,
        whole_file_test,
    }
}

/// Whether `chars[at] == 'r'` begins a raw string (`r"` or `r#"`).
fn looks_like_raw_string(chars: &[char], at: usize) -> bool {
    let mut j = at + 1;
    let mut hashes = 0;
    while chars.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    // An identifier character before the `r` means this `r` is the
    // tail of a name, not a raw-string prefix.
    hashes <= 255
        && chars.get(j) == Some(&'"')
        && !at
            .checked_sub(1)
            .and_then(|prev| chars.get(prev))
            .is_some_and(|prev| prev.is_ascii_alphanumeric() || *prev == '_')
}

/// The length (in chars, including both quotes) of a char literal
/// starting at `chars[at] == '\''`, or `None` when it is a lifetime.
fn char_literal_len(chars: &[char], at: usize) -> Option<usize> {
    let next = chars.get(at + 1)?;
    if *next == '\\' {
        // Escape: find the closing quote within a sane bound.
        let mut j = at + 2;
        while j < chars.len() && j <= at + 12 {
            if chars[j] == '\'' {
                return Some(j - at + 1);
            }
            j += 1;
        }
        None
    } else {
        // A single non-quote char followed by a quote.
        match chars.get(at + 2) {
            Some('\'') if *next != '\'' => Some(3),
            _ => None,
        }
    }
}

/// Capture the `[…]` attribute starting at `chars[at] == '['`,
/// returning its inner text and the span's char length (both
/// brackets included). Strings inside the attribute are kept verbatim
/// (they never carry brackets in the attribute shapes this check
/// parses).
fn capture_attribute(chars: &[char], at: usize) -> Option<(String, usize)> {
    let mut bracket_depth = 0i64;
    let mut text = String::new();
    let mut span = 0usize;
    let mut j = at;
    while j < chars.len() {
        let c = chars[j];
        span += 1;
        if c == '"' {
            // Skip string contents verbatim.
            text.push(c);
            j += 1;
            while j < chars.len() {
                let s = chars[j];
                text.push(s);
                span += 1;
                if s == '\\' && j + 1 < chars.len() {
                    text.push(chars[j + 1]);
                    span += 1;
                    j += 2;
                    continue;
                }
                j += 1;
                if s == '"' {
                    break;
                }
            }
            continue;
        }
        if c == '[' {
            bracket_depth += 1;
            if bracket_depth == 1 {
                j += 1;
                continue;
            }
        } else if c == ']' {
            bracket_depth -= 1;
            if bracket_depth == 0 {
                return Some((text, span));
            }
        }
        text.push(c);
        j += 1;
    }
    None
}

/// Whether an attribute's inner text denotes test context:
/// `test`, `cfg(test)`, `cfg(any(test, …))`, `cfg(all(…, test, …))`.
/// `cfg(not(test))` is deliberately NOT test context (it names release
/// code), and `test` inside a string literal (e.g. a feature named
/// `"integration-tests"`) does not count — the predicate is parsed,
/// not substring-matched.
fn is_test_attribute(text: &str) -> bool {
    let text = text.trim();
    if text == "test" {
        return true;
    }
    let Some(rest) = text.strip_prefix("cfg") else {
        return false;
    };
    let rest = rest.trim();
    let Some(inner) = rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
        return false;
    };
    let inner = inner.trim();
    if inner == "test" {
        return true;
    }
    // any(…) / all(…): `test` as a DIRECT child predicate.
    for combinator in ["any", "all"] {
        let Some(body) = inner
            .strip_prefix(combinator)
            .and_then(|r| r.trim().strip_prefix('('))
            .and_then(|r| r.trim().strip_suffix(')'))
        else {
            continue;
        };
        let mut child_depth = 0i64;
        let mut token = String::new();
        let mut found = false;
        for c in body.chars() {
            match c {
                '(' => {
                    child_depth += 1;
                    token.push(c);
                }
                ')' => {
                    child_depth -= 1;
                    token.push(c);
                }
                ',' if child_depth == 0 => {
                    if token.trim() == "test" {
                        found = true;
                        break;
                    }
                    token.clear();
                }
                _ => token.push(c),
            }
        }
        if token.trim() == "test" {
            found = true;
        }
        if found {
            return true;
        }
    }
    false
}

/// Resolve the file(s) a `mod name;` declaration in `file` pulls in
/// (Rust 2018: `<dir>/<name>.rs` or `<dir>/<name>/mod.rs`, where the
/// directory depends on whether `file` is a `mod.rs`/`lib.rs`).
fn resolve_mod_file(file: &Path, name: &str, out: &mut Vec<PathBuf>) {
    let dir = match file.file_name().and_then(|n| n.to_str()) {
        Some("lib.rs") | Some("main.rs") | Some("mod.rs") => file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
        _ => {
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            file.parent()
                .map(|p| p.join(stem))
                .unwrap_or_else(|| PathBuf::from("."))
        }
    };
    for candidate in [dir.join(format!("{name}.rs")), dir.join(name).join("mod.rs")] {
        if candidate.is_file() && !out.contains(&candidate) {
            out.push(candidate);
        }
    }
}

/// Collect `.rs` files under `dir` recursively (sorted for stable
/// reports; the check_architecture precedent).
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

/// Walk up from `start` to the directory whose Cargo.toml declares a
/// `[workspace]` (the check_architecture precedent).
fn find_workspace_root(start: &Path) -> Result<PathBuf, XtaskError> {
    let mut dir: Option<&Path> = Some(start);
    while let Some(d) = dir {
        let manifest = d.join("Cargo.toml");
        if manifest.is_file() {
            match std::fs::read_to_string(&manifest) {
                Ok(contents) => {
                    if contents.lines().any(|line| line.starts_with("[workspace]")) {
                        return Ok(d.to_path_buf());
                    }
                }
                Err(e) => {
                    return Err(XtaskError::io(format!("reading {}", manifest.display()), e));
                }
            }
        }
        dir = d.parent();
    }
    Err(XtaskError::internal(format!(
        "no Cargo.toml with [workspace] found in {} or any parent — \
         run `cargo xtask check-panics` from inside the Lumen workspace",
        start.display()
    )))
}

// ---------------------------------------------------------------------------
// Match finding
// ---------------------------------------------------------------------------

/// Find every panic-family form in the stripped source.
fn find_matches(stripped: &str) -> Vec<Match> {
    let chars: Vec<char> = stripped.chars().collect();
    let mut matches: Vec<Match> = Vec::new();
    let mut line = 1usize;
    let mut line_start = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '\n' {
            line += 1;
            i += 1;
            line_start = i;
            continue;
        }
        let mut matched: Option<(&'static str, usize)> = None;
        for form in FORMS {
            let f: Vec<char> = form.chars().collect();
            if chars[i..].starts_with(&f[..]) {
                if form.starts_with('.') {
                    // Method form: the dot IS the delimiter.
                    matched = Some((form, f.len()));
                } else {
                    // Macro form: must start a token (not `my_panic!(`
                    // and not the tail of another identifier/macro).
                    let boundary_ok = i
                        .checked_sub(1)
                        .and_then(|p| chars.get(p))
                        .is_none_or(|p| {
                            !(p.is_ascii_alphanumeric() || *p == '_' || *p == '!' || *p == '\'')
                        });
                    if boundary_ok {
                        matched = Some((form, f.len()));
                    }
                }
                if matched.is_some() {
                    break;
                }
            }
        }
        if let Some((form, len)) = matched {
            let snippet: String = chars[line_start..]
                .iter()
                .take_while(|c| **c != '\n')
                .collect::<String>()
                .trim()
                .chars()
                .take(96)
                .collect();
            matches.push(Match { line, form, snippet });
            i += len;
            continue;
        }
        i += 1;
    }
    matches
}

// ---------------------------------------------------------------------------
// Unit tests (in-crate; the fixture-based end-to-end proof lives in
// tools/xtask/tests/check_panics.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_shapes() {
        assert!(is_test_attribute("test"));
        assert!(is_test_attribute("cfg(test)"));
        assert!(is_test_attribute("cfg( test )"));
        assert!(is_test_attribute("cfg(any(test, feature = \"contract\"))"));
        assert!(is_test_attribute("cfg(all(target_os = \"linux\", test))"));
        // Release-context shapes are NOT test context.
        assert!(!is_test_attribute("cfg(not(test))"));
        assert!(!is_test_attribute("cfg(feature = \"contract\")"));
        assert!(!is_test_attribute("cfg(unix)"));
        assert!(!is_test_attribute("cfg(feature = \"integration-tests\")"));
        assert!(!is_test_attribute("track_caller"));
        assert!(!is_test_attribute("allow(dead_code)"));
    }

    #[test]
    fn stripping_removes_doc_examples_and_strings() {
        let src = "/// doc with unreachable! and .unwrap(\n\
                   let s = \"panic!( .expect( \";\n\
                   let real = 1;\n";
        let scan = scan_source(src, Path::new("crates/x/src/lib.rs"));
        assert!(find_matches(&scan.stripped).is_empty());
    }

    #[test]
    fn cfg_test_regions_cover_their_items() {
        let src = "\
fn production() -> u8 { 1 }
#[cfg(test)]
mod tests {
    fn helper() -> u8 { let v = Some(1).unwrap(); v }
    #[test]
    fn t() { assert_eq!(helper(), 1); }
}
fn more_production() -> u8 { unreachable_seed() }
";
        let scan = scan_source(src, Path::new("crates/x/src/lib.rs"));
        assert_eq!(scan.test_regions.len(), 1, "the cfg(test) mod region");
        let ms = find_matches(&scan.stripped);
        assert_eq!(ms.len(), 1, "only the unwrap inside the region");
        assert_eq!(ms[0].line, 5);
        assert!(ms[0].snippet.contains(".unwrap()"));
    }

    #[test]
    fn file_level_test_mod_marks_the_file() {
        // lib.rs declares the mod; tests.rs is resolved as test-only.
        let dir = std::env::temp_dir()
            .join(format!("check-panics-unit-{}", std::process::id()));
        let lib = dir.join("src").join("lib.rs");
        let tests = dir.join("src").join("tests.rs");
        std::fs::create_dir_all(dir.join("src")).must();
        std::fs::write(&lib, "#[cfg(test)]\nmod tests;\n").must();
        std::fs::write(&tests, "fn f() { let _ = unreachable!(); }").must();
        let scan = scan_source(
            "#[cfg(test)]\nmod tests;\n",
            &lib,
        );
        assert_eq!(scan.mod_files, vec![tests.clone()]);
        // And the referenced file's own matches are whole-file test
        // context (the classifier's is_test_mod_file path).
        let scan2 = scan_source("fn f() { let _ = unreachable!(); }", &tests);
        assert_eq!(find_matches(&scan2.stripped).len(), 1);
        std::fs::remove_dir_all(&dir).must();
    }

    #[test]
    fn pub_cfg_test_mod_declaration_is_recognized() {
        let src = "#[cfg(test)]\npub mod tests {\n    fn f() {}\n}\n";
        let scan = scan_source(src, Path::new("crates/x/src/lib.rs"));
        assert_eq!(scan.test_regions.len(), 1);
        assert_eq!(scan.test_regions[0], (2, 4));
    }

    #[test]
    fn inner_cfg_test_whole_file() {
        let src = "#![cfg(test)]\nfn f() { let _ = panic!(); }\n";
        let scan = scan_source(src, Path::new("crates/x/src/lib.rs"));
        assert!(scan.whole_file_test);
    }

    #[test]
    fn macro_forms_need_token_boundaries() {
        let src = "let a = my_panic!(1);\nlet b = Option::unwrap(x);\n";
        let scan = scan_source(src, Path::new("crates/x/src/lib.rs"));
        assert!(find_matches(&scan.stripped).is_empty());
    }

    #[test]
    fn raw_strings_and_lifetimes_do_not_confuse_the_scanner() {
        let src = "let s = r#\".unwrap( unreachable!( \"#;\n\
                   fn f<'a>(x: &'a u8) -> &'a u8 { x }\n";
        let scan = scan_source(src, Path::new("crates/x/src/lib.rs"));
        assert!(find_matches(&scan.stripped).is_empty());
    }

    /// testkit's Must, locally bound so the assertions stay one line.
    trait Must {
        type Value;
        fn must(self) -> Self::Value;
    }
    impl<T, E: std::fmt::Display> Must for Result<T, E> {
        type Value = T;
        fn must(self) -> T {
            match self {
                Ok(v) => v,
                Err(e) => unreachable!("{e}"),
            }
        }
    }
}
