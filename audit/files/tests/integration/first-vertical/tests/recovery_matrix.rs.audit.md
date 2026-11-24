# AUDIT: tests/integration/first-vertical/tests/recovery_matrix.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 1,431 / 31 (24 #[test], incl. 1 #[ignore]) |
| File verdict | REAL-UNPROVEN (genuine full-stack pinning tests; NO committed run evidence — see F-001) |
| Reachable from shipped entry point? | NO (integration test binary; compiled by the baseline build but never executed there) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 4 |

## 1. One-paragraph verdict
This is the strongest test file in my assignment: 19 tier-1 rows drive every
class-B fixture through the real dispatch stack (SessionHarness -> commands ->
access/probe/demux/decode/sinks/wire) and pin outcome CLASS + damage counts +
frame/position accounting + never-fabricated fraction bounds + per-row wall
caps, plus an env-gated tier-2 randomized-corpus decision procedure. The checks
are substantive (asserted via assert_row, which requires at least one check and
fails on any false). But its protective value is currently zero: the committed
baseline aborted the first-vertical package at evidence.rs failures BEFORE this
binary ran, and the evidence ledger itself says no CI runner has ever executed
the workflows.

## 2. What actually works (verified)
- Read the entire file; all 19 tier-1 rows map to fixtures whose derivations I
  byte-verified earlier in this wave (trunc4, negheight0, flip1 etc.).
- Compilation is proven by the baseline (audit/runs/baseline/cargo-test-
  workspace.txt:2722-2726 "Compiling first-vertical ... Finished `test` profile
  ... in 2.02s" — all 6 test binaries built, incl. this one).
- Nothing executed: baseline PACKAGE first-vertical runs unittests,
  baselines_consistency, class_b, evidence — then "error: test failed, to rerun
  pass `-p first-vertical --test evidence`" + exit 101 (lines 2793-2795);
  recovery_matrix/journey/wire_mode binaries are never "Running" in the log.

## 3. Findings
F-001 [S2] Where: audit/runs/baseline/cargo-test-workspace.txt:2722-2795 (the
committed evidence) vs this file.
Code (baseline:2793-2795): `error: test failed, to rerun pass `-p first-vertical --test evidence` / ### PACKAGE first-vertical exit_code=101 / ### PACKAGE first-vertical FINISHED (real failure ec=101)`
What is wrong: cargo test without --no-fail-fast aborts the package at the
evidence.rs failure, so the 19 tier-1 rows here (the "3.C2" fix for defect G2 —
"17 of 22 rows were unpinned", file header lines 5-8) never ran in the only
committed test run; combined with evidence/2.E1-first-vertical.txt's
"first-vertical.ci.3os-green :: PENDING — no CI runner has executed the
workflows", there is NO lane, recorded or claimed, in which these tests have
ever gone green. Why it matters: the matrix is cited as the engine-level
pinning of the class-B union contract; unexecuted pinning is indistinguishable
from no pinning. What correct looks like: --no-fail-fast in the gate (or fix the
evidence.rs drift) and a recorded green run. Confidence: HIGH — Verification:
read (baseline log read directly).
F-002 [S4] Where: recovery_matrix.rs:865,900.
Code: `let transcript = harness.finish("recovery-matrix").transcript_bytes(); ... let _ = transcript;`
What is wrong: the all-disabled row generates the full transcript then throws
it away — no event assertions (unlike every other row); dead work + a weaker
row than its "G1 end-to-end" billing. Confidence: HIGH — read.
F-003 [S4] Where: recovery_matrix.rs:132-213.
What is wrong: `event_records`/`parse_field` is a hand-rolled line-oriented
JSON parser keyed on the harness's pretty-renderer shape (`splitn(2, "\": ")`,
quote trimming); any transcript renderer change silently degrades to empty
fields, and a value containing `", ` edge shapes can mis-split. It is bounded
(4096 records, line 188) — so a >4096-event transcript would silently truncate
event counts that the checks then assert on. Blast radius: this test file only.
Confidence: HIGH — read; UNVERIFIED what a real transcript exceeds 4096 events
(none expected).
F-004 [S4] Where: recovery_matrix.rs:1106-1117.
What is wrong: the tier-2 loop `advance` + `thread::sleep(25ms)` busy-waits to
terminal within the wall budget — acceptable inside an #[ignore] env-gated
harness, but wall-clock paced rows make outcomes time-sensitive (declared
"by design" in the comment at 1058-1061). Confidence: HIGH — read.
F-005 [S4] Where: recovery_matrix.rs:1330.
Code: `#[ignore = "randomized-corpus tier 2: set LUMEN_CORPUS to activate"]`
What is wrong: tier-2 never runs without a gitignored corpus + env var; honest
by design (no-op prints "no-op", line 1333) but contributes to the 51 ignored
baseline tests. Confidence: HIGH — read + baseline ignored counts.

## 4. Per-function table (non-test core)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| event_records | 132-204 | REAL | S4 | bounded hand parser (F-003) |
| parse_field | 208-213 | REAL | S4 | fragile key/value split |
| drive | 216-469 | REAL | — | the row driver; wall cap measured; honest check set |
| events_position_frames | 473-481 | REAL | — | first "frames_accepted" line; 0 default |
| assert_row | 485-496 | REAL | — | requires >=1 check; fails on any false |
| parse_corpus_manifest | 935-1007 | REAL | — | indent=1 JSON line parser; entries drop cleanly |
| json_unquote | 1011-1021 | REAL | — | first quoted value |
| wireable | 1028-1037 | REAL | — | codec-axis filter (mjpeg/pcm only) |
| wall_budget_ms | 1062-1064 | REAL | — | 2x duration + 5 s |
| drive_corpus_entry | 1068-1175 | REAL | S4 | busy-wait loop; hang classified, never passed |
| verdict | 1183-1311 | REAL | — | decision procedure; never-terms + stage whitelists |
| 19 tier-1 #[test] rows | 502-902 | REAL-UNPROVEN | S2 | F-001: never executed in any recorded lane |
| corpus_dirty_outcome_classes_hold | 1329-1431 | REAL-UNPROVEN | — | #[ignore] env-gated tier 2 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 19 matrix_* rows | YES (if executed) | pins stage.code, damage counts, frames, position, wall; e.g. trunc3 pins frames 61 @1,000,000 µs and fraction<1.0 — a decoder regression moves these |
| matrix_all_disabled | YES (if executed) | pins control.unsupported + container name in message |
| corpus tier 2 | YES (if activated) | per-op allowed-stage matrix + never-terms; hang classified, "never silently passed" |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every committed class-B fixture driven through the FULL stack" | file:1-2 | REAL-UNPROVEN | code does it; no committed execution (F-001) |
| "17 of 22 rows were unpinned" (G2 fix) | file:5-8 | UNVERIFIED | historical claim; no run proves the fix |
| "W18-KK's local CI verification caught the stale blanket expectation" | file:688-694 | UNVERIFIED | no workflow run evidence in tree/baseline |
## 7. Dead/dup/unreachable: `let _ = transcript` (F-002); tier-2 sleeps.
## 8. Dependencies: first_vertical::harness (SessionHarness/run_pumps), testkit, commands crate.
## 9. Open questions: none — next agent should re-run with --no-fail-fast.
