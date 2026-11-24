# AUDIT: tests/integration/first-vertical/tests/baselines_consistency.rs
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / functions | 745 / 30 (1 hand-rolled JSON reader + 4 row groups + 2 tests) |
| File verdict | REAL — a genuinely two-sided drift contract; one S3 doc-vs-code gap in row group 3 |
| Reachable from shipped entry point? | NO — integration test; ran in baseline (cargo-test-workspace.txt:2746 `vlc_baseline_rows_stay_consistent_with_repo_reality ... ok`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | ls docs/baselines/raw/linux (dumps exist); ls tests/fixtures/generated/valid (33 files, 25 non-avi/mp4); baseline cite |

## 1. One-paragraph verdict
This is the cure for audit finding M15 ("VLC baseline data never consumed by
any test") and it is an honest cure: the test embeds the consolidated record
AND the verbatim raw dumps at compile time (include_str! — editing either
re-runs it), then cross-checks four row groups where each side is a different
artifact: version-string agreement across 4 JSON locations + the raw dump's
first line, census counts vs the dump's own line count + exit markers, oracle
capability rows, and a file-existence ledger for every recorded evidence path.
The hand-rolled JSON reader is competent (surrogate pairs, control-byte and
float rejection, depth cap 64) and its self-check test feeds it 7 malformed
documents — a parser that never sees bad input would be an echo chamber (its
own words, L702). The one real defect: row group 3's doc claims it pins
"every container family present in the committed valid-fixture set" but the
code maps ONLY `.avi` and `.mp4` extensions — 25 of 33 valid fixtures (mkv,
wav, aiff, ogg, webm, mov, 3gp families, all consumed by demux tests) are
invisible to the capability row.

## 2. What actually works (verified)
— Baseline run: audit/runs/baseline/cargo-test-workspace.txt:2744-2746 —
`Running tests/baselines_consistency.rs` → `test
vlc_baseline_rows_stay_consistent_with_repo_reality ... ok` (and the reader
self-check on the following lines; the 2 failed tests of the baseline are
elsewhere, in first-vertical transcripts/frame tests per AGENT-SPEC §13).
— Evidence files exist: docs/baselines/vlc-3.0.23.json,
raw/linux/04-vlc-version.txt, 05-vlc-list.txt (ls verified).
— Mutation reasoning: single-sided edits (JSON version string OR dump first
line) fail L506-510's assert_eq; deleting any record_paths file fails L663;
a census-row edit that changes line count fails L558-562. The rows genuinely
fail on drift.

## 3. Findings
F-001 [S3]
Where: tests/integration/first-vertical/tests/baselines_consistency.rs:583-592 vs 600-610
Code:
/// Row group 3 — oracle capability for this repo's media: the pinned
/// census must carry the modules that let VLC serve as the
/// differential oracle for (a) every container family present in the
/// committed valid-fixture set, and (b) the D-018 first vertical's
What is wrong: the code only collects families for two extensions:
`for (extension, family) in [(".avi", "avi"), (".mp4", "mp4")] {` (L605).
The committed valid set holds 33 files of which 25 are non-avi/mp4 (mkv x3,
wav x5, aiff x4, ogg x2, webm, mov, 3gp, aifc x2, .structure.json, .oracle
sidecars — ls-verified). The doc's "(a) every container family present in
the committed valid-fixture set" is FALSE: a VLC re-pin that drops the
matroska/wav/ogg demuxer modules keeps this row green while the repo's
committed fixtures in those families lose their differential oracle. Why it
matters: this is the test whose whole point is that the pinned oracle can
serve the repo's media; 76% of the fixture families are uncovered. What
correct looks like: derive the family set from every fixture extension that
maps to a demux test, or fix the doc to "(a) the avi and mp4 families".
Confidence: HIGH — Verification: read + ls.

F-002 [S4]
Where: tests/integration/first-vertical/tests/baselines_consistency.rs:611-613
Code:
    assert!(
        families.len() >= 2,
        "precondition: the committed valid-fixture set covers avi + mp4 (found {families:?})"
    );
What is wrong: with a 2-entry extension map, `families.len() >= 2` can only
mean {avi, mp4} — the assertion text says "covers avi + mp4" which it does
prove, but it reads as if the fixture set's breadth were being verified; the
real breadth (25 non-avi/mp4 files) is never surfaced.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: tests/integration/first-vertical/tests/baselines_consistency.rs:452-457
Code:
fn census_has(name: &str, role: &str) -> Result<bool, String> {
    let rows = census_rows()?;
    Ok(rows
        .iter()
        .any(|(module, description)| module == name && description.contains(role)))
What is wrong: census_rows() re-parses the whole 500-row dump on every call
(6 calls in row groups 2+3); a `description.contains(role)` substring match
also counts e.g. "avcodec" matching a decoder row whose description merely
mentions the word. Harmless at this scale; noting the duplication.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Json::{field,as_str,as_int,as_obj} | 74-106 | REAL | — | |
| parse_json | 124-135 | REAL | — | rejects trailing data |
| Reader::{peek,skip_ws,expect,literal} | 138-168 | REAL | — | |
| Reader::value | 170-194 | REAL | — | depth-capped |
| Reader::{object,array} | 196-253 | REAL | — | rejects trailing commas (tested L734-735) |
| Reader::string | 257-296 | REAL | — | escapes; control bytes rejected; UTF-8 validated |
| Reader::unicode_escape | 300-326 | REAL | — | surrogate pairs; lone surrogates rejected |
| Reader::hex4 / number | 328-373 | REAL | — | leading zero + float + i64-overflow rejected |
| get/string_at/int_at | 381-403 | REAL | — | typed path accessors |
| observed_version_line / dump_exit_code | 409-427 | REAL | — | dump-side readers |
| census_rows / census_has | 432-457 | REAL | S4 (F-003) | re-parse per call |
| row_version_identity | 468-531 | REAL | — | 4-way agreement + structural shape + exits |
| row_module_census | 536-581 | REAL | — | count vs dump lines + exit parity |
| row_oracle_capability | 593-647 | SHALLOW | S3 (F-001) | only avi/mp4 families checked |
| row_evidence_ledger | 652-685 | REAL | — | all record_paths + dumps exist |
| test vlc_baseline_rows… | 691-698 | REAL | — | ran ok in baseline |
| test json_reader_parses… | 707-745 | REAL | — | 7 malformed rejections; baseline ok |

## 5. Test honesty
| Test | Would it fail if code/data were broken? | Reason |
|---|---|---|
| vlc_baseline_rows_stay_consistent_with_repo_reality | YES | every assert is a two-sided equality/existence; single-sided drift fails it |
| json_reader_parses_shapes_and_rejects_malformed | YES | 7 malformed inputs each asserted to error; shapes asserted exactly |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "(a) every container family present in the committed valid-fixture set" | file L585-586 | FALSE (only .avi/.mp4 mapped, L605) | ls: 25/33 non-avi/mp4 |
| "Nothing here re-runs VLC … the rows pin the identity of the oracle" | file L15-17 | REAL | read: no process spawn in file |
| "Deliberately NOT asserted: the literal version digits" | file L33-34 | REAL | read: version string compared to composed parts, never to "3.0.23" literal |

## 7. Dead / duplicated / unreachable code
None dead. Both tests ran in the baseline.

## 8. Dependencies & flags
testkit::Must + testkit::workspace_root; std-only JSON reader (C5 rule,
documented L58-59). No feature gates. include_str! path arithmetic
(4× ../) correct (files exist).

## 9. Open questions for the next agent
- Should the capability row also pin the `raw/linux/17` avi_mjpeg avcodec
  selection it cites (L587-588) instead of only asserting module presence?
