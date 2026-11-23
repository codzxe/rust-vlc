# AUDIT: tools/xtask/src/cmd/compat_matrix.rs
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-23 |
| Lines / functions | 1797 / 35 (+6 test helpers, 12 tests, `json` submodule) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (dev tooling: registered in `tools/xtask/src/cmd/mod.rs:179-184`, invoked via `cargo xtask`; not a dependency of lumen-cli/desktop release lane). Production usage PROVEN (ledger + matrix writes dated 2026-11-10). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 5 S4: 4 |
| Commands run | `cargo test -p xtask --bin xtask compat_matrix` → 12 passed/0 failed; `cargo run -q -p xtask -- compat-matrix apply-transitions ...` ×4 (noteless-row probe → exit 2 "added/removed keys"; normal-row apply → exit 0 APPLIED; control-char probe → exit 0 + raw `\x01` written, tomllib rejects; find_root probe → root found, exit 2 on same noteless fixture); `python3` tomllib parse of probe output → FAIL "Illegal character '\x01'"; rg/sed/awk inventory of matrix (252 rows, 252 evidence, 252 single-line status, 240 notes → 12 noteless rows). |

## 1. One-paragraph verdict
This is real code doing real work, not spreadsheet theater: the writer is registered, its 12 in-module tests pass (I ran them), and it has actually been used in production — `docs/corpus/compat-matrix-transitions-applied.jsonl` records 6 applied proposals (2026-11-10), the corresponding matrix rows carry the machine-written notes verbatim, and the producer side exists (`tests/compat/src/report.rs:800-811` emits the outbox; `.github/workflows/compat-matrix.yml` runs the corpus through the real open/decode path and uploads the outbox without editing the matrix). The no-self-verification policy is real: `to = "verified"` is rejected in code and the shipped matrix contains 0/252 verified cells. However, I executed the tool and proved two S2 defects: (1) the notes-insertion path for rows without a `notes` line is self-defeating — `edit_row` inserts the line, then `verify_diff` rejects the plan as "added/removed keys", so the 12 noteless rows of the shipped matrix (4.8%) are unwritable through the only sanctioned writer; (2) `escape_toml` does not escape control characters (other than \n\r\t), so an outbox reason containing `\u0001` is written as a RAW control byte into the matrix with exit 0 "APPLIED" — the file becomes spec-invalid TOML (python tomllib: "Illegal character '\x01'"), and the advertised self-check is blind to it because the re-parser (`toml_lite`) is equally lenient and `verify_diff` skips content comparison of status/notes/evidence on edited rows entirely.

## 2. What actually works (verified)
- `cargo test -p xtask --bin xtask compat_matrix` → `test result: ok. 12 passed; 0 failed` (matches baseline cargo-test-workspace.txt:6591-6606).
- End-to-end apply via the real binary on a temp root (row WITH notes): `applied: codec-audio-mp3.linux: implemented -> partial`, exit 0; matrix status cell flipped, notes appended, evidence appended, ledger JSONL line written and well-formed (all checked by reading the output files).
- Rejection paths work: stale `from` and `verified` rejected with nothing written (unit tests; I read the branch code).
- `find_root` success path: invoked without `--root` from a temp dir containing Cargo.toml+docs/ → root resolved (run proceeded to the row-edit stage).
- Production usage: `docs/corpus/compat-matrix-transitions-applied.jsonl` (6 lines, 2 outboxes) matches the live matrix content (`docs/feature-matrix.toml` rows container-asf-wmv:214-217, codec-audio-pcm, codec-audio-aac-lc etc. all show `linux = "partial"` + the 20:07:51Z/20:08:26Z machine notes).

