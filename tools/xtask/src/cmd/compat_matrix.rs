//! `xtask compat-matrix` — the 8.B2 transitions writer (issue #53, card
//! 8.B2; the parity tooling lane closed by issue #57 built the readers —
//! `feature-audit` + `parity-report` — and left the writer deliberately
//! unbuilt: "no code path exists that edits the matrix"). This module is
//! that writer, landed under issue #53's granted ownership with exactly
//! one dispatch-line registration in `cmd/mod.rs`.
//!
//! # The contract (doc 10 §7.3, MP §15: the tool decides, never the author)
//!
//! Status transitions for `docs/feature-matrix.toml` flow ONLY through a
//! transitions outbox (schema `compat-matrix/transitions/1`, emitted by
//! the compat-matrix runner at `tests/compat/` or authored by a triage
//! round — a proposal document, reviewable in git) into THIS tool:
//!
//! ```text
//! runner / triage  →  outbox (proposals)  →  cargo xtask compat-matrix
//!                                             apply-transitions  →  matrix
//!                                           (feature-audit then judges)
//! ```
//!
//! The tool re-checks every proposal against the LIVE matrix before
//! writing anything, and REJECTS (never silently applies):
//!
//! * a `from` status that no longer matches the row's current platform
//!   status (a stale or already-superseded outbox);
//! * `to = "verified"` — **no self-verification path exists in code**
//!   (the 8.F1 AC; agents never mark their own work verified, and no CI
//!   evidence exists while billing is blocked);
//! * unknown rows, unknown platforms, out-of-vocabulary statuses,
//!   schema mismatches, unparseable outboxes;
//! * evidence whose repo-path tokens do not resolve under the root
//!   (feature-audit's R4 rule, reused verbatim).
//!
//! Application is all-or-nothing per invocation: any rejection writes
//! NOTHING (the matrix, the ledger and the report stay untouched).
//! Proposals for the same (row, platform) coalesce — reasons merge into
//! one machine-written notes sentence, and each original proposal keeps
//! its own ledger line. Re-applying an outbox already recorded in the
//! ledger is a no-op (`already-applied`), not an error.
//!
//! What the tool WRITES per applied (row, platform):
//! * the status cell (`<platform> = "<from>"` → `"<to>"`);
//! * a machine-written append to the row's `notes` (reason + next step +
//!   provenance) — satisfying R6 (partial needs notes) by construction;
//! * an append to the row's `evidence` (the proposal's evidence field,
//!   path-checked) — so the flip is R4-resolvable in the matrix itself;
//!   a citation is appended ONCE: coalesced proposals that cite the SAME
//!   artifact append it once (IN-8, integration audit R9), and a
//!   citation already present on the evidence line is never re-appended
//!   (a later outbox re-citing it — e.g. across a status flip-back —
//!   leaves the cell byte-identical);
//! * one JSONL line per original proposal in the ledger (default
//!   `docs/corpus/compat-matrix-transitions-applied.jsonl`).
//!
//! The rewritten matrix is re-parsed and diffed against the pre-parse
//! row-by-row before it is written: only the intended status/notes/
//! evidence cells may differ, or the tool errors out without writing.
//!
//! Std-only (xtask C5): the outbox JSON is read by a strict minimal
//! RFC 8259 parser below (objects, arrays, strings with the full escape
//! set incl. `\uXXXX` surrogate pairs, numbers, booleans, null) — no
//! `serde_json` here.
//!
//! Exit codes: 0 applied (or already-applied / dry-run clean) |
//! 1 rejected proposals (details on stdout, nothing written) |
//! 2 xtask error (unreadable files, parse failures, internal errors).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::XtaskError;
use crate::cmd::feature_audit::unresolved_paths;
use crate::cmd::toml_lite::{self, Value};

const USAGE: &str = "\
Usage: cargo xtask compat-matrix apply-transitions --outbox <relpath>
                 [--matrix <relpath>] [--ledger <relpath>] [--dry-run]
                 [--root <dir>]

The 8.B2 status writer (issue #53; the doc 10 §7.3 outbox -> tool ->
matrix flow). Applies a transitions outbox (schema
compat-matrix/transitions/1) to docs/feature-matrix.toml after
re-checking every proposal against the live matrix:

  - `from` must equal the row's CURRENT platform status (stale = reject)
  - `to` must be in the status vocabulary and NEVER `verified`
    (no self-verification path exists in code)
  - evidence path tokens must resolve under the root (R4)
  - same (row, platform) proposals coalesce; re-applying a ledgered
    outbox is a no-op (already-applied)
  - all-or-nothing: any rejection writes NOTHING

  --outbox <rel>   outbox path relative to the root (required)
  --matrix <rel>   matrix path (default docs/feature-matrix.toml)
  --ledger <rel>   applied-transitions ledger
                   (default docs/corpus/compat-matrix-transitions-applied.jsonl)
  --dry-run        validate + print the plan, write nothing
  --root <dir>     workspace root (default: walk up from the cwd)

Exit code: 0 applied/no-op | 1 rejected proposals | 2 xtask error.";

/// The only accepted outbox schema.
const OUTBOX_SCHEMA: &str = "compat-matrix/transitions/1";
/// The ledger's schema string.
const LEDGER_SCHEMA: &str = "compat-matrix/transitions-applied/1";
/// The status vocabulary (R3; mirrors feature_audit).
const STATUSES: [&str; 7] = [
    "not_started",
    "in_progress",
    "implemented",
    "verified",
    "partial",
    "unsupported",
    "blocked",
];
/// The four matrix platform columns (R3; mirrors feature_audit).
const PLATFORMS: [&str; 4] = ["windows", "linux", "macos", "web"];

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut root_override: Option<PathBuf> = None;
    let mut outbox_rel: Option<String> = None;
    let mut matrix_rel = "docs/feature-matrix.toml".to_string();
    let mut ledger_rel = "docs/corpus/compat-matrix-transitions-applied.jsonl".to_string();
    let mut dry_run = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "--dry-run" => dry_run = true,
            "--outbox" | "--matrix" | "--ledger" | "--root" => {
                let flag = args[i].as_str();
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "{flag} requires a path\n{USAGE}"
                    )));
                };
                match flag {
                    "--outbox" => outbox_rel = Some(value.clone()),
                    "--matrix" => matrix_rel = value.clone(),
                    "--ledger" => ledger_rel = value.clone(),
                    _ => root_override = Some(PathBuf::from(value)),
                }
            }
            "apply-transitions" => {} // the only subcommand; positional
            other => {
                return Err(XtaskError::Usage(format!(
                    "unexpected argument `{other}`\n{USAGE}"
                )));
            }
        }
        i += 1;
    }
    let Some(outbox_rel) = outbox_rel else {
        return Err(XtaskError::Usage(format!(
            "apply-transitions requires --outbox <relpath>\n{USAGE}"
        )));
    };
    let root = match root_override {
        Some(dir) => dir,
        None => find_root()?,
    };
    let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, dry_run)?;
    let code = if outcome.rejected.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };
    Ok(code)
}

