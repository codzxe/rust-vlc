//! The baselines consumer (card W1-14; audit finding M15's cure:
//! "docs/baselines VLC data never consumed by any test").
//!
//! ONE test that makes `docs/baselines/vlc-3.0.23.json` — the pinned
//! external-oracle record (VLC 3.0.23 Vetinari, the differential
//! reference engine per doc 02 §2.4) — **load-bearing**: a set of
//! consistency rows, each of which FAILS if either side drifts.
//!
//! The two sides are never the same artifact:
//!
//! * the consolidated record — `docs/baselines/vlc-3.0.23.json`
//!   (authored by task 0.A4 from the three platform pin sessions), and
//! * the repo's reality — the *verbatim* raw dumps the pin sessions
//!   committed under `docs/baselines/raw/linux/` (independent
//!   recordings of the same oracle), plus the actual tree (recorded
//!   paths) and the committed first-vertical fixture set.
//!
//! Nothing here re-runs VLC (the pinned debs are not part of the tree);
//! the rows instead pin the *identity* of the oracle — version string,
//! module census, capability rows, evidence-path ledger — so that any
//! future re-pin or accidental edit must touch both sides consistently
//! or this test goes red. That is the D-5 posture: the baseline is an
//! external-oracle contract, and a contract nobody checks is a rumor.
//!
//! The rows, in audit order:
//!
//! | row group | JSON side | repo-reality side | drift caught |
//! |-----------|-----------|-------------------|--------------|
//! | version identity | `version_guard` + `platforms.linux.expected_version_string` + `run_results["vlc --version"]` | first output line + exit marker of `raw/linux/04-vlc-version.txt` | a version re-pin edited one side only |
//! | module census | `run_results["vlc --list" / "vlc --list-verbose"].modules` | non-comment line count + exit marker of `raw/linux/05-vlc-list.txt` | census rows edited on one side only |
//! | oracle capability | (census identity: the dump the JSON's 502-row counts) | `raw/linux/05-vlc-list.txt` contains the demuxer/decoder modules for the formats this repo's committed fixtures and the D-018 first vertical (AVI/MP4 + MJPEG + PCM) actually use | an oracle re-pin that can no longer serve as differential reference for our formats |
//! | evidence ledger | `record_paths` + `platforms.*.dumps.available` | the files exist in the tree | recorded evidence moved/deleted without updating the record |
//!
//! Deliberately NOT asserted: the literal version digits ("3.0.23").
//! A consistent re-pin (JSON + both dumps together) stays green; a
//! one-sided edit fails. The rows test *agreement*, not a frozen
//! constant — the frozen bytes live in the evidence, not in test code.
//!
//! Std-only by C5: the JSON reader below is the crate's own writer's
//! reading twin (the `src/json.rs` 2.E1 precedent — the repo hand-rolls
//! its data formats; no external dependency is admitted for a test).

use testkit::Must;

/// The consolidated pin record, embedded at compile time: cargo tracks
/// the file, so editing the JSON recompiles and re-runs this test —
/// the record can never silently stop being consumed.
const BASELINE: &str = include_str!("../../../../docs/baselines/vlc-3.0.23.json");

/// The verbatim version dump (raw/linux/04): pinned oracle's own
/// `--version` output, captured 2026-10-19 by 0.A2.
const VERSION_DUMP: &str = include_str!("../../../../docs/baselines/raw/linux/04-vlc-version.txt");

/// The verbatim terse module census (raw/linux/05): one line per VLC
/// module, `vlc --list`, captured in the same session.
const LIST_DUMP: &str = include_str!("../../../../docs/baselines/raw/linux/05-vlc-list.txt");

// ---------------------------------------------------------------------------
// A minimal std-only JSON reader (the 2.E1 writer's reading twin)
// ---------------------------------------------------------------------------

/// One JSON value — exactly the subset the baseline record uses
/// (objects, arrays, strings, integers, booleans, null; no floats).
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// This value's field `name`, if it is an object carrying it.
    fn field(&self, name: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(key, _)| key == name).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string content, if this is a string.
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(text) => Some(text),
            _ => None,
        }
    }

    /// The integer, if this is an integer.
    fn as_int(&self) -> Option<i64> {
        match self {
            Json::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// The object's (key, value) pairs, if this is an object.
    fn as_obj(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(fields) => Some(fields),
            _ => None,
        }
    }
}

