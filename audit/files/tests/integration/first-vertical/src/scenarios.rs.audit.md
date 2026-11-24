# AUDIT: tests/integration/first-vertical/src/scenarios.rs
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / functions | 1549 / 17 fns (12 scenarios + 5 helpers) |
| File verdict | REAL — full-stack scenario driver, executed green at HEAD per baseline |
| Reachable from shipped entry point? | NO — test-support lib; consumed by `cargo test -p first-vertical` (evidence.rs, recovery_matrix via fixtures) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | full read (1549 lines, 3 passes); rg consumers; baseline citation (no cargo in sandbox) |

## 1. One-paragraph verdict
This is a real, unusually honest scenario driver: every scenario dispatches
the actual command layer (`session.open`, `playback.pause`, ...) through a
`SessionHarness` over the real AVI/MJPEG/PCM fixture and class-B derivatives,
and every assertion is a machine-evaluated `Check` (broken fixture ⇒ failed
check, never a panic). The orchestrator baseline proves the scenarios
themselves ran green at HEAD (`committed_transcripts_match_fresh_runs`
asserts `green(id)` BEFORE the byte-compare, and the recorded panic is the
drift assert at evidence.rs:47 — so all 12 scenarios' checks passed; only
the committed transcript bytes are stale). Two honesty defects: one trail
check's name claims a 3-edge sequence its body does not verify, and
`state_of` contains a panic path that contradicts the module's own
"the lib never panics" contract.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/cargo-test-workspace.txt:2764-2774 — `committed_transcripts_match_fresh_runs ... FAILED`, panic at tests/evidence.rs:47 (the drift assert), i.e. `green(id)` loops (scenarios + all their checks) passed for every id first.
- Seek scenario pins exact values (would fail on drift):
```
    checks.push(Check::int(
        "the 50% seek lands at exactly half the duration",
        landing,
        DURATION_US / 2,
    ));
```
(scenarios.rs:489-493) plus session-survival and accounting pins (154/46/16/1, lines 528-551).
- Wire-session determinism proof (scenarios.rs:1210-1216):
```
                match wire_session_run() {
                    Ok((stdout2, _stderr2, code2)) => {
                        checks.push(Check::is(
                            "two identical wire runs are byte-identical",
                            stdout2 == stdout && code2 == code,
```
- Class-B containment semantics pinned (exactly one coalesced non-fatal `playback.error`, clean `Playing→Ended`, lines 990-1006).
- Honest error paths: `broken_run` (77-97) emits a failed check + minimal transcript, no panic.
- Consumers: tests/integration/first-vertical/tests/evidence.rs:11 (`use first_vertical::scenarios::{self, SCENARIO_IDS}`), src/evidence.rs:298-299 (regen driver).

