//! `xtask feature-audit` — the docs/feature-matrix.toml auditor (issue #20,
//! card 8.F1; the debt named in the matrix header lines 10-16 and
//! docs/gates/phase5-records-audit.md §2).
//!
//! CI's inline python3 check (card 1.E2) covers R9 parse/meta, R1
//! id-regex/uniqueness, R2 ranges, R3 platform columns + vocabulary and the
//! R4/R5/R6 *emptiness* conditionals. This tool adds what only a real tool
//! can do — evidence RESOLUTION and honest counting:
//!
//! | # | Check | What it proves |
//! |---|-------|----------------|
//!   A | structure  | everything the inline CI check proves, in Rust (R9/R1/R2/R3, R4/R5/R6 conditionals) |
//!   B | evidence-presence | any platform claiming `implemented`/`verified` carries a non-empty `evidence` (the honest backfill rule: implemented-with-evidence, never a bare flip) |
//!   C | evidence-resolution | every repo-path token inside `evidence` resolves to an existing file/dir under the workspace root (R4 as written: "evidence resolving to an existing artifact"); `**` globs check the prefix; `inventory/…` refs map to `docs/inventory/…` |
//!   D | per-phase / per-category counts | the derived inventory the parity report consumes |
//!
//! NOT done here (honest register, same as the header's owed list):
//! R7 config cross-check (`config-validator` owns the product.toml side),
//! R8 inventory completeness (needs a formats.json↔row reconciliation spec),
//! R10 anti-hallucination beyond path existence, and `not_started → verified`
//! jump rejection in git history (needs a `--from` revision argument; v0
//! audits the working tree only).
//!
//! Exit codes: 0 pass | 1 violations | 2 xtask error (parse failures are
//! exit 1 with a `parse:` line — a broken matrix is a failed audit, the
//! message names the file:line).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::XtaskError;
use crate::cmd::toml_lite::{self, Value};

const USAGE: &str = "\
Usage: cargo xtask feature-audit [--root <dir>] [--matrix <relpath>]