/// Nesting cap: the baseline's deepest path is six levels; a committed
/// record that nests deeper than this is not a shape this reader knows
/// (bounded work on untrusted-shaped input, the D-1 posture).
const MAX_DEPTH: usize = 64;

/// Byte-oriented recursive-descent reader. UTF-8-safe by construction:
/// multi-byte sequences only ever appear inside strings, where every
/// non-ASCII byte (≥ 0x80) is copied verbatim — structural bytes are
/// all ASCII.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

/// Parse a complete JSON document (integers only; a float anywhere is
/// a shape this consumer rejects loudly rather than rounding).
fn parse_json(text: &str) -> Result<Json, String> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        pos: 0,
    };
    let value = reader.value(0)?;
    reader.skip_ws();
    if reader.pos != reader.bytes.len() {
        return Err(format!("trailing data at byte {}", reader.pos));
    }
    Ok(value)
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected {:?} at byte {}", byte as char, self.pos))
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), String> {
        let want = word.as_bytes();
        if self.pos + want.len() > self.bytes.len() {
            return Err(format!("truncated literal at byte {}", self.pos));
        }
        if &self.bytes[self.pos..self.pos + want.len()] == want {
            self.pos += want.len();
            Ok(())
        } else {
            Err(format!("expected literal {word:?} at byte {}", self.pos))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err(format!("nesting deeper than {MAX_DEPTH}"));
        }
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => {
                self.literal("true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.literal("false")?;
                Ok(Json::Bool(false))
            }
            Some(b'n') => {
                self.literal("null")?;
                Ok(Json::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            other => Err(format!("unexpected {other:?} at byte {}", self.pos)),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut fields = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(fields));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.value(depth + 1)?;
            fields.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Obj(fields));
                }
                other => {
                    return Err(format!(
                        "expected ',' or '}}' in object, got {other:?} at byte {}",
                        self.pos
                    ));
                }
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Arr(items));
                }
                other => {
                    return Err(format!(
                        "expected ',' or ']' in array, got {other:?} at byte {}",
                        self.pos
                    ));
                }
            }
        }
    }

    /// One string's contents; assumes the cursor sits on the opening
    /// quote (verified, then consumed).
    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        loop {
            let byte = self.peek().ok_or("unterminated string")?;
            self.pos += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(out)
                        .map_err(|_| "string is not valid UTF-8".to_string());
                }
                b'\\' => {
                    let escape = self.peek().ok_or("truncated escape")?;
                    self.pos += 1;
                    match escape {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0C),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let ch = self.unicode_escape()?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => {
                            return Err(format!("invalid escape \\{}", other as char));
                        }
                    }
                }
                byte if byte < 0x20 => {
                    return Err(format!("control byte {byte:#04x} in string"));
                }
                byte => out.push(byte),
            }
        }
    }

    /// A `\uXXXX` escape (cursor after the `u`), with surrogate-pair
    /// combining.
    fn unicode_escape(&mut self) -> Result<char, String> {
        let first = self.hex4()?;
        let code = u32::from(first);
        if (0xD800..=0xDBFF).contains(&code) {
            if self.peek() == Some(b'\\') {
                self.pos += 1;
            } else {
                return Err("lone high surrogate".to_string());
            }
            if self.peek() == Some(b'u') {
                self.pos += 1;
            } else {
                return Err("high surrogate not followed by \\u".to_string());
            }
            let second = self.hex4()?;
            let low = u32::from(second);
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err(format!("invalid low surrogate {low:#x}"));
            }
            let combined = 0x1_0000 + ((code - 0xD800) << 10) + (low - 0xDC00);
            char::from_u32(combined).ok_or("surrogate pair out of range".to_string())
        } else if (0xDC00..=0xDFFF).contains(&code) {
            Err(format!("lone low surrogate {code:#x}"))
        } else {
            char::from_u32(code).ok_or(format!("code point {code:#x} is not a char"))
        }
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let digit = self.peek().ok_or("truncated \\u escape")?;
            self.pos += 1;
            let nibble = match digit {
                b'0'..=b'9' => u16::from(digit - b'0'),
                b'a'..=b'f' => u16::from(digit - b'a' + 10),
                b'A'..=b'F' => u16::from(digit - b'A' + 10),
                other => {
                    return Err(format!("non-hex byte in \\u escape: {:?}", other as char));
                }
            };
            value = value * 16 + nibble;
        }
        Ok(value)
    }

    /// An integer (optional `-`, digits). A decimal point or exponent
    /// is rejected: this consumer's contract is integer-only, loudly.
    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == digits_start {
            return Err(format!("no digits in number at byte {start}"));
        }
        if self.bytes[digits_start] == b'0' && self.pos > digits_start + 1 {
            return Err(format!("leading zero in number at byte {start}"));
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(format!(
                "non-integer number at byte {start} (this consumer is integer-only)"
            ));
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "number is not UTF-8".to_string())?;
        text.parse::<i64>()
            .map(Json::Int)
            .map_err(|_| format!("integer out of i64 range at byte {start}"))
    }
}