## 3. Findings
F-001 [S3] — check name promises a sequence; body verifies a disjunction
Where: tests/integration/first-vertical/src/scenarios.rs:272-282
Code:
```
    checks.push(Check::is(
        "the trail crosses Idle→Opening→Buffering→Playing",
        trail.iter().any(|(from, to)| {
            matches!(
                (from, to),
                (PlaybackState::Idle, PlaybackState::Opening)
                    | (PlaybackState::Opening, PlaybackState::Buffering)
                    | (PlaybackState::Buffering, PlaybackState::Playing)
            )
        }) && trail.len() >= 3,
```
What is wrong: `any()` over an OR-pattern passes if ANY ONE of the three edges occurred (plus trail length ≥ 3). A state machine that skipped `Opening→Buffering` (e.g. Idle→Opening→Playing→...) still passes as long as ≥3 transitions exist. The claim "crosses Idle→Opening→Buffering→Playing" (all three edges, in order) is not what is tested. Contrast the `play` scenario (374-382), which correctly conjoins two separate `.any()` calls with `&&`.
Why it matters: this is the headline lifecycle claim of the `open` scenario recorded in the ledger; a state-machine regression that reorders/drops the middle edge would pass unnoticed here.
What correct looks like: three ordered `.any()` conjuncts (like `play`) or an actual subsequence match on the trail.
Confidence: HIGH — Verification: read.
F-002 [S3] — panic path contradicts the module's own no-panic contract
Where: scenarios.rs:1514-1525 (`state_of`), vs module doc scenarios.rs:6-7
Code:
```
        other => unreachable!("unknown state name {other:?}"),
```
vs doc: "the lib never panics; the test files turn checks into test verdicts".
What is wrong: a future `PlaybackState` variant (or any transcript state-name drift) turns every scenario into a panic instead of a failed check. Same class at tests/evidence.rs:16-17 (`unreachable!("scenario dispatch is total...")`) — that one is genuinely total over `SCENARIO_IDS`; this one depends on engine evolution.
What correct looks like: return an `Option`/record a failed `Check` for unknown names.
Confidence: HIGH — Verification: read.
F-003 [S4] — incomplete JSON string escaper
Where: scenarios.rs:1363-1375 (`escape_json_string`)
What is wrong: escapes `\ " \n \r` only; a path containing a raw tab, control char, or U+2028 would emit invalid JSON into the wire script. Workspace-derived fixture paths make this latent-only.
Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parent_source | 68-72 | REAL | — | fixture_path-checked |
| broken_run | 77-97 | REAL | — | honest failure shape |
| labels_json | 100-110 | REAL | — | |
| scenario_open | 125-316 | REAL | S3 | F-001 weak trail check; otherwise dense factual pins (duration/seekable/tracks/codecs/dims/trail/devices) |
| scenario_play | 323-385 | REAL | — | resume-advances proof, both-limb accounting growth |
| scenario_pause | 392-448 | REAL | — | frozen-at-landing pin (256 ticks), zero drift |
| scenario_seek | 456-561 | REAL | — | exact 50% landing, session survival, accounting pins |
| scenario_stop | 568-648 | REAL | — | Stopped/Idle/not-ready refusal pins |
| scenario_run_to_end | 655-792 | REAL | — | 60+60 accounting closure + PNG structural checks |
| push_typed_open_refusal | 801-828 | REAL | — | stage.code identity + message substring |
| scenario_missing_file | 833-887 | REAL | — | input.inaccessible, no session, not-ready |
| scenario_corrupt_truncated_open | 892-930 | REAL | — | demux.corrupt refusal |
| scenario_corrupt_truncated_midplay | 944-1020 | REAL | — | ED-FIX-3 containment: 1 coalesced error, clean Ended |
| scenario_corrupt_garbage_appended | 1025-1027 | REAL | — | delegating |
| scenario_corrupt_noidx | 1033-1035 | REAL | — | delegating |
| class_b_plays_to_end | 1040-1100 | REAL | — | exact-duration + full accounting |
| scenario_wire_session / wire_session_run / lumen_binary_path / escape_json_string / readers | 1130-1526 | REAL | S4 | real subprocess wire proof + double-run determinism; F-003 escaper |

## 5. Test honesty
| Scenario | Would it fail if code were broken? | Reason |
|---|---|---|
| open | YES (except middle-edge reorder) | F-001 is the one weak spot; all other pins are exact |
| play/pause/seek/stop | YES | exact position/int/accounting pins (e.g. seek: 1_000_000µs, 154 frames) |
| run-to-end | YES | 60+60 closure, staging-bytes = frames×320×240×4 |
| missing-file + 3 corrupt | YES | typed stage.code + message substrings, exact event counts |
| wire-session | YES | exit 0, 7 responses, requestId echo, seq monotonic, byte-identical rerun |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the lib never panics" | scenarios.rs:6-7 | FALSE-ish | `unreachable!` at :1524 (F-002) |
| Phase-2 subset of doc 10 §6.4 IDs, later-wave rows honestly absent | scenarios.rs:10-21 | REAL | SCENARIO_IDS:52-65 matches the listed 12; no repeated-seek/track-switch scenarios present |

## 7. Dead / duplicated / unreachable code
None observed; all 17 functions are called (by_id dispatches all 12 scenarios; helpers used by them).

## 8. Dependencies & flags
Uses commands crate types, lumen_cli::host::HeadlessAccounting (run_accounting, 1440-1488), std::process for the real wire subprocess. No feature gates.

## 9. Open questions for the next agent
- Trace harness.rs `settle/advance` → lumen_cli host pacing: confirm no wall-clock dependence leaks into transcripts (fresh-run drift in the baseline suggests SOMETHING became run-dependent — which normalization field drifted is worth pinning down; the baseline panic shows normalized committed vs fresh mismatch on `open`).
- tests/recovery_matrix.rs (same crate) duplicates fixture constants (TRUNCATED_*) — check for drift between the two files' fixture lists.