## 3. Findings
F-001 [S2]
Where: tools/xtask/src/cmd/compat_matrix.rs:675-682 and 789-793
Code:
```
        None => {
            // R6: partial needs notes — insert a notes line right after
            // the evidence line so the row stays shape-legal.
            lines.insert(
                block.evidence_line + 1,
                format!("notes = \"{}\"", escape_toml(&note)),
            );
        }
```
```
        if entries_before.len() != entries_after.len() {
            return Err(XtaskError::internal(format!(
                "the rewrite added/removed keys on row {id} — only status/notes/evidence are \
                 writable"
            )));
```
What is wrong: the notes-insertion branch adds a NEW `notes` entry to a row that had none, which changes the parsed entry count, which `verify_diff` treats as a hard error. The branch can therefore never produce a writable plan — it always aborts the whole invocation. Why it matters: the shipped `docs/feature-matrix.toml` has 12 rows without a `notes` line (codec-audio-speex, codec-audio-musepack, subtitle-teletext, subtitle-kate, protocol-nfs, protocol-ftp-sftp, protocol-mmsh-mms, sd-upnp, sd-mdns-renderers, sd-sap, ui-dialog-open-capture, hotkey-nav-disc — 252 rows but only 240 `notes =` lines), so 4.8% of the matrix is unwritable through "the ONLY status writer", and the module doc's claim that this path satisfies "R6 (partial needs notes) by construction" (L44) is false — R6-by-construction fails by construction. The failure is also misclassified as an internal error (exit 2) with a message ("only status/notes/evidence are writable") that contradicts what the tool itself just did (wrote a notes line). PROVEN by execution: `cargo run -q -p xtask -- compat-matrix apply-transitions --outbox docs/corpus/outbox.json --root /tmp/xtask-a03` → exit 2, `xtask error: internal error: the rewrite added/removed keys on row protocol-nfs — only status/notes/evidence are writable`. No unit test covers a noteless row (the fixture gives every row notes), so this survives the whole test suite. What correct looks like: verify_diff must compare entry SETS, allowing exactly one added `notes` key on rows whose group is applied and which had no notes; or edit_row must not insert and the validation must reject noteless rows upfront with a real rejection message.
Confidence: HIGH — Verification: ran (output above).

F-002 [S2]
Where: tools/xtask/src/cmd/compat_matrix.rs:744-757 (writer) vs 974-977 (the JSON escaper that does it right)
Code:
```
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
```
What is wrong: every other control character (U+0000–U+0007, U+000B, U+000E–U+001F, U+007F) is pushed RAW into a TOML basic string, which the TOML v1.0 spec forbids. The tool then re-parses with `toml_lite`, whose `parse_string` (toml_lite.rs:437-468) accepts any raw char, so the advertised pre-write self-check passes and the tool reports success. Why it matters: the outbox JSON fully legitimately carries these — the tool's own JSON parser decodes `\u0001` (compat_matrix.rs:1135-1154) and its own ledger writer escapes them correctly (`c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32))`, L976); only the matrix write is unsanitized. PROVEN by execution: outbox reason `"probe A\u0001B ctrl"` → exit 0 `result: APPLIED`, and the written notes line contains a raw 0x01 byte (`cat -A`: `probe A^AB ctrl`); `python3 -c "import tomllib; tomllib.load(...)"` → `TOMLParseError: Illegal character '\x01' (at line 11, column 186)`. The repo's central evidence document is silently corrupted to spec-invalid TOML. Blast-radius bound: `.github/workflows/ci.yml:283-316` re-validates the matrix with strict tomllib in CI, so the corruption is caught post-commit — but the designated integrity gate (this tool, "re-parsed and diffed ... before it is written", L55-57) is the thing that failed. What correct looks like: escape_toml must emit `\uXXXX` for all control chars (copy the L976 limb), and toml_lite should reject raw control chars so the self-check has teeth.
Confidence: HIGH — Verification: ran (apply + tomllib parse both pasted above).