// ---------------------------------------------------------------------------
// Row helpers over the parsed record
// ---------------------------------------------------------------------------

/// Walk a `field.field.field` path; every hop must exist.
fn get<'a>(root: &'a Json, path: &[&str]) -> Result<&'a Json, String> {
    let mut node = root;
    for name in path {
        node = node
            .field(name)
            .ok_or_else(|| format!("baseline is missing field `{name}` (path {path:?})"))?;
    }
    Ok(node)
}

/// A required string field.
fn string_at<'a>(root: &'a Json, path: &[&str]) -> Result<&'a str, String> {
    get(root, path)?
        .as_str()
        .ok_or_else(|| format!("field at {path:?} is not a string"))
}

/// A required integer field.
fn int_at(root: &Json, path: &[&str]) -> Result<i64, String> {
    get(root, path)?
        .as_int()
        .ok_or_else(|| format!("field at {path:?} is not an integer"))
}

/// The pinned oracle's observed `--version` line from the verbatim
/// dump: the first non-comment, non-blank line (the dump's comment
/// header and its `# $ $ …` invocation echo are all `#` lines, so the
/// first output line is exactly this).
fn observed_version_line() -> Result<&'static str, String> {
    VERSION_DUMP
        .lines()
        .find(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .ok_or_else(|| "version dump carries no output line".to_string())
}

/// The dump's recorded exit marker (`# exit=<code>`), if present.
fn dump_exit_code(dump: &str) -> Result<i64, String> {
    for line in dump.lines().rev() {
        if let Some(rest) = line.trim().strip_prefix("# exit=") {
            return rest
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("malformed exit marker: {rest:?}"));
        }
    }
    Err("dump records no `# exit=` marker".to_string())
}

/// The module census from the verbatim `--list` dump: one `(name,
/// description)` row per non-comment, non-blank line (the dump's own
/// header documents "one line per module").
fn census_rows() -> Result<Vec<(String, String)>, String> {
    let mut rows = Vec::new();
    for line in LIST_DUMP.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        let (name, description) = trimmed
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("census line is not `name  description`: {trimmed:?}"))?;
        rows.push((name.to_string(), description.trim().to_string()));
    }
    if rows.is_empty() {
        return Err("module census dump carries no rows".to_string());
    }
    Ok(rows)
}

/// Does the census contain a module row named `name` whose description
/// mentions `role` ("demuxer" / "decoder")?
fn census_has(name: &str, role: &str) -> Result<bool, String> {
    let rows = census_rows()?;
    Ok(rows
        .iter()
        .any(|(module, description)| module == name && description.contains(role)))
}

// ---------------------------------------------------------------------------
// The one test: every row group must hold
// ---------------------------------------------------------------------------

/// Row group 1 — version identity: the JSON's four recordings of the
/// oracle version string must all agree with each other AND with the
/// verbatim dump's first output line, the dump must record the same
/// exit code the JSON claims, and the line must have the shape
/// `VLC version <baseline> <codename> (<git_describe>)`.
fn row_version_identity(baseline: &Json) {
    let version = string_at(baseline, &["platforms", "linux", "expected_version_string"]).must();
    let guard_line = string_at(baseline, &["version_guard", "linux_observed_first_line"]).must();
    let run_line = string_at(
        baseline,
        &[
            "platforms",
            "linux",
            "run_results",
            "vlc --version",
            "first_line",
        ],
    )
    .must();
    assert_eq!(
        version, guard_line,
        "row: linux.expected_version_string == version_guard.linux_observed_first_line"
    );
    assert_eq!(
        version, run_line,
        "row: expected_version_string == run_results[\"vlc --version\"].first_line"
    );

    let baseline_version = string_at(baseline, &["baseline"]).must();
    let codename = string_at(baseline, &["codename"]).must();
    let git_describe = string_at(baseline, &["git_describe"]).must();
    let cross_platform = string_at(baseline, &["version_guard", "cross_platform_assert"]).must();
    assert_eq!(
        cross_platform,
        format!("{baseline_version} {codename}"),
        "row: cross_platform_assert == \"<baseline> <codename>\" (internal composition)"
    );
    assert_eq!(
        version,
        format!("VLC version {baseline_version} {codename} ({git_describe})"),
        "row: the observed line is structurally `VLC version <baseline> <codename> (<git_describe>)`"
    );

    let observed = observed_version_line().must();
    assert_eq!(
        version, observed,
        "row: JSON version string == verbatim first output line of raw/linux/04"
    );
    let dump_exit = dump_exit_code(VERSION_DUMP).must();
    let json_exit = int_at(
        baseline,
        &[
            "platforms",
            "linux",
            "run_results",
            "vlc --version",
            "exit_code",
        ],
    )
    .must();
    assert_eq!(
        dump_exit, json_exit,
        "row: raw dump 04 exit marker == run_results[\"vlc --version\"].exit_code"
    );
    assert_eq!(
        json_exit, 0,
        "row: the pinned oracle's --version run exited 0"
    );
}