Audits docs/feature-matrix.toml (issue #20, card 8.F1):

  A  structure            R9/R1/R2/R3 + R4/R5/R6 conditionals (CI parity)
  B  evidence-presence    implemented/verified rows carry evidence
  C  evidence-resolution  evidence path tokens resolve under the root
  D  counts               per-phase and per-category row counts

  --root <dir>      workspace root (default: walk up from the cwd)
  --matrix <rel>    matrix path relative to the root
                    (default: docs/feature-matrix.toml)

Exit code: 0 pass | 1 violations | 2 xtask error.";

const STATUSES: [&str; 7] = [
    "not_started",
    "in_progress",
    "implemented",
    "verified",
    "partial",
    "unsupported",
    "blocked",
];
const PLATFORMS: [&str; 4] = ["windows", "linux", "macos", "web"];
const PRIORITIES: [&str; 4] = ["P0", "P1", "P2", "P3"];
const ROW_REQUIRED: [&str; 6] = [
    "id",
    "category",
    "priority",
    "phase_target",
    "status",
    "evidence",
];

/// One matrix row, minimally typed for the audit.
struct Row {
    id: String,
    category: String,
    phase: Option<i64>,
    evidence: String,
    has_notes: bool,
    has_blocked_reason: bool,
    statuses: Vec<(String, String)>, // platform, value
    line_hint: usize,                // array index + 1 for messages
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut root_override: Option<PathBuf> = None;
    let mut matrix_rel = "docs/feature-matrix.toml".to_string();
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
                        "--root requires a directory\n{USAGE}"
                    )));
                };
                root_override = Some(PathBuf::from(dir));
            }
            "--matrix" => {
                i += 1;
                let Some(p) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--matrix requires a path\n{USAGE}"
                    )));
                };
                matrix_rel = p.clone();
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
        None => find_root()?,
    };
    let matrix_path = root.join(&matrix_rel);
    let text = std::fs::read_to_string(&matrix_path)
        .map_err(|e| XtaskError::io(format!("reading {}", matrix_path.display()), e))?;

    let mut violations: Vec<String> = Vec::new();
    let doc = match toml_lite::parse(&text) {
        Ok(doc) => doc,
        Err(err) => {
            // A parse failure is a failed audit (exit 1), with file:line.
            println!("=== LUMEN FEATURE-AUDIT REPORT ===");
            println!("matrix: {}", matrix_path.display());
            println!("parse: {err}");
            println!("result: FAILURE");
            return Ok(ExitCode::from(1));
        }
    };

    // --- A: structure (meta) ------------------------------------------------
    let Some(meta) = doc.table("meta") else {
        violations.push("[meta] table missing (R9)".to_string());
        return finish(&root, &matrix_rel, &[], &violations);
    };
    let mut has_version = false;
    for (k, v) in &meta.entries {
        if k == "schema_version" {
            if v.as_int().is_none() {
                violations.push("[meta] schema_version must be an integer (R9)".to_string());
            } else {
                has_version = true;
            }
        }
    }
    if !has_version {
        violations.push("[meta] schema_version missing (R9)".to_string());
    }

    // --- A/B/C: rows ---------------------------------------------------------
    let rows_src = doc.array_rows("feature");
    if rows_src.is_empty() {
        violations.push("no [[feature]] rows (R9)".to_string());
        return finish(&root, &matrix_rel, &[], &violations);
    }
    let mut rows: Vec<Row> = Vec::new();
    let mut seen_ids: Vec<String> = Vec::new();
    for (index, src) in rows_src.iter().enumerate() {
        let row = match parse_row(src, index) {
            Ok(row) => row,
            Err(msg) => {
                violations.push(msg);
                continue;
            }
        };
        // R1: id regex + uniqueness.
        if !valid_id(&row.id) {
            violations.push(format!(
                "{}: id {:?} violates ^[a-z0-9]+(-[a-z0-9]+)+$ (R1)",
                row.line_hint, row.id
            ));
        }
        if seen_ids.iter().any(|s| s == &row.id) {
            violations.push(format!("{}: duplicate id {:?} (R1)", row.line_hint, row.id));
        }
        seen_ids.push(row.id.clone());
        // R2.
        let priority = src
            .entries
            .iter()
            .find(|(k, _)| k == "priority")
            .map(|(_, v)| v.as_str().unwrap_or("?").to_string())
            .unwrap_or_else(|| "<missing>".into());
        if !PRIORITIES.contains(&priority.as_str()) {
            violations.push(format!(
                "{}: priority {priority:?} not in {:?} (R2)",
                row.line_hint, PRIORITIES
            ));
        }
        match row.phase {
            Some(p) if (2..=8).contains(&p) => {}
            _ => violations.push(format!(
                "{}: phase_target {:?} outside 2..=8 (R2)",
                row.line_hint, row.phase
            )),
        }
        // R3: platform columns + vocabulary.
        let mut platforms: Vec<&str> = row.statuses.iter().map(|(p, _)| p.as_str()).collect();
        platforms.sort_unstable();
        let expected = sorted_platforms();
        if platforms != expected {
            violations.push(format!(
                "{}: status keys {platforms:?} != exactly {expected:?} (R3)",
                row.line_hint
            ));
        }
        for (platform, value) in &row.statuses {
            if !STATUSES.contains(&value.as_str()) {
                violations.push(format!(
                    "{}: status.{platform} = {value:?} not in the vocabulary (R3)",
                    row.line_hint
                ));
            }
        }
        let values: Vec<&str> = row.statuses.iter().map(|(_, v)| v.as_str()).collect();
        // R4/R5/R6 conditionals + the B check (implemented needs evidence too).
        let claims_work = values
            .iter()
            .any(|v| *v == "implemented" || *v == "verified");
        if claims_work && row.evidence.trim().is_empty() {
            violations.push(format!(
                "{}: implemented/verified without evidence (R4 — implemented-with-evidence is the backfill rule)",
                row.line_hint
            ));
        }
        if values
            .iter()
            .any(|v| *v == "blocked" || *v == "unsupported")
            && !row.has_blocked_reason
        {
            violations.push(format!(
                "{}: blocked/unsupported without blocked_reason (R5)",
                row.line_hint
            ));
        }
        if values.contains(&"partial") && !row.has_notes {
            violations.push(format!("{}: partial without notes (R6)", row.line_hint));
        }
        // C: evidence path resolution.
        if !row.evidence.trim().is_empty() {
            for missing in unresolved_paths(&root, &row.evidence) {
                violations.push(format!(
                    "{}: evidence cites missing path `{}` (R4)",
                    row.line_hint, missing
                ));
            }
        }
        rows.push(row);
    }

    finish(&root, &matrix_rel, &rows, &violations)
}