/// What one invocation did.
#[derive(Default)]
pub struct Outcome {
    /// (row, platform, from, to) proposals applied (post-coalescing).
    pub applied: Vec<(String, String, String, String)>,
    /// Proposals skipped because their outbox is already in the ledger.
    pub already: usize,
    /// Rejection messages (empty ⇒ nothing rejected).
    pub rejected: Vec<String>,
}

/// The full apply pipeline. Exposed for the in-module tests (which drive
/// it against fixture matrices in temp dirs).
pub fn apply(
    root: &Path,
    outbox_rel: &str,
    matrix_rel: &str,
    ledger_rel: &str,
    dry_run: bool,
) -> Result<Outcome, XtaskError> {
    // --- read + parse the outbox -----------------------------------------
    let outbox_path = root.join(outbox_rel);
    let outbox_text = std::fs::read_to_string(&outbox_path)
        .map_err(|e| XtaskError::io(format!("reading {}", outbox_path.display()), e))?;
    let outbox = Outbox::parse(&outbox_text).map_err(|msg| {
        XtaskError::internal(format!("outbox {} is not applicable: {msg}", outbox_rel))
    })?;

    // --- read + parse the matrix -----------------------------------------
    let matrix_path = root.join(matrix_rel);
    let matrix_text = std::fs::read_to_string(&matrix_path)
        .map_err(|e| XtaskError::io(format!("reading {}", matrix_path.display()), e))?;
    let before = toml_lite::parse(&matrix_text).map_err(|err| {
        XtaskError::internal(format!("matrix {} does not parse: {err}", matrix_rel))
    })?;

    // --- read the ledger (idempotency) ------------------------------------
    let ledger_path = root.join(ledger_rel);
    let ledger_text = std::fs::read_to_string(&ledger_path).unwrap_or_default();
    let ledgered = parse_ledger(&ledger_text).map_err(|msg| {
        XtaskError::internal(format!("ledger {} does not parse: {msg}", ledger_rel))
    })?;

    // --- validate + coalesce ----------------------------------------------
    let mut outcome = Outcome::default();
    // (row, platform) -> coalesced proposal
    let mut groups: Vec<(String, String, Group)> = Vec::new();
    for proposal in &outbox.transitions {
        let key = (proposal.row.clone(), proposal.platform.clone());
        if ledgered.contains(&(outbox_rel.to_string(), key.0.clone(), key.1.clone())) {
            outcome.already += 1;
            continue;
        }
        if let Some((_, _, group)) = groups
            .iter_mut()
            .find(|(r, p, _)| (r.as_str(), p.as_str()) == (key.0.as_str(), key.1.as_str()))
        {
            group.members.push(proposal.clone());
        } else {
            groups.push((
                key.0,
                key.1,
                Group {
                    members: vec![proposal.clone()],
                },
            ));
        }
    }

    for (row, platform, group) in &groups {
        if let Some(message) = validate(root, &before, row, platform, group) {
            outcome.rejected.push(message);
        }
    }
    if !outcome.rejected.is_empty() {
        for message in &outcome.rejected {
            println!("rejected: {message}");
        }
        println!(
            "result: REJECTED ({} proposal group(s) rejected, nothing written)",
            outcome.rejected.len()
        );
        return Ok(outcome);
    }

    // --- plan the rewrite --------------------------------------------------
    let mut plan = matrix_text.clone();
    for (row, platform, group) in &groups {
        let current = current_status(&before, row, platform.as_str());
        let edited = edit_row(&plan, row, platform, group, current.as_deref())
            .map_err(XtaskError::internal)?;
        plan = edited;
    }

    // --- self-check: re-parse + row-by-row diff ----------------------------
    let after = toml_lite::parse(&plan)
        .map_err(|err| XtaskError::internal(format!("rewritten matrix does not parse: {err}")))?;
    verify_diff(&before, &after, &groups)?;

    // --- write (matrix + ledger), or stop at the dry-run print ------------
    if dry_run {
        for (row, platform, group) in &groups {
            let to = group.members[0].to.as_str();
            println!(
                "dry-run: {row}.{platform}: {} -> {to} ({} proposal(s))",
                group.members[0].from,
                group.members.len()
            );
        }
        println!(
            "result: DRY-RUN CLEAN ({} group(s) would apply, {} already-applied, nothing written)",
            groups.len(),
            outcome.already
        );
        return Ok(outcome);
    }
    std::fs::write(&matrix_path, &plan)
        .map_err(|e| XtaskError::io(format!("writing {}", matrix_path.display()), e))?;
    if let Some(parent) = ledger_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| XtaskError::io(format!("creating {}", parent.display()), e))?;
    }
    let applied_at = iso8601_now();
    let mut ledger_line = String::new();
    for (row, platform, group) in &groups {
        outcome.applied.push((
            row.clone(),
            platform.clone(),
            group.members[0].from.clone(),
            group.members[0].to.clone(),
        ));
        for member in &group.members {
            ledger_line.push_str(&render_ledger_line(
                &applied_at,
                outbox_rel,
                matrix_rel,
                row,
                platform,
                member,
            ));
            ledger_line.push('\n');
        }
    }
    if !ledger_line.is_empty() {
        use std::io::Write as _;
        let mut ledger = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&ledger_path)
            .map_err(|e| XtaskError::io(format!("opening {}", ledger_path.display()), e))?;
        ledger
            .write_all(ledger_line.as_bytes())
            .map_err(|e| XtaskError::io(format!("appending {}", ledger_path.display()), e))?;
    }

    for (row, platform, from, to) in &outcome.applied {
        println!("applied: {row}.{platform}: {from} -> {to}");
    }
    println!(
        "result: APPLIED ({} group(s), {} proposal line(s) to the ledger, {} already-applied)",
        outcome.applied.len(),
        ledger_line.lines().count(),
        outcome.already
    );
    Ok(outcome)
}

/// One coalesced group: every proposal for the same (row, platform).
struct Group {
    members: Vec<Proposal>,
}

/// One outbox proposal (schema `compat-matrix/transitions/1`).
#[derive(Clone)]
pub struct Proposal {
    pub row: String,
    pub platform: String,
    pub from: String,
    pub to: String,
    pub reason: String,
    pub next_step: String,
    pub evidence: String,
}

/// The parsed outbox.
pub struct Outbox {
    pub transitions: Vec<Proposal>,
}