F-003 [S3]
Where: tools/xtask/src/cmd/compat_matrix.rs:799-809 (and doc claim L55-57)
Code:
```
            if matches!(key.as_str(), "status" | "notes" | "evidence") {
                continue;
            }
            if value_before != value_after {
```
What is wrong: for edited rows the diff skips ALL comparison of `status`, `notes`, `evidence` — it only asserts the target platform's status equals `to` (L811-822). The doc claims "only the intended status/notes/evidence cells may differ" (L55-57); the implemented check cannot distinguish intended from unintended changes within those three keys: a mangled OTHER platform's status cell, a corrupted notes append, or a mangled evidence string on an edited row all pass. Given the module's own history (the IN-8 double-append bug, see F-004) this is not a hypothetical defense-in-depth. What correct looks like: for edited rows, assert every OTHER platform's status is byte-identical and notes/evidence differ only by the exact appended suffixes the tool constructed.
Confidence: HIGH — Verification: read.

F-004 [S3]
Where: tools/xtask/src/cmd/compat_matrix.rs:21-23 (contract claim); contradicted by docs/feature-matrix.toml:674-680 and scripts/q2v_matrix_backfill.py
Code:
```
//! Status transitions for `docs/feature-matrix.toml` flow ONLY through a
//! transitions outbox (schema `compat-matrix/transitions/1`, emitted by
```
What is wrong: the "ONLY" is historically false twice over. (1) docs/feature-matrix.toml:674-680 documents a hand repair: "The duplicate segment was removed by hand (the tool only appends)" (IN-8 / R10-1, 2026-11-10). (2) `scripts/q2v_matrix_backfill.py:39,244` is a second, regex-based status writer ("MATRIX = \"docs/feature-matrix.toml\"", rewrites `status = {...}` lines) committed as the Q2-V backfill reproducer. mod.rs:30 doubles down ("the ONLY status writer, never hand-edited"). There is no mechanical gate that detects an out-of-band edit (no hash, no ledger cross-check on read), so the contract is enforced only by convention. Why it matters: the whole 8.B2 governance story ("the tool decides, never the author") rests on this invariant. What correct looks like: a checker (feature-audit) that flags matrix diffs whose (row, platform, from→to) is absent from the applied-ledger.
Confidence: HIGH — Verification: read (all three files quoted/cited).

F-005 [S3]
Where: tools/xtask/src/cmd/compat_matrix.rs:229
Code:
```
        if ledgered.contains(&(outbox_rel.to_string(), key.0.clone(), key.1.clone())) {
            outcome.already += 1;
            continue;
```
What is wrong: idempotency is keyed by the outbox path AS TYPED on the command line. `--outbox docs/x.json` and `--outbox ./docs/x.json` are different keys, so the same outbox re-applied under a different spelling is not recognized as already-applied. Normally the STALE check then rejects it — but after an A→B→A flip-back the re-apply is valid again and double-writes the machine notes sentence. No path normalization (`components()`/canonicalize) is performed.
Confidence: HIGH — Verification: read (path reasoning; not executed).
F-006 [S3]
Where: tools/xtask/src/cmd/compat_matrix.rs:1040-1056 (value → object/array → value recursion, unbounded)
Code:
```
                Some(b'{') => self.object(),
                Some(b'[') => self.array(),
```
What is wrong: the JSON parser recurses per nesting level with no depth limit. An outbox (a committed, reviewable file — but authored by a runner or a triage paste) with e.g. 100k nested arrays aborts the process with a stack overflow instead of returning the documented exit 2; the exit-code contract (L64-66) is broken by a crash. Why it matters: the tool is the integrity gate and should fail typed on any input. What correct looks like: a depth counter (limit ~128) returning `Err`.
Confidence: MEDIUM — Verification: read (not executed; overflow depends on stack size).

F-007 [S3]
Where: tools/xtask/src/cmd/compat_matrix.rs:296-333
Code:
```
    std::fs::write(&matrix_path, &plan)
        .map_err(|e| XtaskError::io(format!("writing {}", matrix_path.display()), e))?;
```
What is wrong: the repo's central evidence file is overwritten in place — no temp-file+rename, no fsync — and the matrix is written BEFORE the ledger append (L296 vs L325-333). A crash between the two leaves the flip landed with no ledger record; a crash mid-write can truncate the matrix. The all-or-nothing guarantee is documented for rejections ("any rejection writes NOTHING") but there is no atomicity across the two files or against partial writes. Recoverable via git, but the tool positions itself as the trustworthy writer.
Confidence: HIGH — Verification: read.