/// Print the report and derive the exit code.
fn finish(
    root: &Path,
    matrix_rel: &str,
    rows: &[Row],
    violations: &[String],
) -> Result<ExitCode, XtaskError> {
    println!("=== LUMEN FEATURE-AUDIT REPORT ===");
    println!("matrix: {} (root {})", matrix_rel, root.display());
    println!("rows: {}", rows.len());

    // D: per-phase and per-category counts (derived, never stored).
    let mut by_phase: BTreeMap<i64, usize> = BTreeMap::new();
    let mut by_category: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_status: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows {
        if let Some(p) = row.phase {
            *by_phase.entry(p).or_default() += 1;
        }
        *by_category.entry(row.category.as_str()).or_default() += 1;
        for (_, v) in &row.statuses {
            *by_status.entry(v.as_str()).or_default() += 1;
        }
    }
    println!("per-phase rows: {}", fmt_counts(&by_phase));
    println!("per-category rows: {}", fmt_counts(&by_category));
    println!("per-platform-status cells: {}", fmt_counts(&by_status));

    if violations.is_empty() {
        println!("result: PASS");
        Ok(ExitCode::SUCCESS)
    } else {
        for v in violations {
            println!("violation: {v}");
        }
        println!("result: FAILURE ({} violation(s))", violations.len());
        Ok(ExitCode::from(1))
    }
}

fn fmt_counts<K: std::fmt::Display>(map: &BTreeMap<K, usize>) -> String {
    map.iter()
        .map(|(k, n)| format!("{k}={n}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn sorted_platforms() -> Vec<&'static str> {
    let mut p = PLATFORMS.to_vec();
    p.sort_unstable();
    p
}

// ---------------------------------------------------------------------------
// Row parsing
// ---------------------------------------------------------------------------

fn parse_row(src: &toml_lite::ArrayRow, index: usize) -> Result<Row, String> {
    let line_hint = index + 1;
    let get = |key: &str| -> Option<&Value> {
        src.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    };
    for required in ROW_REQUIRED {
        if get(required).is_none() {
            return Err(format!("row {line_hint}: missing key `{required}`"));
        }
    }
    let id = get("id").and_then(Value::as_str).unwrap_or("?").to_string();
    let category = get("category")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();
    let phase = get("phase_target").and_then(Value::as_int);
    let evidence = get("evidence")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let has_notes = get("notes")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty());
    let has_blocked_reason = get("blocked_reason")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty());
    let Some(status) = get("status") else {
        return Err(format!("row {line_hint}: status missing"));
    };
    let Some(table) = status.as_table() else {
        return Err(format!("{id}: status is not a table (R3)"));
    };
    let statuses: Vec<(String, String)> = table
        .iter()
        .map(|(p, v)| (p.clone(), v.as_str().unwrap_or("?").to_string()))
        .collect();
    Ok(Row {
        id,
        category,
        phase,
        evidence,
        has_notes,
        has_blocked_reason,
        statuses,
        line_hint,
    })
}

fn valid_id(id: &str) -> bool {
    let mut segments = id.split('-');
    let first_ok = segments.next().is_some_and(|s| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    });
    if !first_ok {
        return false;
    }
    let rest_ok = segments.all(|s| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    });
    rest_ok && id.contains('-')
}

// ---------------------------------------------------------------------------
// Evidence path resolution (check C)
// ---------------------------------------------------------------------------

/// Extract repo-path-like tokens from an evidence string and return the ones
/// that do NOT resolve under `root`.
///
/// A token is path-like when it contains `/` and its first segment is an
/// existing top-level directory of the workspace (or one of the known
/// top-level files). This is deliberately conservative: prose like
/// "Matroska/WebM" or "3/10/60/300" never matches because `Matroska`/`3` are
/// not workspace directories, while every real citation (`crates/...`,
/// `ui/src/...`, `docs/...`) does. `#anchors` are stripped, `**` globs
/// validate their directory prefix, and the historical `inventory/...`
/// shorthand maps to `docs/inventory/...` (the matrix's own convention, see
/// its `inventory_ref` fields).
pub fn unresolved_paths(root: &Path, evidence: &str) -> Vec<String> {
    let mut missing = Vec::new();
    for token in path_tokens(evidence) {
        if let Some(bad) = check_token(root, &token) {
            missing.push(bad);
        }
    }
    missing
}