/// Row group 2 — module census: the JSON's two census numbers must
/// agree with each other and with the verbatim dump's own line count,
/// and the dump's exit marker must match the JSON's exit code.
fn row_module_census(baseline: &Json) {
    let terse = int_at(
        baseline,
        &["platforms", "linux", "run_results", "vlc --list", "modules"],
    )
    .must();
    let verbose = int_at(
        baseline,
        &[
            "platforms",
            "linux",
            "run_results",
            "vlc --list-verbose",
            "modules",
        ],
    )
    .must();
    assert_eq!(
        terse, verbose,
        "row: run_results[\"vlc --list\"].modules == run_results[\"vlc --list-verbose\"].modules"
    );

    let counted = census_rows().must().len() as i64;
    assert_eq!(
        terse, counted,
        "row: run_results[\"vlc --list\"].modules == non-comment line count of raw/linux/05"
    );

    let dump_exit = dump_exit_code(LIST_DUMP).must();
    let json_exit = int_at(
        baseline,
        &[
            "platforms",
            "linux",
            "run_results",
            "vlc --list",
            "exit_code",
        ],
    )
    .must();
    assert_eq!(
        dump_exit, json_exit,
        "row: raw dump 05 exit marker == run_results[\"vlc --list\"].exit_code"
    );
    assert_eq!(json_exit, 0, "row: the pinned oracle's --list run exited 0");
}

/// Row group 3 — oracle capability for this repo's media: the pinned
/// census must carry the modules that let VLC serve as the
/// differential oracle for (a) every container family present in the
/// committed valid-fixture set, and (b) the D-018 first vertical's
/// codec pair (MJPEG video — served through VLC's `avcodec`, the
/// selection observed in raw/linux/17's avi_mjpeg probe; raw PCM
/// audio — VLC's `araw`). If a future re-pin loses these rows, or a
/// fixture lands in a container family the pinned oracle cannot
/// demux, this row fails: the baseline would no longer be a usable
/// external reference for the media this repo actually ships.
fn row_oracle_capability(baseline: &Json) {
    // (a) container families of the committed valid fixtures.
    let fixture_dir = testkit::workspace_root()
        .must()
        .join("tests/fixtures/generated/valid");
    let entries = std::fs::read_dir(&fixture_dir)
        .unwrap_or_else(|error| unreachable!("read {}: {error}", fixture_dir.display()));
    let mut families: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry
            .unwrap_or_else(|error| unreachable!("iterate {}: {error}", fixture_dir.display()));
        let name = entry.file_name().to_string_lossy().into_owned();
        for (extension, family) in [(".avi", "avi"), (".mp4", "mp4")] {
            if name.ends_with(extension) && !families.iter().any(|f| f.as_str() == family) {
                families.push(family.to_string());
            }
        }
    }
    assert!(
        families.len() >= 2,
        "precondition: the committed valid-fixture set covers avi + mp4 (found {families:?})"
    );
    for family in &families {
        assert!(
            census_has(family, "demuxer").must(),
            "row: pinned census carries a `{family}` demuxer (a committed fixture family \
             the differential oracle must be able to demux)"
        );
    }

    // (b) the first vertical's codec pair (D-018: AVI/MP4 + MJPEG + PCM).
    assert!(
        census_has("avcodec", "decoder").must(),
        "row: pinned census carries an `avcodec` decoder row (VLC's MJPEG path, per \
         raw/linux/17's observed avi_mjpeg selection)"
    );
    assert!(
        census_has("araw", "decoder").must(),
        "row: pinned census carries an `araw` decoder row (VLC's raw-PCM path)"
    );

    // And the capability rows live in the SAME dump the JSON's census
    // numbers count (row group 2), so this group is pinned to the
    // record, not to an unrelated file.
    let terse = int_at(
        baseline,
        &["platforms", "linux", "run_results", "vlc --list", "modules"],
    )
    .must();
    assert_eq!(
        terse,
        census_rows().must().len() as i64,
        "row: capability rows come from the dump the JSON census counts"
    );
}