F-008 [S4]
Where: tools/xtask/src/cmd/compat_matrix.rs:138-151
Code:
```
                let Some(value) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "{flag} requires a path\n{USAGE}"
                    )));
                };
```
What is wrong: option values are consumed blindly; `--outbox --dry-run` sets the outbox path to the literal string `--dry-run` and silently drops the dry-run flag, failing later with a confusing IO error (exit 2) instead of a usage error. No leading-dash validation on values.
Confidence: HIGH — Verification: read.

F-009 [S4]
Where: tools/xtask/src/cmd/compat_matrix.rs:438-468 (untested branches); tests at 1416-1795
Code:
```
    if !STATUSES.contains(&first.to.as_str()) {
```
What is wrong: no test exercises (a) an out-of-vocabulary `to` status, (b) coalesced proposals that disagree on `to`/`from` (L455-468), (c) CLI parsing in `run()` at all (`tools/xtask/tests/cli.rs` contains zero compat tests — `rg -n 'compat' tools/xtask/tests/cli.rs` → empty), (d) an unparseable ledger. Mutation reasoning: deleting the vocabulary check or the disagree-checks breaks zero tests. The 12 existing tests are otherwise honest end-to-end FS tests that would catch real regressions (F-001/F-002 prove the gaps, not the tests, are the problem).
Confidence: HIGH — Verification: read + ran the suite (12 pass).

F-010 [S4]
Where: tools/xtask/src/cmd/compat_matrix.rs:828-841
Code:
```
        map.insert(id, src.entries.clone());
```
What is wrong: `row_map` (BTreeMap) silently collapses duplicate row ids; `current_status`/`scan_blocks`/`edit_row` all pick the FIRST match. In a duplicate-id matrix the tool would edit the first row while verify_diff compares the collapsed map — the second duplicate keeps the stale status and the check passes. No duplicate-id detection anywhere in the writer (the ci.yml structural check does check R1 uniqueness, post-commit). Edge case, limited blast radius.
Confidence: MEDIUM — Verification: read.

