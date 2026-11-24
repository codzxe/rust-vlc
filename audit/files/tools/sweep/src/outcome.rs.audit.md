# AUDIT: tools/sweep/src/outcome.rs
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / functions | 438 / 11 prod fns + 9 unit tests |
| File verdict | REAL — clean classification/tally core of the corpus sweep; all 9 tests passed in baseline |
| Reachable from shipped entry point? | NO — dev tool (`tools/sweep` workspace member). Reachable from tools/sweep/src/main.rs:60 (`use outcome::{Outcome, Tally, WorkerLine, class_of, classify, crash_note, parse_worker_line}`) and worker.rs:40 (`use crate::outcome::sanitize`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg caller sweep; baseline cite cargo-test-workspace.txt:3300-3312 |

## 1. One-paragraph verdict
A small, well-made classification module: a 7-outcome taxonomy where nothing
is hidden (`Unknown` for protocol bugs, `Panic` for signal deaths, counted
not guessed), a strict 15-field tab-protocol parser that returns None on any
malformation, honest crash-note extraction, and a deterministic per-class
tally. The 9 unit tests are real and passed in the baseline
(cargo-test-workspace.txt:3305-3312). Two S4 notes only: a marker line that
prints before a deadline kill outranks the kill in `classify`, and the
`Outcome::ALL` array duplicates the enum's declaration order by hand.

## 2. What actually works (verified)
— Baseline: cargo-test-workspace.txt:3300-3312 — `Compiling lumen-sweep` →
`running 15 tests` → all `outcome::tests::* ... ok` (9 of the 15 are this
module's) plus main.rs's `tests::*` rows.
— Mutation reasoning: `outcome_mapping_is_total_for_worker_statuses`
(L329-344) pins every status string including `panic → None`; breaks if
mapping drifts. `rejects_malformed_lines` (L376-380) proves the parser
refuses truncated/prefixed/missing-field lines. `tally_records_and_renders`
(L426-437) asserts the rendered matrix row exactly
(`"avi-video\t1\t0\t0\t0\t1\t0\t0\t2"`).
— `class_of` slice-safety hand-checked: `stem[digits + 1..]` is guarded by
`stem[digits..].starts_with('-')`, ASCII-digit count == byte count.

## 3. Findings
F-001 [S4]
Where: tools/sweep/src/outcome.rs:194-209
Code:
    if let Some(line) = marker {
        // A parsed marker with an unrecognized status field is a
        // protocol bug, not an engine outcome — count it honestly.
        return Outcome::from_worker_status(&line.status).unwrap_or(Outcome::Unknown);
    }
    if timed_out {
        return Outcome::Timeout;
    }
What is wrong: when a worker prints its marker line and THEN hangs (killed at
the deadline), `timed_out` is never consulted — a post-marker hang (e.g. stuck
teardown) is reported as the marker's outcome, not Timeout. Design choice
(marker = final statement) but the hang is invisible in the census.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: tools/sweep/src/outcome.rs:53-61
Code:
    pub const ALL: [Outcome; 7] = [
        Outcome::Ok,
        Outcome::Unsupported,
        Outcome::DirtyReject,
        Outcome::TypedError,
        Outcome::Panic,
        Outcome::Timeout,
        Outcome::Unknown,
    ];
What is wrong: manual duplication of the enum's variant order — `row[outcome
as usize]` indexes by discriminant while render/ALL iterate this array; a
future variant inserted into the enum but not ALL (or reordered) silently
mislabels tally columns with no compile error. Currently consistent (Ok=0 …
Unknown=6; derive order matches).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Outcome::as_str | 64-75 | REAL | — | stable column names |
| Outcome::from_worker_status | 78-87 | REAL | — | total map tested incl. None case |
| parse_worker_line | 127-167 | REAL | — | 14 fixed fields + detail; extra field → None (L147-149) |
| class_of | 175-186 | REAL | — | corpus-v4 `NNNN-class` naming → class (the corpus tie-in); slice-safe |
| classify | 194-210 | REAL | S4 (F-001) | marker outranks timeout |
| crash_note | 216-239 | REAL | — | panic line first, else exit fact + 200-char stderr tail |
| sanitize | 244-261 | REAL | — | tab/CR/LF → space; 400-char cap; "-" for empty |
| Tally::new/record/total/outcome_totals/render | 270-322 | REAL | S4 (F-002) | BTreeMap deterministic; columns match ALL |
| 9 tests | 328-437 | REAL | — | baseline ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| outcome_mapping_is_total_for_worker_statuses | YES | exact Option equality on 5 statuses |
| parses_a_full_ok_line | YES | field-by-field equality |
| parses_an_error_line_with_spaces_in_detail | YES | detail must survive with spaces |
| rejects_malformed_lines | YES | 3 malformed forms → None |
| class_of_handles_corpus_and_fixture_naming | YES | 4 naming shapes incl. <3-digit fallback |
| sanitize_kills_field_breaking_chars | YES | tab/newline → space; blank → "-" |
| classify_maps_worker_and_crash_paths | YES | 6 branches incl. bogus-status → Unknown |
| crash_note_finds_panic_line_or_exit_fact | YES | panic line / killed-by-signal / exit=N |
| tally_records_and_renders | YES | exact rendered row string |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`Unknown` … a tool bug — counted, never hidden" | file L5-7 | REAL | classify L203-205 maps exit-0-no-marker → Unknown; test L411 |
| "the manifest's `class` field equals `<class>` here" | file L170-171 | REAL-UNPROVEN | naming read from gen_corpus.py side (W16-A04's audit domain); class_of logic read |

## 7. Dead / duplicated / unreachable code
No dead code. ALL/enum duplication noted (F-002).

## 8. Dependencies & flags
std-only (BTreeMap, Path). No feature gates. Consumed by sweep main.rs/worker.rs
(verified callers, §header).

## 9. Open questions for the next agent
- Does sweep's supervisor ever observe marker-then-timeout in practice?
  (main.rs owner; L383/416 Sweepline constructors.)