impl Outbox {
    /// Parse + schema-check an outbox document.
    pub fn parse(text: &str) -> Result<Outbox, String> {
        let value = json::parse(text).map_err(|err| format!("JSON: {err}"))?;
        let object = value
            .as_object()
            .ok_or("the outbox must be a JSON object")?;
        let schema = object
            .iter()
            .find(|(k, _)| k == "schema")
            .and_then(|(_, v)| v.as_str())
            .ok_or("the outbox must carry a `schema` string")?;
        if schema != OUTBOX_SCHEMA {
            return Err(format!(
                "schema {schema:?} != {OUTBOX_SCHEMA:?} (the only accepted shape)"
            ));
        }
        let transitions = object
            .iter()
            .find(|(k, _)| k == "transitions")
            .and_then(|(_, v)| v.as_array())
            .ok_or("the outbox must carry a `transitions` array")?;
        let mut parsed = Vec::new();
        for transition in transitions {
            let entry = transition
                .as_object()
                .ok_or("every transition must be a JSON object")?;
            let string = |key: &str| -> Result<String, String> {
                entry
                    .iter()
                    .find(|(k, _)| k == key)
                    .and_then(|(_, v)| v.as_str())
                    .map(str::to_string)
                    .ok_or_else(|| format!("a transition is missing the `{key}` string"))
            };
            parsed.push(Proposal {
                row: string("row")?,
                platform: string("platform")?,
                from: string("from")?,
                to: string("to")?,
                reason: string("reason")?,
                next_step: string("next_step")?,
                evidence: string("evidence")?,
            });
        }
        Ok(Outbox {
            transitions: parsed,
        })
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate one coalesced group against the live matrix. `None` = fine.
fn validate(
    root: &Path,
    before: &toml_lite::Document,
    row: &str,
    platform: &str,
    group: &Group,
) -> Option<String> {
    let first = &group.members[0];
    if !PLATFORMS.contains(&platform) {
        return Some(format!(
            "{row}: platform {platform:?} is not one of {PLATFORMS:?}"
        ));
    }
    if !STATUSES.contains(&first.to.as_str()) {
        return Some(format!(
            "{row}.{platform}: `to` status {:?} is not in the vocabulary {STATUSES:?}",
            first.to
        ));
    }
    if first.to == "verified" {
        return Some(format!(
            "{row}.{platform}: `to` = \"verified\" is REJECTED — no self-verification path \
             exists in code (MP §15, doc 10 §7.3); verified requires Lead-applied CI evidence"
        ));
    }
    if !row_exists(before, row) {
        return Some(format!(
            "{row}: no such [[feature]] row in the matrix (the outbox is stale or wrong)"
        ));
    }
    for member in &group.members {
        if member.to != first.to {
            return Some(format!(
                "{row}.{platform}: proposals disagree on `to` ({:?} vs {:?}) — split the outbox",
                first.to, member.to
            ));
        }
        if member.from != first.from {
            return Some(format!(
                "{row}.{platform}: proposals disagree on `from` ({:?} vs {:?}) — the outbox is \
                 internally inconsistent",
                first.from, member.from
            ));
        }
        if let Some(missing) = unresolved_paths(root, &member.evidence).into_iter().next() {
            return Some(format!(
                "{row}.{platform}: evidence cites missing path `{missing}` (R4)"
            ));
        }
    }
    let Some(current) = current_status(before, row, platform) else {
        return Some(format!(
            "{row}.{platform}: the row exists but has no `{platform}` status column (R3 shape)"
        ));
    };
    if current != first.from {
        return Some(format!(
            "{row}.{platform}: STALE — the outbox says from {:?} but the live matrix says \
             {:?}; regenerate the outbox from a fresh run",
            first.from, current
        ));
    }
    None
}

/// Whether a `[[feature]]` row with this id exists.
fn row_exists(doc: &toml_lite::Document, row: &str) -> bool {
    doc.array_rows("feature").iter().any(|src| {
        src.entries
            .iter()
            .any(|(k, v)| k == "id" && v.as_str() == Some(row))
    })
}

/// The row's CURRENT status on one platform.
fn current_status(doc: &toml_lite::Document, row: &str, platform: &str) -> Option<String> {
    let rows = doc.array_rows("feature");
    let src = rows.iter().find(|src| {
        src.entries
            .iter()
            .any(|(k, v)| k == "id" && v.as_str() == Some(row))
    })?;
    let status = src
        .entries
        .iter()
        .find(|(k, _)| k == "status")
        .map(|(_, v)| v)?;
    status
        .as_table()?
        .iter()
        .find(|(k, _)| k == platform)
        .and_then(|(_, v)| v.as_str())
        .map(str::to_string)
}

// ---------------------------------------------------------------------------
// The rewrite
// ---------------------------------------------------------------------------

/// One located feature block in the matrix text (line indexes,
/// 0-based; `status_line`/`evidence_line` point at the row's own
/// single-line entries).
struct Block {
    id: String,
    start: usize,
    status_line: usize,
    notes_line: Option<usize>,
    evidence_line: usize,
}

/// Scan the matrix text into feature blocks. Every row in the shipped
/// matrix keeps `id`, `status` (a single-line inline table) and
/// `evidence` on their own lines; a row that breaks that shape is a loud
/// error, never a silent skip.
fn scan_blocks(text: &str) -> Result<Vec<Block>, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();
    let mut current: Option<Block> = None;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[[") {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            current = Some(Block {
                id: String::new(),
                start: index,
                status_line: usize::MAX,
                notes_line: None,
                evidence_line: usize::MAX,
            });
            continue;
        }
        if trimmed.starts_with('[') {
            // a [table] header ends the last feature block
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            continue;
        }
        if let Some(block) = current.as_mut() {
            let key = trimmed.split('=').next().unwrap_or("").trim();
            match key {
                "id" => {
                    let value = trimmed
                        .strip_prefix("id")
                        .and_then(|rest| rest.trim_start().strip_prefix('='))
                        .map(|rest| rest.trim())
                        .unwrap_or("");
                    let parsed = toml_lite::parse(&format!("v = {value}"))
                        .ok()
                        .and_then(|doc| doc.get("v").cloned());
                    if let Some(Value::Str(id)) = parsed {
                        block.id = id;
                    }
                }
                "status" => block.status_line = index,
                "notes" => block.notes_line = Some(index),
                "evidence" => block.evidence_line = index,
                _ => {}
            }
        }
    }
    if let Some(block) = current.take() {
        blocks.push(block);
    }
    for block in &blocks {
        if block.id.is_empty() {
            return Err(format!(
                "feature block at line {} has no readable `id` line",
                block.start + 1
            ));
        }
        if block.status_line == usize::MAX {
            return Err(format!(
                "feature `{}` has no single-line `status = {{ ... }}` line (the writer's \
                 shape assumption — extend it before editing such a row)",
                block.id
            ));
        }
        if block.evidence_line == usize::MAX {
            return Err(format!("feature `{}` has no `evidence` line", block.id));
        }
    }
    Ok(blocks)
}