F-011 [S4]
Where: tools/xtask/src/cmd/compat_matrix.rs:723-729
Code:
```
    let Some(quote) = line.rfind('"') else {
        return Err(format!("the line has no closing TOML string quote: {line}"));
    };
    line.insert_str(quote, &escape_toml(addition));
```
What is wrong: `rfind('"')` assumes the last quote on the line closes the string; a trailing comment containing a quote (`evidence = "x" # see "doc"`) would make the append land inside the comment, silently corrupting the row. Verified the shipped matrix has zero trailing comments on evidence lines (`rg -c 'evidence = ".*"  *#' docs/feature-matrix.toml` → none), so this is latent.
Confidence: HIGH — Verification: read + rg on the shipped matrix.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 124-178 | REAL | — | CLI parse+dispatch; executed ×4 by me; no test covers it (F-009) |
| `apply` | 193-345 | REAL | — | full pipeline; executed (apply, dry-run path via tests, rejection path) |
| `Outcome` (struct) | 181-189 | REAL | — | data holder, populated correctly |
| `Outbox::parse` | 371-417 | REAL | — | schema+field checks; tested; first-wins on duplicate JSON keys (deterministic, RFC-soft) |
| `validate` | 425-488 | REAL | — | platform/vocab/verified/row/stale/evidence checks; verified+stale branches tested, vocab+disagree branches untested (F-009) |
| `row_exists` | 491-497 | REAL | — | linear scan, fine for 252 rows |
| `current_status` | 500-518 | REAL | — | first-match on duplicate ids (F-010) |
| `scan_blocks` | 539-610 | REAL | — | loud errors on bad shape; tested |
| `edit_row` | 614-719 | REAL | S2 | needle replace is precise; notes-insertion branch self-defeating (F-001) |
| `append_into_string` | 723-729 | REAL | S4 | rfind-quote assumption (F-011) |
| `string_value_of` | 733-741 | REAL | — | reuses toml_lite, correct |
| `escape_toml` | 744-757 | REAL | S2 | control-char gap, proven by execution (F-002) |
| `verify_diff` | 762-825 | REAL | S2/S3 | entry-count check kills F-001; skips status/notes/evidence comparison on edited rows (F-003) |
| `row_map` | 828-841 | REAL | S4 | duplicate-id collapse (F-010) |
| `parse_ledger` | 848-878 | REAL | — | strict, loud; untested branch (F-009d) |
| `render_ledger_line` | 881-917 | REAL | — | executed; ledger line verified well-formed |
| `json::JValue::{as_str,as_array,as_object}` | 941-961 | REAL | — | trivial accessors |
| `json::string` | 964-982 | REAL | — | correct RFC 8259 escaping incl. controls (the irony: the ledger is clean, the matrix is not) |
| `json::parse` | 986-997 | REAL | — | trailing-data rejection tested |
| `Parser::{peek,bump,skip_ws,expect}` | 1005-1029 | REAL | — | byte-level helpers |
| `Parser::literal` | 1031-1038 | REAL | — | true/false/null; prefix-matches then caught by structure checks |
| `Parser::value` | 1040-1056 | REAL | S3 | unbounded recursion (F-006) |
| `Parser::object` | 1058-1086 | REAL | — | RFC-correct comma/brace handling; duplicate keys allowed (first-wins upstream) |
| `Parser::array` | 1088-1111 | REAL | — | RFC-correct |
| `Parser::string` | 1113-1184 | REAL | — | full escape set, surrogate pairs, UTF-8 reassembly; tested |
| `Parser::hex4` | 1186-1198 | REAL | — | correct |
| `Parser::number` | 1200-1242 | REAL | — | RFC 8259 grammar correct (leading-zero, bare '.', bare 'e' all rejected); f64 overflow→inf accepted (unused by schemas) |
| `utf8_width` | 1245-1252 | REAL | — | 0xC0/0xC1/0xF5+ rejected downstream by from_utf8 |
| `iso8601_now` | 1261-1275 | REAL | — | executed; timestamps in my probe output match UTC |
| `civil_from_days` | 1278-1289 | REAL | — | Hinnant algorithm; output dates correct |
| `find_root` | 1291-1308 | REAL | — | success path executed (probe ran root-less from temp root) |
| tests `Must`/`fixture`/`outbox`/`proposal`/`proposal_evidence`/`nano` | 1318-1414 | REAL | — | honest helpers; temp dirs, real FS |
| 12 tests | 1416-1795 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| json_parser_roundtrips_the_outbox_shapes | YES | real parse of real shapes; trailing-garbage + surrogate-pair negatives |
| outbox_parser_enforces_the_schema | YES | wrong-schema and missing-transitions both rejected |
| applies_a_transition_end_to_end | YES | asserts the written file's status/notes/evidence + ledger content |
| dry_run_writes_nothing | YES | byte-compares matrix to fixture, asserts ledger absent |
| rejects_stale_from_and_writes_nothing | YES | asserts STALE message + byte-identical matrix + no ledger |
| rejects_verified_transitions | YES | asserts rejection + untouched matrix |
| rejects_unknown_rows_platforms_and_unresolved_evidence | YES | three independent rejections asserted; matrix byte-identical |
| coalesces_same_row_and_platform_and_is_idempotent | YES | asserts 1 flip / 2 ledger lines / already=2 on re-apply / ledger byte-stable |
| coalesced_evidence_is_deduped | YES | counts exact citation occurrences (the IN-8 regression) |
| recited_evidence_is_not_reappended | YES | byte-compares evidence cell across two applies + asserts second flip landed |
| scans_blocks_and_finds_rows | YES | block layout asserted; statusless row errors |
| escape_toml_covers_the_writing_surface | NO (for control chars) | only quotes/backslash/newline asserted — this is the test hole F-002 slipped through |
All 12 executed by me: `12 passed; 0 failed`. Note what the suite does NOT cover: noteless rows (F-001 slipped through), control chars, vocab/disagree branches, CLI parsing, ledger corruption (F-009).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "left the writer deliberately unbuilt: 'no code path exists that edits the matrix'" (issue #57 history) | compat_matrix.rs:1-6 | UNVERIFIED | issue text not accessible (no git/forge access); current code state contradicts the quote by design (this module IS the writer) |
| "exactly one dispatch-line registration in cmd/mod.rs" | compat_matrix.rs:5-6 | REAL | mod.rs:179-184, single `Subcommand` entry; `rg compat_matrix tools/` → only mod.rs + this file |
| "Status transitions ... flow ONLY through a transitions outbox" | compat_matrix.rs:10-13 | FALSE (in practice) | docs/feature-matrix.toml:674-680 hand-repair; scripts/q2v_matrix_backfill.py:39 second writer (F-004) |
| "REJECTS ... `to = "verified"` — no self-verification path exists in code" | compat_matrix.rs:26-28 | REAL | L444-449 implemented; test rejects_verified_transitions; `rg '= "verified"' docs/feature-matrix.toml` → 0 status cells; windows-status census: 34 blocked / 66 implemented / 99 not_started / 46 partial / 7 unsupported = 252, zero verified |
| "verified requires Lead-applied CI evidence" (implied pathway) | compat_matrix.rs:446-447 | PAPER-ONLY | no code path anywhere applies verified: CI (compat-matrix.yml:42-44) "NEVER edits docs/feature-matrix.toml" and never invokes the writer; the terminal state is unreachable through sanctioned tooling |
| "The rewritten matrix is re-parsed and diffed ... only the intended status/notes/evidence cells may differ" | compat_matrix.rs:55-57 | SHALLOW | verify_diff skips content comparison of those three keys on edited rows (F-003) and uses the same lenient parser (F-002 proof) |
| "a citation already present ... is never re-appended" | compat_matrix.rs:47-51 | REAL | test recited_evidence_is_not_reappended; my execution of the production ledger shape confirms the design landed (aac-lc comment documents the pre-fix bug + fix) |
| "Std-only (xtask C5) ... no `serde_json` here" | compat_matrix.rs:59-62 | REAL | xtask Cargo.toml has no [dependencies] (zero deps); own parser at 923-1253 |
| "Exit codes: 0 ... | 1 rejected ... | 2 xtask error" | compat_matrix.rs:64-66 | REAL | observed 0 (apply) and 2 (F-001 probe) in my runs; rejection path (1) covered by unit tests |

## 7. Dead / duplicated / unreachable code
- The `edit_row` notes-insertion branch (L675-682) is effectively dead-for-purpose: reachable, but its output is always rejected by verify_diff (F-001) — it can never contribute to a successful write.
- `unreachable!` in the test helper `must()` (L1329, 1339) — test-only panics, acceptable.
- No other dead code found; every function is called (json module exercised via parse; `utf8_width` via string; `iso8601_now`/`civil_from_days` via apply).

## 8. Dependencies & flags
- Zero external crates (xtask Cargo.toml — no [dependencies] section). No feature gates on this module. Consumers: `cmd/mod.rs` dispatch table only. Reuses `feature_audit::unresolved_paths` (R4 path-resolution, existence-only — content of evidence is NOT checked, by design; feature-audit judges) and `toml_lite` (lenient TOML subset; rejects `"""`/literal strings loudly, which protects edit_row's single-line assumptions).

## 9. Open questions for the next agent
- Who audits `tests/compat/src/report.rs` (the outbox PRODUCER — gap rules, `from` statuses taken from the live matrix)? The writer trusts the runner's reasons verbatim; a runner bug propagates into matrix notes (the 2026-11-10 ledger shows exactly this pipeline working, but producer correctness is unaudited here).
- The "verified" terminal state has no sanctioned pathway (see §6) — is a Lead-applied CI mechanism planned in code, or is the 7-status vocabulary's terminal state permanently aspirational?
- Should `toml_lite` reject raw control chars so every xtask consumer (feature-audit, parity-report, config-validator) gains the strict-parser teeth ci.yml already has?
