//! The soak SCENARIO suite (card 8.C1, issue #54 / P8-3): the
//! config-driven tier above the 3.E4 soak loop — five scenario kinds
//! selected by a TOML manifest, run through the SAME command-layer
//! seam (`lumen_soak_dispatch` over a [`SoakHost`], ARCH-5) against
//! the committed `tests/fixtures/` media.
//!
//! # The three files, one code path
//!
//! The manifest selects everything (durations, cycle counts, targets,
//! budgets, fixtures); the CI-short default, the 4 h nightly tier and
//! the bounded sandbox profile are three FILES, not three code paths:
//!
//! | profile | manifest | who runs it |
//! |---|---|---|
//! | CI-short (merge gate) | `tests/soak/scenarios.toml` (the default) | `cargo test --workspace` |
//! | nightly 4 h | `tests/soak/scenarios-nightly.toml` | `.github/workflows/soak-nightly.yml` |
//! | bounded sandbox | `tests/soak/scenarios-sandbox.toml` | the real in-sandbox measurement (docs/soak/) |
//!
//! # Configuration (env, the `tests/soak/main.rs` discipline)
//!
//! | env | default | meaning |
//! |---|---|---|
//! | `SOAK_SCENARIOS` | `tests/soak/scenarios.toml` | the manifest to run: a workspace-relative or absolute path. Parse/refusal errors fail loudly, never default (the `SOAK_MINUTES` dirty-world rule). |
//! | `SOAK_SCENARIOS_REPORT` | (unset) | write the full markdown suite report to this path |
//! | `SOAK_SCENARIOS_ONLY` | (unset) | run ONLY the scenario with this id (the sandbox chunking knob: this sandbox's tool calls are wall-capped below the bounded profile's total, so the sandbox measurement runs the five scenarios as five processes, one per scenario, each a clean allocator baseline; CI and the nightly never set it). A set-but-unknown id is a loud refusal. |
//!
//! # Test-binary discipline
//!
//! The counting allocator is process-global: every test in this binary
//! takes `serialize_tests()` first (the `tests/soak/main.rs` and
//! `reopen_alloc.rs` precedent), so no sibling test thread allocates
//! inside a measured window. The four `#[path]`-included modules below
//! are the 3.E4 harness's own — shared verbatim, not forked (the
//! runner's module docs record the lineage).
//!
//! # Verdict discipline
//!
//! Budget and liveness violations are RECORDED per scenario (the run
//! continues so the report stays complete for triage) and asserted
//! empty at the end — a red suite is a finding, never noise. A harness
//! break (a command the engine refuses, a terminal that never lands)
//! fails fast: nothing after it is measurable.

mod manifest;
mod report;
mod runner;

// The 3.E4 harness modules, shared verbatim via #[path] (not forked).
// `allow(dead_code)`: this second driver uses a subset of their API —
// the remainder is the `soak` binary's surface, unused here by design
// (the shared file must not grow this binary's private items).
#[path = "../soak/dispatch.rs"]
#[allow(dead_code)]
mod dispatch;
#[path = "../soak/memory.rs"]
#[allow(dead_code)]
mod memory;
#[path = "../soak/scenario.rs"]
#[allow(dead_code)]
mod scenario;

use std::path::{Path, PathBuf};

use testkit::Must;
use testkit::alloc_count::{CountingGlobalAlloc, serialize_tests};

use manifest::{Manifest, ScenarioSpec};
use report::suite_markdown;
use runner::run_scenario;

/// The process-global counting allocator — the live-bytes instrument
/// (ED-FIX-2; the workspace's only allocator instrumentation, the
/// `tests/soak/main.rs` allocator verbatim).
#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

/// The live-bytes reader handed to the runners (a plain fn — sampling
/// allocates nothing beyond the reserved sample slot).
fn live_bytes() -> isize {
    ALLOC.state().snapshot().live_bytes
}

// ---------------------------------------------------------------------------
// Configuration (env-parsed; parse errors fail loudly, never default)
// ---------------------------------------------------------------------------

/// Resolve the manifest path (env `SOAK_SCENARIOS`: absolute, or
/// workspace-relative; default the CI-short profile).
///
/// # Errors
/// A set-but-unusable value (empty) refuses; a missing/unparsable file
/// refuses at load with the path in the sentence.
fn manifest_path() -> Result<PathBuf, String> {
    let configured = std::env::var("SOAK_SCENARIOS").ok();
    match configured.as_deref().map(str::trim) {
        None | Some("") => Ok(testkit::workspace_root()
            .must()
            .join("tests/soak/scenarios.toml")),
        Some(relative) => {
            let path = Path::new(relative);
            if path.is_absolute() {
                Ok(path.to_path_buf())
            } else {
                Ok(testkit::workspace_root().must().join(path))
            }
        }
    }
}