/// Apply one group's edits to the matrix text (status + notes +
/// evidence). `current` is the re-checked live status for the assert.
fn edit_row(
    text: &str,
    row: &str,
    platform: &str,
    group: &Group,
    current: Option<&str>,
) -> Result<String, String> {
    let blocks = scan_blocks(text)?;
    let Some(block) = blocks.iter().find(|block| block.id == row) else {
        return Err(format!(
            "{row}: the row vanished between validation and edit"
        ));
    };
    let first = &group.members[0];
    if current != Some(first.from.as_str()) {
        return Err(format!(
            "{row}.{platform}: live status changed under the tool ({current:?} != {:?})",
            first.from
        ));
    }
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();

    // 1. the status cell: `<platform> = "<from>"` -> `<platform> = "<to>"`
    //    inside the row's status line (exact, asserted, once).
    let status_line = lines[block.status_line].clone();
    let needle = format!("{platform} = \"{}\"", first.from);
    let matches = status_line.matches(&needle).count();
    if matches != 1 {
        return Err(format!(
            "{row}.{platform}: expected exactly one `{needle}` in the status line, found \
             {matches} — refusing to guess",
        ));
    }
    lines[block.status_line] =
        status_line.replacen(&needle, &format!("{platform} = \"{}\"", first.to), 1);

    // 2. the notes append (machine-written, escaped for a TOML basic
    //    string): reason(s) + next step(s) + provenance.
    let applied_at = iso8601_now();
    let reasons: Vec<String> = group
        .members
        .iter()
        .map(|member| member.reason.clone())
        .collect();
    let next_steps: Vec<String> = group
        .members
        .iter()
        .map(|member| member.next_step.clone())
        .collect();
    let note = format!(
        " {applied_at} compat-matrix gap triage (issue #53, card 8.B2; applied by cargo xtask \
         compat-matrix apply-transitions): {platform} {}->{} — {} Next step: {}",
        first.from,
        first.to,
        reasons.join(" | "),
        next_steps.join(" | ")
    );
    match block.notes_line {
        Some(index) => {
            append_into_string(&mut lines[index], &note)?;
        }
        None => {
            // R6: partial needs notes — insert a notes line right after
            // the evidence line so the row stays shape-legal.
            lines.insert(
                block.evidence_line + 1,
                format!("notes = \"{}\"", escape_toml(&note)),
            );
        }
    }

    // 3. the evidence append: the proposals' evidence fields (R4-checked).
    //    Two idempotence limbs (IN-8, integration audit R9: the aac-lc
    //    pair double-appended an identical string):
    //    (a) coalesced proposals of one run cite the SAME artifact (every
    //    transition a run emits cites that run's report) — each DISTINCT
    //    citation is appended once, in order, never once per proposal;
    //    (b) a citation that is already a "; "-separated segment of the
    //    row's CURRENT evidence value is not re-appended, so a later
    //    outbox re-citing the same artifact leaves the cell unchanged.
    let existing_evidence =
        string_value_of(&lines[block.evidence_line], "evidence").ok_or_else(|| {
            format!(
                "{row}: the evidence line is not a quoted TOML string: {}",
                lines[block.evidence_line]
            )
        })?;
    let present: Vec<&str> = existing_evidence.split("; ").collect();
    let mut citations: Vec<String> = Vec::new();
    for member in &group.members {
        if citations.contains(&member.evidence) || present.contains(&member.evidence.as_str()) {
            continue;
        }
        citations.push(member.evidence.clone());
    }
    if !citations.is_empty() {
        let citation = format!("; {}", citations.join("; "));
        append_into_string(&mut lines[block.evidence_line], &citation)?;
    }

    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// Append `addition` (raw text) inside the TOML basic string that
/// occupies `line` (`key = "...."`), escaping it for TOML.
fn append_into_string(line: &mut String, addition: &str) -> Result<(), String> {
    let Some(quote) = line.rfind('"') else {
        return Err(format!("the line has no closing TOML string quote: {line}"));
    };
    line.insert_str(quote, &escape_toml(addition));
    Ok(())
}

/// The parsed string value of a `key = "..."` line (the scan_blocks `id`
/// parse, generalized). `None` = not a single-line quoted string.
fn string_value_of(line: &str, key: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    let parsed = toml_lite::parse(&format!("v = {rest}")).ok()?;
    match parsed.get("v")? {
        Value::Str(value) => Some(value.clone()),
        _ => None,
    }
}

/// Escape raw text for a TOML basic string (the escapes toml_lite reads).
fn escape_toml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// Verify the rewrite changed ONLY the intended cells: every row must be
/// identical except the edited rows' status/notes/evidence, and each
/// edited row's platform status must equal the group's `to`.
fn verify_diff(
    before: &toml_lite::Document,
    after: &toml_lite::Document,
    groups: &[(String, String, Group)],
) -> Result<(), XtaskError> {
    let rows_before = row_map(before);
    let rows_after = row_map(after);
    if rows_before.len() != rows_after.len() {
        return Err(XtaskError::internal(
            "the rewrite changed the row count — refusing to write",
        ));
    }
    for (id, entries_before) in &rows_before {
        let entries_after = rows_after
            .get(id)
            .ok_or_else(|| XtaskError::internal(format!("the rewrite dropped row {id}")))?;
        let edited = groups.iter().any(|(row, _, _)| row == id);
        if !edited {
            if entries_before != entries_after {
                return Err(XtaskError::internal(format!(
                    "the rewrite touched UNEDITED row {id} — refusing to write"
                )));
            }
            continue;
        }
        // edited row: only status/notes/evidence may differ, and the
        // target platform's status must now be `to`.
        if entries_before.len() != entries_after.len() {
            return Err(XtaskError::internal(format!(
                "the rewrite added/removed keys on row {id} — only status/notes/evidence are \
                 writable"
            )));
        }
        for (key, value_before) in entries_before {
            let value_after = entries_after
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
                .ok_or_else(|| XtaskError::internal(format!("row {id} lost key {key}")))?;
            if matches!(key.as_str(), "status" | "notes" | "evidence") {
                continue;
            }
            if value_before != value_after {
                return Err(XtaskError::internal(format!(
                    "the rewrite changed row {id} key `{key}` — only status/notes/evidence are \
                     writable"
                )));
            }
        }
        for (row, platform, group) in groups {
            if row == id {
                let want = &group.members[0].to;
                let got = current_status(after, row, platform);
                if got.as_deref() != Some(want.as_str()) {
                    return Err(XtaskError::internal(format!(
                        "post-write check: {row}.{platform} is {got:?}, expected {want:?} — \
                         refusing to write"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Row id -> its entries, for the diff check.
fn row_map(doc: &toml_lite::Document) -> BTreeMap<String, Vec<(String, Value)>> {
    let mut map = BTreeMap::new();
    for src in doc.array_rows("feature") {
        let id = src
            .entries
            .iter()
            .find(|(k, _)| k == "id")
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("?")
            .to_string();
        map.insert(id, src.entries.clone());
    }
    map
}

// ---------------------------------------------------------------------------
// The ledger
// ---------------------------------------------------------------------------

/// Parse the ledger's (outbox, row, platform) keys from JSONL text.
fn parse_ledger(text: &str) -> Result<Vec<(String, String, String)>, String> {
    let mut keys = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value =
            json::parse(trimmed).map_err(|err| format!("line {}: JSON: {err}", index + 1))?;
        let object = value
            .as_object()
            .ok_or_else(|| format!("line {}: not a JSON object", index + 1))?;
        let get = |key: &str| -> Result<String, String> {
            object
                .iter()
                .find(|(k, _)| k == key)
                .and_then(|(_, v)| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| format!("line {}: missing `{key}`", index + 1))
        };
        let schema = get("schema")?;
        if schema != LEDGER_SCHEMA {
            return Err(format!(
                "line {}: schema {schema:?} != {LEDGER_SCHEMA:?}",
                index + 1
            ));
        }
        keys.push((get("outbox")?, get("row")?, get("platform")?));
    }
    Ok(keys)
}

/// Render one ledger line for one applied proposal.
fn render_ledger_line(
    applied_at: &str,
    outbox_rel: &str,
    matrix_rel: &str,
    row: &str,
    platform: &str,
    member: &Proposal,
) -> String {
    let mut out = String::with_capacity(512);
    out.push('{');
    let mut first = true;
    let mut field = |out: &mut String, key: &str, value: &str| {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!("\"{key}\":{}", json::string(value)));
    };
    field(&mut out, "schema", LEDGER_SCHEMA);
    field(&mut out, "applied_at", applied_at);
    field(&mut out, "outbox", outbox_rel);
    field(&mut out, "matrix", matrix_rel);
    field(&mut out, "row", row);
    field(&mut out, "platform", platform);
    field(&mut out, "from", &member.from);
    field(&mut out, "to", &member.to);
    field(&mut out, "reason", &member.reason);
    field(&mut out, "next_step", &member.next_step);
    field(&mut out, "evidence", &member.evidence);
    field(
        &mut out,
        "applied_by",
        "cargo xtask compat-matrix apply-transitions",
    );
    out.push('}');
    out
}

// ---------------------------------------------------------------------------
// Minimal JSON (RFC 8259) — the outbox/ledger reader
// ---------------------------------------------------------------------------

pub mod json {
    //! A strict std-only JSON value + parser. Covers the whole RFC 8259
    //! grammar the outbox and ledger use (objects with ordered entries,
    //! arrays, strings with the full escape set including `\uXXXX`
    //! surrogate pairs, numbers, booleans, null). Numbers are kept as
    //! f64 (nothing in the schemas needs exact big integers).

    #[derive(Debug, Clone, PartialEq)]
    pub enum JValue {
        Null,
        Bool(bool),
        Num(f64),
        Str(String),
        Arr(Vec<JValue>),
        Obj(Vec<(String, JValue)>),
    }

    impl JValue {
        pub fn as_str(&self) -> Option<&str> {
            match self {
                JValue::Str(s) => Some(s),
                _ => None,
            }
        }

        pub fn as_array(&self) -> Option<&[JValue]> {
            match self {
                JValue::Arr(items) => Some(items),
                _ => None,
            }
        }

        pub fn as_object(&self) -> Option<&[(String, JValue)]> {
            match self {
                JValue::Obj(entries) => Some(entries),
                _ => None,
            }
        }
    }

    /// Render a JSON string literal (RFC 8259 escaping).
    pub fn string(text: &str) -> String {
        let mut out = String::with_capacity(text.len() + 2);
        out.push('"');
        for c in text.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{0008}' => out.push_str("\\b"),
                '\u{000C}' => out.push_str("\\f"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    /// Parse one complete JSON document (trailing whitespace allowed,
    /// trailing garbage rejected).
    pub fn parse(text: &str) -> Result<JValue, String> {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            pos: 0,
        };
        let value = parser.value()?;
        parser.skip_ws();
        if parser.pos != parser.bytes.len() {
            return Err(format!("trailing data at byte {}", parser.pos));
        }
        Ok(value)
    }

    struct Parser<'a> {
        bytes: &'a [u8],
        pos: usize,
    }

    impl<'a> Parser<'a> {
        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.pos).copied()
        }

        fn bump(&mut self) -> Option<u8> {
            let byte = self.peek();
            if byte.is_some() {
                self.pos += 1;
            }
            byte
        }

        fn skip_ws(&mut self) {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.pos += 1;
            }
        }

        fn expect(&mut self, byte: u8) -> Result<(), String> {
            if self.bump() == Some(byte) {
                Ok(())
            } else {
                Err(format!("expected {:?} at byte {}", byte as char, self.pos))
            }
        }

        fn literal(&mut self, word: &str, value: JValue) -> Result<JValue, String> {
            if self.bytes[self.pos..].starts_with(word.as_bytes()) {
                self.pos += word.len();
                Ok(value)
            } else {
                Err(format!("expected `{word}` at byte {}", self.pos))
            }
        }

        fn value(&mut self) -> Result<JValue, String> {
            self.skip_ws();
            match self.peek() {
                Some(b'{') => self.object(),
                Some(b'[') => self.array(),
                Some(b'"') => Ok(JValue::Str(self.string()?)),
                Some(b't') => self.literal("true", JValue::Bool(true)),
                Some(b'f') => self.literal("false", JValue::Bool(false)),
                Some(b'n') => self.literal("null", JValue::Null),
                Some(byte) if byte == b'-' || byte.is_ascii_digit() => self.number(),
                other => Err(format!(
                    "unexpected {:?} at byte {}",
                    other.map(|b| b as char).unwrap_or('\0'),
                    self.pos
                )),
            }
        }

        fn object(&mut self) -> Result<JValue, String> {
            self.expect(b'{')?;
            let mut entries = Vec::new();
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok(JValue::Obj(entries));
            }
            loop {
                self.skip_ws();
                let key = self.string()?;
                self.skip_ws();
                self.expect(b':')?;
                let value = self.value()?;
                entries.push((key, value));
                self.skip_ws();
                match self.bump() {
                    Some(b',') => continue,
                    Some(b'}') => return Ok(JValue::Obj(entries)),
                    other => {
                        return Err(format!(
                            "expected `,` or `}}` in object, got {:?} at byte {}",
                            other.map(|b| b as char).unwrap_or('\0'),
                            self.pos
                        ));
                    }
                }
            }
        }

        fn array(&mut self) -> Result<JValue, String> {
            self.expect(b'[')?;
            let mut items = Vec::new();
            self.skip_ws();
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(JValue::Arr(items));
            }
            loop {
                items.push(self.value()?);
                self.skip_ws();
                match self.bump() {
                    Some(b',') => continue,
                    Some(b']') => return Ok(JValue::Arr(items)),
                    other => {
                        return Err(format!(
                            "expected `,` or `]` in array, got {:?} at byte {}",
                            other.map(|b| b as char).unwrap_or('\0'),
                            self.pos
                        ));
                    }
                }
            }
        }

        fn string(&mut self) -> Result<String, String> {
            self.expect(b'"')?;
            let mut out = String::new();
            loop {
                let Some(byte) = self.bump() else {
                    return Err("unterminated string".to_string());
                };
                match byte {
                    b'"' => return Ok(out),
                    b'\\' => {
                        let Some(escape) = self.bump() else {
                            return Err("unterminated escape".to_string());
                        };
                        match escape {
                            b'"' => out.push('"'),
                            b'\\' => out.push('\\'),
                            b'/' => out.push('/'),
                            b'b' => out.push('\u{0008}'),
                            b'f' => out.push('\u{000C}'),
                            b'n' => out.push('\n'),
                            b'r' => out.push('\r'),
                            b't' => out.push('\t'),
                            b'u' => {
                                let high = self.hex4()?;
                                let code = if (0xD800..0xDC00).contains(&high) {
                                    // surrogate pair
                                    if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                        return Err("unpaired surrogate in \\u escape".to_string());
                                    }
                                    let low = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&low) {
                                        return Err("invalid low surrogate".to_string());
                                    }
                                    0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
                                } else if (0xDC00..0xE000).contains(&high) {
                                    return Err("unpaired low surrogate".to_string());
                                } else {
                                    high
                                };
                                let ch =
                                    char::from_u32(code).ok_or("invalid \\u escape codepoint")?;
                                out.push(ch);
                            }
                            other => {
                                return Err(format!("unsupported escape `\\{}`", other as char));
                            }
                        }
                    }
                    byte if byte < 0x20 => {
                        return Err("raw control character in string".to_string());
                    }
                    byte => {
                        // Re-assemble UTF-8 multibyte sequences.
                        if byte < 0x80 {
                            out.push(byte as char);
                        } else {
                            let width = utf8_width(byte)
                                .ok_or_else(|| format!("invalid UTF-8 lead byte {byte:#x}"))?;
                            let start = self.pos - 1;
                            let end = start + width;
                            if end > self.bytes.len() {
                                return Err("truncated UTF-8 sequence".to_string());
                            }
                            let chunk = std::str::from_utf8(&self.bytes[start..end])
                                .map_err(|_| "invalid UTF-8 sequence".to_string())?;
                            out.push_str(chunk);
                            self.pos = end;
                        }
                    }
                }
            }
        }

        fn hex4(&mut self) -> Result<u32, String> {
            let mut value = 0u32;
            for _ in 0..4 {
                let Some(byte) = self.bump() else {
                    return Err("truncated \\u escape".to_string());
                };
                let digit = (byte as char)
                    .to_digit(16)
                    .ok_or_else(|| format!("invalid hex digit {:?}", byte as char))?;
                value = value * 16 + digit;
            }
            Ok(value)
        }

        fn number(&mut self) -> Result<JValue, String> {
            let start = self.pos;
            if self.peek() == Some(b'-') {
                self.pos += 1;
            }
            match self.peek() {
                Some(b'0') => {
                    self.pos += 1;
                }
                Some(byte) if byte.is_ascii_digit() => {
                    while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                        self.pos += 1;
                    }
                }
                _ => return Err(format!("invalid number at byte {start}")),
            }
            if self.peek() == Some(b'.') {
                self.pos += 1;
                if !matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                    return Err("fraction without digits".to_string());
                }
                while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
            if matches!(self.peek(), Some(b'e' | b'E')) {
                self.pos += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
                if !matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                    return Err("exponent without digits".to_string());
                }
                while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
            let text = std::str::from_utf8(&self.bytes[start..self.pos])
                .map_err(|_| "number is not UTF-8".to_string())?;
            text.parse::<f64>()
                .map(JValue::Num)
                .map_err(|_| format!("unparsable number {text:?}"))
        }
    }

    fn utf8_width(byte: u8) -> Option<usize> {
        match byte {
            0xC0..=0xDF => Some(2),
            0xE0..=0xEF => Some(3),
            0xF0..=0xF7 => Some(4),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Time + root discovery
// ---------------------------------------------------------------------------

/// A UTC ISO-8601 timestamp (std-only: the system clock + the
/// well-known civil-date conversion — no external crates).
fn iso8601_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = now / 86_400;
    let secs = now % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Days-since-epoch → (y, m, d) — Howard Hinnant's civil-from-days.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    /// A fixture matrix with the shipped shape: two rows, single-line
    /// status/notes/evidence, plus a leading [meta] table.
    const MATRIX: &str = r#"[meta]
schema_version = 1

[[feature]]
id = "container-x"
category = "container"
priority = "P0"
phase_target = 5
status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }
evidence = "crates/demux/src/x/** (card 5.D1)"
notes = "existing note"

[[feature]]
id = "codec-y"
category = "audio-codec"
priority = "P1"
phase_target = 5
status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }
evidence = "crates/codecs/src/audio/y/** (card 5.C1)"
notes = "existing note two"
"#;

    fn fixture() -> (std::path::PathBuf, String, String, String) {
        let root =
            std::env::temp_dir().join(format!("xtask-cm-{}-{:x}", std::process::id(), nano()));
        std::fs::create_dir_all(root.join("docs/corpus")).must();
        let matrix_rel = "docs/feature-matrix.toml".to_string();
        let outbox_rel = "docs/corpus/outbox.json".to_string();
        let ledger_rel = "docs/corpus/applied.jsonl".to_string();
        std::fs::write(root.join(&matrix_rel), MATRIX).must();
        (root, outbox_rel, ledger_rel, matrix_rel)
    }

    fn outbox(body: &str) -> String {
        format!(
            "{{\"schema\":\"{OUTBOX_SCHEMA}\",\"generated\":\"2026-11-10T00:00:00Z\",\"transitions\":[{body}]}}"
        )
    }

    fn proposal(row: &str, platform: &str, from: &str, to: &str) -> String {
        format!(
            "{{\"row\":\"{row}\",\"platform\":\"{platform}\",\"from\":\"{from}\",\"to\":\"{to}\",\
             \"reason\":\"the reason \\u2014 with an em dash\",\"next_step\":\"the next step\",\
             \"evidence\":\"docs/corpus/compat-matrix-20261110.md (this run)\"}}"
        )
    }

    /// A proposal with an explicit evidence field (the IN-8 tests:
    /// coalesced citations may differ).
    fn proposal_evidence(
        row: &str,
        platform: &str,
        from: &str,
        to: &str,
        evidence: &str,
    ) -> String {
        format!(
            "{{\"row\":\"{row}\",\"platform\":\"{platform}\",\"from\":\"{from}\",\"to\":\"{to}\",\
             \"reason\":\"the reason \\u2014 with an em dash\",\"next_step\":\"the next step\",\
             \"evidence\":\"{evidence}\"}}"
        )
    }

    fn nano() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0)
    }

    #[test]
    fn json_parser_roundtrips_the_outbox_shapes() {
        let text = outbox(&proposal("container-x", "linux", "implemented", "partial"));
        let value = json::parse(&text).must();
        let transitions = value
            .as_object()
            .must()
            .iter()
            .find(|(k, _)| k == "transitions")
            .and_then(|(_, v)| v.as_array())
            .must();
        assert_eq!(transitions.len(), 1);
        let reason = transitions[0]
            .as_object()
            .must()
            .iter()
            .find(|(k, _)| k == "reason")
            .and_then(|(_, v)| v.as_str())
            .must();
        assert_eq!(reason, "the reason — with an em dash");
        // trailing garbage rejected
        assert!(json::parse(&format!("{text} x")).is_err());
        // surrogate pair
        let pair = json::parse("\"\\ud83d\\ude00\"").must();
        assert_eq!(pair.as_str(), Some("\u{1F600}"));
        // unpaired surrogate rejected
        assert!(json::parse("\"\\ud83d\"").is_err());
    }

    #[test]
    fn outbox_parser_enforces_the_schema() {
        assert!(
            Outbox::parse(&outbox(&proposal("a-b", "linux", "implemented", "partial"))).is_ok()
        );
        let wrong_schema = outbox(&proposal("a-b", "linux", "implemented", "partial"))
            .replace(OUTBOX_SCHEMA, "other/1");
        assert!(Outbox::parse(&wrong_schema).is_err());
        assert!(Outbox::parse("{\"schema\":\"compat-matrix/transitions/1\"}").is_err());
    }

    #[test]
    fn applies_a_transition_end_to_end() {
        let (root, outbox_rel, ledger_rel, matrix_rel) = fixture();
        std::fs::write(
            root.join(&outbox_rel),
            outbox(&proposal("container-x", "linux", "implemented", "partial")),
        )
        .must();
        // the cited evidence must exist (R4 reuse)
        std::fs::write(root.join("docs/corpus/compat-matrix-20261110.md"), "# run").must();

        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);
        assert_eq!(outcome.applied.len(), 1);

        let written = std::fs::read_to_string(root.join(&matrix_rel)).must();
        assert!(
            written.contains("linux = \"partial\""),
            "status cell not flipped:\n{written}"
        );
        assert!(
            written.contains("compat-matrix gap triage (issue #53, card 8.B2"),
            "notes append missing:\n{written}"
        );
        assert!(
            written.contains("the reason — with an em dash"),
            "reason not carried into notes:\n{written}"
        );
        assert!(
            written.contains("; docs/corpus/compat-matrix-20261110.md (this run)"),
            "evidence append missing"
        );
        // other platforms + other rows untouched
        assert!(written.contains("windows = \"implemented\""));
        assert!(written.contains("codec-y"));

        // the ledger line parses and carries the keys
        let ledger = std::fs::read_to_string(root.join(&ledger_rel)).must();
        let line = ledger.lines().next().must();
        let value = json::parse(line).must();
        let object = value.as_object().must();
        let get = |key: &str| {
            object
                .iter()
                .find(|(k, _)| k == key)
                .and_then(|(_, v)| v.as_str())
                .must()
                .to_string()
        };
        assert_eq!(get("row"), "container-x");
        assert_eq!(get("from"), "implemented");
        assert_eq!(get("to"), "partial");
        assert_eq!(get("outbox"), outbox_rel);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn dry_run_writes_nothing() {
        let (root, outbox_rel, ledger_rel, matrix_rel) = fixture();
        std::fs::write(
            root.join(&outbox_rel),
            outbox(&proposal("container-x", "linux", "implemented", "partial")),
        )
        .must();
        std::fs::write(root.join("docs/corpus/compat-matrix-20261110.md"), "# run").must();
        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, true).must();
        assert!(outcome.rejected.is_empty());
        assert_eq!(
            std::fs::read_to_string(root.join(&matrix_rel)).must(),
            MATRIX
        );
        assert!(!root.join(&ledger_rel).exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rejects_stale_from_and_writes_nothing() {
        let (root, outbox_rel, ledger_rel, matrix_rel) = fixture();
        // the matrix says implemented; the outbox claims partial -> partial
        std::fs::write(
            root.join(&outbox_rel),
            outbox(&proposal("container-x", "linux", "partial", "blocked")),
        )
        .must();
        std::fs::write(root.join("docs/corpus/compat-matrix-20261110.md"), "# run").must();
        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert_eq!(outcome.rejected.len(), 1);
        assert!(
            outcome.rejected[0].contains("STALE"),
            "{:?}",
            outcome.rejected
        );
        assert_eq!(
            std::fs::read_to_string(root.join(&matrix_rel)).must(),
            MATRIX
        );
        assert!(!root.join(&ledger_rel).exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rejects_verified_transitions() {
        let (root, outbox_rel, ledger_rel, matrix_rel) = fixture();
        std::fs::write(
            root.join(&outbox_rel),
            outbox(&proposal("container-x", "linux", "implemented", "verified")),
        )
        .must();
        std::fs::write(root.join("docs/corpus/compat-matrix-20261110.md"), "# run").must();
        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert_eq!(outcome.rejected.len(), 1);
        assert!(outcome.rejected[0].contains("verified"));
        assert_eq!(
            std::fs::read_to_string(root.join(&matrix_rel)).must(),
            MATRIX
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rejects_unknown_rows_platforms_and_unresolved_evidence() {
        let (root, outbox_rel, ledger_rel, matrix_rel) = fixture();
        std::fs::write(root.join("docs/corpus/compat-matrix-20261110.md"), "# run").must();
        // unknown row
        std::fs::write(
            root.join(&outbox_rel),
            outbox(&proposal("container-zz", "linux", "implemented", "partial")),
        )
        .must();
        assert_eq!(
            apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false)
                .must()
                .rejected
                .len(),
            1
        );
        // unknown platform
        std::fs::write(
            root.join(&outbox_rel),
            outbox(&proposal("container-x", "bsd", "implemented", "partial")),
        )
        .must();
        assert_eq!(
            apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false)
                .must()
                .rejected
                .len(),
            1
        );
        // evidence citing a missing path
        std::fs::write(
            root.join(&outbox_rel),
            outbox(
                "{\"row\":\"container-x\",\"platform\":\"linux\",\"from\":\"implemented\",\
                 \"to\":\"partial\",\"reason\":\"r\",\"next_step\":\"n\",\
                 \"evidence\":\"docs/corpus/missing-evidence.md\"}",
            ),
        )
        .must();
        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert_eq!(outcome.rejected.len(), 1);
        assert!(outcome.rejected[0].contains("missing-evidence.md"));
        assert_eq!(
            std::fs::read_to_string(root.join(&matrix_rel)).must(),
            MATRIX
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn coalesces_same_row_and_platform_and_is_idempotent() {
        let (root, outbox_rel, ledger_rel, matrix_rel) = fixture();
        std::fs::write(root.join("docs/corpus/compat-matrix-20261110.md"), "# run").must();
        let body = format!(
            "{},{}",
            proposal("codec-y", "linux", "implemented", "partial"),
            proposal("codec-y", "linux", "implemented", "partial")
        );
        std::fs::write(root.join(&outbox_rel), outbox(&body)).must();

        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);
        assert_eq!(
            outcome.applied.len(),
            1,
            "two proposals coalesce to one flip"
        );
        let written = std::fs::read_to_string(root.join(&matrix_rel)).must();
        assert!(written.contains("the reason — with an em dash | the reason — with an em dash"));
        // TWO ledger lines (one per original proposal)
        let ledger = std::fs::read_to_string(root.join(&ledger_rel)).must();
        assert_eq!(ledger.lines().count(), 2);

        // re-apply: everything is already-applied, nothing changes
        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert!(outcome.rejected.is_empty());
        assert_eq!(outcome.already, 2);
        assert_eq!(outcome.applied.len(), 0);
        assert_eq!(
            std::fs::read_to_string(root.join(&ledger_rel)).must(),
            ledger
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// IN-8 (integration audit R9): coalesced proposals of one group
    /// cite the same run report — an identical evidence citation is
    /// appended ONCE, while distinct citations are all kept.
    #[test]
    fn coalesced_evidence_is_deduped() {
        let (root, outbox_rel, ledger_rel, matrix_rel) = fixture();
        std::fs::write(root.join("docs/corpus/compat-matrix-20261110.md"), "# run").must();
        std::fs::write(root.join("docs/corpus/other-run.md"), "# other").must();

        // Same evidence twice (the aac-lc shape: two findings, one run
        // report) — one citation.
        let same = format!(
            "{},{}",
            proposal("codec-y", "linux", "implemented", "partial"),
            proposal("codec-y", "linux", "implemented", "partial")
        );
        std::fs::write(root.join(&outbox_rel), outbox(&same)).must();
        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);
        let written = std::fs::read_to_string(root.join(&matrix_rel)).must();
        assert_eq!(
            written
                .matches("docs/corpus/compat-matrix-20261110.md (this run)")
                .count(),
            1,
            "an identical evidence citation is appended once, not per proposal"
        );

        // Distinct evidence within one group — both citations kept.
        let distinct = format!(
            "{},{}",
            proposal_evidence(
                "container-x",
                "linux",
                "implemented",
                "partial",
                "docs/corpus/other-run.md"
            ),
            proposal("container-x", "linux", "implemented", "partial")
        );
        std::fs::write(root.join(&outbox_rel), outbox(&distinct)).must();
        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);
        let written = std::fs::read_to_string(root.join(&matrix_rel)).must();
        let line = written
            .lines()
            .find(|l| l.starts_with("evidence") && l.contains("demux/src/x"))
            .must();
        assert!(
            line.contains("docs/corpus/other-run.md"),
            "a distinct citation is kept: {line}"
        );
        assert!(
            line.contains("docs/corpus/compat-matrix-20261110.md (this run)"),
            "a distinct citation is kept: {line}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// IN-8 (integration audit R9), the re-cite limb: a LATER outbox (a
    /// different file, so the ledger's already-applied key cannot catch
    /// it) that cites an artifact already on the evidence line leaves
    /// the evidence cell byte-identical — double-apply is a no-op on
    /// the evidence, even across a further status flip.
    #[test]
    fn recited_evidence_is_not_reappended() {
        let (root, outbox_rel, ledger_rel, matrix_rel) = fixture();
        std::fs::write(root.join("docs/corpus/compat-matrix-20261110.md"), "# run").must();
        let second_outbox = "docs/corpus/outbox-2.json".to_string();

        // First apply: implemented -> partial, citing the run report.
        std::fs::write(
            root.join(&outbox_rel),
            outbox(&proposal("codec-y", "linux", "implemented", "partial")),
        )
        .must();
        let outcome = apply(&root, &outbox_rel, &matrix_rel, &ledger_rel, false).must();
        assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);
        let after_first = std::fs::read_to_string(root.join(&matrix_rel)).must();
        let evidence_after_first = after_first
            .lines()
            .find(|l| l.starts_with("evidence") && l.contains("codecs/src/audio/y"))
            .must()
            .to_string();

        // Second apply: partial -> blocked from a NEW outbox file, same
        // citation (a triage round re-citing the same run report).
        std::fs::write(
            root.join(&second_outbox),
            outbox(&proposal("codec-y", "linux", "partial", "blocked")),
        )
        .must();
        let outcome = apply(&root, &second_outbox, &matrix_rel, &ledger_rel, false).must();
        assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);

        let after_second = std::fs::read_to_string(root.join(&matrix_rel)).must();
        let evidence_after_second = after_second
            .lines()
            .find(|l| l.starts_with("evidence") && l.contains("codecs/src/audio/y"))
            .must()
            .to_string();
        assert_eq!(
            evidence_after_first, evidence_after_second,
            "a citation already present is not re-appended"
        );
        assert_eq!(
            evidence_after_second
                .matches("docs/corpus/compat-matrix-20261110.md (this run)")
                .count(),
            1
        );
        // The flip itself did land (the no-op is evidence-only).
        assert!(
            after_second.contains("linux = \"blocked\""),
            "the second flip applied: status is blocked"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn scans_blocks_and_finds_rows() {
        let blocks = scan_blocks(MATRIX).must();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].id, "container-x");
        assert!(blocks[0].notes_line.is_some());
        assert_ne!(blocks[0].status_line, usize::MAX);
        // a statusless row is a loud error
        assert!(scan_blocks("[[feature]]\nid = \"x-y\"\n").is_err());
    }

    #[test]
    fn escape_toml_covers_the_writing_surface() {
        assert_eq!(escape_toml("plain"), "plain");
        assert_eq!(escape_toml("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }
}