/// Row group 4 — evidence ledger: every path the record claims exists
/// (`record_paths`, and each platform's `dumps.available`) must be a
/// real file in the tree.
fn row_evidence_ledger(baseline: &Json) {
    let root = testkit::workspace_root().must();

    let record_paths = get(baseline, &["record_paths"]).must().as_obj().must();
    assert!(
        record_paths.len() >= 5,
        "precondition: record_paths still names the five pin documents"
    );
    for (key, value) in record_paths {
        let path = value.as_str().must();
        let resolved = root.join(path);
        assert!(
            resolved.exists(),
            "row: record_paths.{key} -> {path} exists in the tree"
        );
    }

    for platform in ["linux", "windows", "macos"] {
        let dumps = get(baseline, &["platforms", platform, "dumps", "available"]).must();
        let entries = dumps.as_obj().must();
        assert!(
            !entries.is_empty(),
            "precondition: platforms.{platform}.dumps.available is non-empty"
        );
        for (key, value) in entries {
            let path = value.as_str().must();
            let resolved = root.join(path);
            assert!(
                resolved.exists(),
                "row: platforms.{platform}.dumps.available.{key} -> {path} exists in the tree"
            );
        }
    }
}

/// The baselines consumer: `docs/baselines/vlc-3.0.23.json` must stay
/// consistent with the verbatim evidence and the tree it records.
/// See the module docs for the row table; each group is a set of
/// two-sided consistency rows that fail on single-sided drift.
#[test]
fn vlc_baseline_rows_stay_consistent_with_repo_reality() {
    let baseline = parse_json(BASELINE).must();
    row_version_identity(&baseline);
    row_module_census(&baseline);
    row_oracle_capability(&baseline);
    row_evidence_ledger(&baseline);
}

// ---------------------------------------------------------------------------
// Reader self-checks: the JSON reader is a parser, and a parser that
// never sees a malformed input is an echo chamber of one (D-5).
// ---------------------------------------------------------------------------

/// The reader accepts a representative document of every shape the
/// baseline uses, and rejects the classic malformations.
#[test]
fn json_reader_parses_shapes_and_rejects_malformed() {
    use Json::{Arr, Bool, Int, Null, Obj};

    let value = parse_json(
        r#"{"baseline":"3.0.23","n":502,"ok":true,"none":null,"list":[1,2],"esc":"a\"b\\c\nd\u0041\u00e9\uD83D\uDE00"}"#,
    )
    .must();
    let Obj(fields) = &value else {
        unreachable!("the document is an object");
    };
    assert_eq!(fields.len(), 6);
    assert_eq!(string_at(&value, &["baseline"]).must(), "3.0.23");
    assert_eq!(int_at(&value, &["n"]).must(), 502);
    assert_eq!(get(&value, &["ok"]).must(), &Bool(true));
    assert_eq!(get(&value, &["none"]).must(), &Null);
    assert_eq!(get(&value, &["list"]).must(), &Arr(vec![Int(1), Int(2)]));
    assert_eq!(
        string_at(&value, &["esc"]).must(),
        "a\"b\\c\ndA\u{e9}\u{1f600}",
        "escapes (incl. \\uXXXX surrogate pairs) decode"
    );
    assert!(get(&value, &["missing"]).is_err());

    for bad in [
        "{\"a\":}",       // value missing
        "{\"a\" 1}",      // colon missing
        "[1,]",           // trailing comma
        "{\"a\":1,}",     // trailing comma (object)
        "{\"a\":0.5}",    // float: this consumer is integer-only
        "{\"a\":01}",     // leading zero (parsed as 0 then junk `1`)
        "\"unterminated", // no closing quote (truncated document)
    ] {
        assert!(
            parse_json(bad).is_err(),
            "malformed document must be rejected: {bad:?}"
        );
    }
}