/// The optional report path (env `SOAK_SCENARIOS_REPORT`).
fn report_path() -> Option<PathBuf> {
    std::env::var("SOAK_SCENARIOS_REPORT")
        .ok()
        .filter(|text| !text.trim().is_empty())
        .map(PathBuf::from)
}

/// The optional single-scenario filter (env `SOAK_SCENARIOS_ONLY`).
///
/// # Errors
/// A set-but-empty value refuses (an empty filter is a broken setup,
/// not "all").
fn only_scenario() -> Result<Option<String>, String> {
    match std::env::var("SOAK_SCENARIOS_ONLY").ok() {
        None => Ok(None),
        Some(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(
                    "SOAK_SCENARIOS_ONLY is set but empty (unset it, or name one \
                            scenario id)"
                        .to_string(),
                );
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

/// The scenarios this run executes (the filter applied to the
/// manifest's list).
///
/// # Errors
/// A filter naming no scenario id in the manifest refuses with the
/// legal ids.
fn select_scenarios(manifest: &Manifest) -> Result<Vec<&ScenarioSpec>, String> {
    match only_scenario()? {
        None => Ok(manifest.scenarios.iter().collect()),
        Some(wanted) => match manifest.scenarios.iter().find(|spec| spec.id == wanted) {
            Some(spec) => Ok(vec![spec]),
            None => Err(format!(
                "SOAK_SCENARIOS_ONLY={wanted:?} names no scenario in {} (legal ids: {})",
                manifest.source,
                manifest
                    .scenarios
                    .iter()
                    .map(|spec| spec.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        },
    }
}

// ---------------------------------------------------------------------------
// The suite
// ---------------------------------------------------------------------------

/// THE suite: run the manifest's scenarios in file order against the
/// real engine stack, emit the markdown report, and assert every
/// configured liveness and memory budget held (violations were
/// recorded, not thrown, so the report above is complete for triage).
#[test]
fn soak_scenario_suite() {
    let _serial = serialize_tests();
    let path = manifest_path().must();
    let manifest = manifest::load(&path)
        .unwrap_or_else(|error| unreachable!("soak manifest {}: {error}", path.display()));
    let selected = select_scenarios(&manifest)
        .unwrap_or_else(|error| unreachable!("soak scenario selection: {error}"));

    let mut results = Vec::with_capacity(selected.len());
    for spec in selected {
        println!(
            "\n=== soak scenario {} ({}) — {} ===",
            spec.id,
            spec.kind.label(),
            spec.duration_min.map_or_else(
                || format!("{} cycles", spec.cycles.must()),
                |minutes| { format!("{minutes} min") }
            )
        );
        let started = std::time::Instant::now();
        let result = run_scenario(spec, &manifest.defaults, live_bytes).unwrap_or_else(|error| {
            unreachable!(
                "soak scenario {} — harness break (nothing after this is measurable): {error}",
                spec.id
            )
        });
        println!(
            "scenario {} finished in {:.1} s — {} cycles, {} violations, {} notes",
            result.id,
            started.elapsed().as_secs_f64(),
            result.measured_cycles,
            result.failures.len(),
            result.notes.len()
        );
        results.push(result);
    }

    let markdown = suite_markdown(&manifest, &results);
    if let Some(report) = report_path() {
        std::fs::write(&report, markdown.as_bytes()).unwrap_or_else(|error| {
            unreachable!(
                "SOAK_SCENARIOS_REPORT write to {}: {error}",
                report.display()
            )
        });
    }
    println!("\n{markdown}");

    let failures: Vec<String> = results
        .iter()
        .flat_map(|result| result.failures.iter().cloned())
        .collect();
    assert!(
        failures.is_empty(),
        "soak scenario suite: {} budget/liveness violation(s) across {} scenario(s) — the \
         report above carries every sentence:\n  - {}",
        failures.len(),
        results.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// The shipped-profile validation test (the schema contract, enforced)
// ---------------------------------------------------------------------------

/// The three shipped manifests load through the loader (which enforces
/// the `tests/soak/scenarios.schema.json` contract in Rust: unknown
/// keys, wrong types, out-of-range values, duplicate ids and wrong
/// kind axes are loud refusals), and the schema file itself is present,
/// syntactically valid JSON, and names every key and kind the loader
/// accepts (the drift check — loader and schema move together or not
/// at all).
#[test]
fn shipped_manifests_validate_against_the_schema_contract() {
    let _serial = serialize_tests();
    let root = testkit::workspace_root().must();
    let schema_path = root.join("tests/soak/scenarios.schema.json");
    let schema = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|error| unreachable!("schema {}: {error}", schema_path.display()));
    assert!(
        json_is_syntactically_valid(&schema),
        "tests/soak/scenarios.schema.json is not valid JSON (the human/tool-facing contract \
         itself is broken)"
    );
    for key in manifest::DEFAULTS_KEYS {
        assert!(
            schema.contains(&format!("\"{key}\"")),
            "the schema does not name the [defaults] key {key:?} — loader/schema drift"
        );
    }
    for key in manifest::SCENARIO_KEYS {
        assert!(
            schema.contains(&format!("\"{key}\"")),
            "the schema does not name the [[scenario]] key {key:?} — loader/schema drift"
        );
    }
    for kind in [
        "play-loop",
        "seek-storm",
        "playlist-cycle",
        "open-close-churn",
        "reopen-stress",
    ] {
        assert!(
            schema.contains(&format!("\"{kind}\"")),
            "the schema does not name the kind {kind:?} — loader/schema drift"
        );
    }
    assert!(
        schema.contains("\"const\": 1"),
        "the schema's shape version must stay pinned at 1 (bump loader and schema together)"
    );

    for relative in [
        "tests/soak/scenarios.toml",
        "tests/soak/scenarios-nightly.toml",
        "tests/soak/scenarios-sandbox.toml",
    ] {
        let path = root.join(relative);
        let loaded = manifest::load(&path).unwrap_or_else(|error| {
            unreachable!(
                "{relative}: the shipped profile must validate (the loader enforces the schema \
                 contract): {error}"
            )
        });
        assert!(
            !loaded.scenarios.is_empty(),
            "{relative}: a shipped profile with no scenarios is a ritual, not a measurement (D-6)"
        );
    }
}

// ---------------------------------------------------------------------------
// A minimal JSON syntax validator (test-only; no dependency)
// ---------------------------------------------------------------------------

/// Whether `text` is syntactically valid JSON (RFC 8259 shape:
/// whitespace, one value, end of input). No data extraction — the
/// drift checks above are containment checks by design; this proves
/// the file is parseable at all, which a corrupted half-edit is not.
fn json_is_syntactically_valid(text: &str) -> bool {
    let bytes: Vec<char> = text.chars().collect();
    let mut cursor = 0usize;
    skip_ws(&bytes, &mut cursor);
    if !parse_value(&bytes, &mut cursor) {
        return false;
    }
    skip_ws(&bytes, &mut cursor);
    cursor == bytes.len()
}

/// Skip JSON whitespace.
fn skip_ws(text: &[char], cursor: &mut usize) {
    while *cursor < text.len() && matches!(text[*cursor], ' ' | '\t' | '\n' | '\r') {
        *cursor += 1;
    }
}

/// Parse one JSON value at the cursor (true on success, cursor
/// advanced past it).
fn parse_value(text: &[char], cursor: &mut usize) -> bool {
    if *cursor >= text.len() {
        return false;
    }
    match text[*cursor] {
        '{' => parse_object(text, cursor),
        '[' => parse_array(text, cursor),
        '"' => parse_string(text, cursor),
        't' => parse_literal(text, cursor, "true"),
        'f' => parse_literal(text, cursor, "false"),
        'n' => parse_literal(text, cursor, "null"),
        c if c == '-' || c.is_ascii_digit() => parse_number(text, cursor),
        _ => false,
    }
}

/// One `{ … }` (empty, or `"key": value` comma-separated).
fn parse_object(text: &[char], cursor: &mut usize) -> bool {
    *cursor += 1; // '{'
    skip_ws(text, cursor);
    if *cursor < text.len() && text[*cursor] == '}' {
        *cursor += 1;
        return true;
    }
    loop {
        skip_ws(text, cursor);
        if *cursor >= text.len() || text[*cursor] != '"' {
            return false;
        }
        if !parse_string(text, cursor) {
            return false;
        }
        skip_ws(text, cursor);
        if *cursor >= text.len() || text[*cursor] != ':' {
            return false;
        }
        *cursor += 1;
        skip_ws(text, cursor);
        if !parse_value(text, cursor) {
            return false;
        }
        skip_ws(text, cursor);
        if *cursor >= text.len() {
            return false;
        }
        match text[*cursor] {
            ',' => {
                *cursor += 1;
            }
            '}' => {
                *cursor += 1;
                return true;
            }
            _ => return false,
        }
    }
}

/// One `[ … ]` (empty, or comma-separated values).
fn parse_array(text: &[char], cursor: &mut usize) -> bool {
    *cursor += 1; // '['
    skip_ws(text, cursor);
    if *cursor < text.len() && text[*cursor] == ']' {
        *cursor += 1;
        return true;
    }
    loop {
        skip_ws(text, cursor);
        if !parse_value(text, cursor) {
            return false;
        }
        skip_ws(text, cursor);
        if *cursor >= text.len() {
            return false;
        }
        match text[*cursor] {
            ',' => {
                *cursor += 1;
            }
            ']' => {
                *cursor += 1;
                return true;
            }
            _ => return false,
        }
    }
}

/// One `"…"` with `\` escapes.
fn parse_string(text: &[char], cursor: &mut usize) -> bool {
    *cursor += 1; // '"'
    while *cursor < text.len() {
        match text[*cursor] {
            '"' => {
                *cursor += 1;
                return true;
            }
            '\\' => {
                *cursor += 1;
                if *cursor >= text.len() {
                    return false;
                }
                // The six structural escapes plus \uXXXX.
                match text[*cursor] {
                    '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' => *cursor += 1,
                    'u' => {
                        *cursor += 1;
                        for _ in 0..4 {
                            if *cursor >= text.len() || !text[*cursor].is_ascii_hexdigit() {
                                return false;
                            }
                            *cursor += 1;
                        }
                    }
                    _ => return false,
                }
            }
            c if (c as u32) >= 0x20 => *cursor += 1,
            // Unescaped control characters are invalid JSON.
            _ => return false,
        }
    }
    false
}

/// One bare literal (`true`/`false`/`null`).
fn parse_literal(text: &[char], cursor: &mut usize, word: &str) -> bool {
    for expected in word.chars() {
        if *cursor >= text.len() || text[*cursor] != expected {
            return false;
        }
        *cursor += 1;
    }
    true
}

/// One number (`-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?`).
fn parse_number(text: &[char], cursor: &mut usize) -> bool {
    if *cursor < text.len() && text[*cursor] == '-' {
        *cursor += 1;
    }
    if *cursor >= text.len() || !text[*cursor].is_ascii_digit() {
        return false;
    }
    if text[*cursor] == '0' {
        *cursor += 1;
    } else {
        while *cursor < text.len() && text[*cursor].is_ascii_digit() {
            *cursor += 1;
        }
    }
    if *cursor < text.len() && text[*cursor] == '.' {
        *cursor += 1;
        let start = *cursor;
        while *cursor < text.len() && text[*cursor].is_ascii_digit() {
            *cursor += 1;
        }
        if *cursor == start {
            return false;
        }
    }
    if *cursor < text.len() && matches!(text[*cursor], 'e' | 'E') {
        *cursor += 1;
        if *cursor < text.len() && matches!(text[*cursor], '+' | '-') {
            *cursor += 1;
        }
        let start = *cursor;
        while *cursor < text.len() && text[*cursor].is_ascii_digit() {
            *cursor += 1;
        }
        if *cursor == start {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Config-parsing tests (pure)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole-binary serializer (the counting allocator is
    /// process-global; a sibling test's allocations inside a measured
    /// window are pollution).
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        serialize_tests()
    }

    #[test]
    fn json_validator_accepts_and_rejects_real_shapes() {
        let _serial = serial();
        // Accepts: the canonical examples from RFC 8259 §13 (recreated
        // from the grammar, not copied) plus edge shapes.
        for good in [
            "{}",
            "[]",
            "null",
            "true",
            "false",
            "42",
            "-0.5e+10",
            "\"x\"",
            "{\"a\": [1, 2, {\"b\": null}], \"c\": \"\\u0041\\n\"}",
            "  { \"k\" : \"v\" }  \n",
        ] {
            assert!(
                json_is_syntactically_valid(good),
                "valid JSON rejected: {good:?}"
            );
        }
        // Rejects: truncated, trailing junk, bad escapes, control
        // characters, bare words.
        for bad in [
            "",
            "{",
            "[1,]",
            "{\"a\"}",
            "{\"a\": }",
            "\"unterminated",
            "\"bad \\q escape\"",
            "\"raw\nnewline\"",
            "hello",
            "1 2",
            "{,}",
        ] {
            assert!(
                !json_is_syntactically_valid(bad),
                "invalid JSON accepted: {bad:?}"
            );
        }
    }

    #[test]
    fn only_scenario_filter_shapes_refuse_loudly() {
        let _serial = serial();
        // The filter contract is env-driven and process-global: the
        // selection over the real default manifest is consistent with
        // whatever the environment asks for (unset = all — the CI and
        // nightly posture).
        let root = testkit::workspace_root().must();
        let manifest = manifest::load(&root.join("tests/soak/scenarios.toml"))
            .unwrap_or_else(|error| unreachable!("default manifest: {error}"));
        let selected = select_scenarios(&manifest).must();
        match only_scenario().must() {
            None => assert_eq!(
                selected.len(),
                manifest.scenarios.len(),
                "unset filter = all scenarios"
            ),
            Some(wanted) => {
                assert_eq!(selected.len(), 1, "one named scenario");
                assert_eq!(selected[0].id, wanted);
            }
        }
    }
}