/// Split the string into maximal runs of path-plausible characters.
fn path_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '#' | '*') {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Workspace top-level directory names that evidence citations may start
/// with (structure constants of THIS repo — apps, crates, design, docs,
/// download, fuzz, keymaps, locales, schemas, scripts, tests, themes, tools,
/// ui — not evidence, and deliberately NOT `target/`). A token whose first
/// segment is none of these (or a known top-level file) is treated as prose.
const ROOT_DIRS: [&str; 14] = [
    "apps", "crates", "design", "docs", "download", "fuzz", "keymaps", "locales", "schemas",
    "scripts", "tests", "themes", "tools", "ui",
];
const ROOT_FILES: [&str; 9] = [
    "product.toml",
    "product.schema.json",
    "README.md",
    "worklog.md",
    "AGENTS.md",
    "deny.toml",
    "boundaries.toml",
    "rust-toolchain.toml",
    "Cargo.toml",
];

/// Validate one token; `None` = fine (or not path-like), `Some(bad)` = the
/// unresolved path as reported.
fn check_token(root: &Path, token: &str) -> Option<String> {
    // Strip a trailing/inline anchor (`docs/x.md#section`).
    let token = token.split('#').next().unwrap_or(token);
    if token.is_empty() {
        return None;
    }
    // Inventory shorthand (the matrix's inventory_ref convention).
    let mapped;
    let token = if let Some(rest) = token.strip_prefix("inventory/") {
        mapped = format!("docs/inventory/{rest}");
        &mapped
    } else {
        token
    };
    if !token.contains('/') {
        return None;
    }
    let first = token.split('/').next().unwrap_or("");
    if first.is_empty() {
        return None; // `//...` — URL residue, not a repo path
    }
    if !ROOT_DIRS.contains(&first) && !ROOT_FILES.contains(&first) {
        return None; // prose like "Matroska/WebM" or "3/10/60/300"
    }
    // `**` globs cite a tree: validate the directory prefix.
    let mut path = token.to_string();
    if let Some(pos) = path.find("**") {
        path.truncate(pos);
        let path = path.trim_end_matches('/');
        if path.is_empty() {
            return None;
        }
        return if root.join(path).exists() {
            None
        } else {
            Some(path.to_string())
        };
    }
    // Trailing `/` cites a directory.
    let path = path.trim_end_matches('/');
    if root.join(path).exists() {
        None
    } else {
        Some(path.to_string())
    }
}

// ---------------------------------------------------------------------------
// Root discovery
// ---------------------------------------------------------------------------

fn find_root() -> Result<PathBuf, XtaskError> {
    let cwd =
        std::env::current_dir().map_err(|e| XtaskError::io("determining current directory", e))?;
    let mut dir: &Path = &cwd;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs").is_dir() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                return Err(XtaskError::internal(
                    "workspace root not found — run inside the Lumen workspace or pass --root",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait Must {
        type Value;
        #[track_caller]
        fn must(self) -> Self::Value;
    }
    impl<T, E: std::fmt::Display> Must for Result<T, E> {
        type Value = T;
        #[track_caller]
        fn must(self) -> T {
            match self {
                Ok(v) => v,
                Err(e) => unreachable!("test must(): {e}"),
            }
        }
    }
    impl<T> Must for Option<T> {
        type Value = T;
        #[track_caller]
        fn must(self) -> T {
            match self {
                Some(v) => v,
                None => unreachable!("test must(): None"),
            }
        }
    }

    #[test]
    fn id_regex_matches_matrix_conventions() {
        assert!(valid_id("container-matroska-webm"));
        assert!(valid_id("ui-screen-player"));
        assert!(valid_id("hotkey-2x-rate"));
        assert!(!valid_id("Container-X"), "uppercase rejected");
        assert!(!valid_id("container"), "single segment rejected");
        assert!(!valid_id("container--x"), "empty segment rejected");
        assert!(!valid_id("container_x"), "underscore rejected");
    }

    #[test]
    fn unresolved_paths_ignores_prose_and_urls() {
        let tmp = std::env::temp_dir().join(format!("xtask-fa-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("crates/demo")).must();
        std::fs::write(tmp.join("crates/demo/lib.rs"), "").must();
        let evidence =
            "Matroska/WebM; 3/10/60/300 s; https://example.org/x; see issue #34 and MP §17";
        assert!(unresolved_paths(&tmp, evidence).is_empty());
    }

    #[test]
    fn unresolved_paths_flags_missing_files_and_accepts_existing() {
        let tmp = std::env::temp_dir().join(format!("xtask-fa2-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("crates/demo")).must();
        std::fs::write(tmp.join("crates/demo/lib.rs"), "").must();
        std::fs::create_dir_all(tmp.join("docs")).must();
        let evidence =
            "crates/demo/lib.rs (card 5.D1); crates/demo/missing.rs; crates/demo/** works";
        let missing = unresolved_paths(&tmp, evidence);
        assert_eq!(missing, vec!["crates/demo/missing.rs".to_string()]);
    }

    #[test]
    fn unresolved_paths_maps_inventory_shorthand() {
        let tmp = std::env::temp_dir().join(format!("xtask-fa3-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("docs/inventory")).must();
        std::fs::write(tmp.join("docs/inventory/formats.json"), "{}").must();
        assert!(unresolved_paths(&tmp, "inventory/formats.json#matroska-webm").is_empty());
        assert_eq!(
            unresolved_paths(&tmp, "inventory/options-linux.json#rate"),
            vec!["docs/inventory/options-linux.json".to_string()]
        );
    }
}
