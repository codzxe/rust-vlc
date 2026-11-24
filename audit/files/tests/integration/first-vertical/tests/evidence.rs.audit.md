# AUDIT: tests/integration/first-vertical/tests/evidence.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 227 / 4 tests + 1 helper |
| File verdict | REAL (executed in the committed baseline; its two currency assertions FAILED there — catching real staleness) |
| Reachable from shipped entry point? | NO (integration test binary; executed in baseline) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 0 S4: 3 |

## 1. One-paragraph verdict
The evidence-currency suite is honest test code with teeth: it re-runs every
scenario, asserts green first, and compares committed artifacts byte-for-byte.
In the committed baseline it ran and 2 of its 4 tests FAILED (1 passed, 1
ignored) — proving the assertions are real, and proving the repo's evidence
tree is STALE at HEAD: committed transcripts and final-frame.png no longer
match fresh runs. The failure also aborted the package before
recovery_matrix/journey/wire_mode binaries ran (see that audit's F-001).

## 2. What actually works (verified)
- Baseline run (audit/runs/baseline/cargo-test-workspace.txt:2761-2791):
  `running 4 tests / test committed_transcripts_match_fresh_runs ... FAILED /
   test ledger_matches_the_artifacts ... ok / test regen ... ignored, rewrites
   the committed evidence artifacts / test final_frame_is_current ... FAILED /
   test result: FAILED. 1 passed; 2 failed; 1 ignored`
- ledger_matches_the_artifacts passing means the ledger digests/byte counts
  match the committed files on disk (self-consistency held).
- Failure messages verify assertion targets: :47 "the committed transcript for
  `open` drifted from a fresh run" (frames_accepted 30 vs 94, event seq drift);
  :162 "the committed final-frame.png drifted from a fresh run" (PNG bytes
  differ — real byte diff in baseline output).

## 3. Findings
F-001 [S2] Where: this file vs evidence/ artifacts at HEAD.
Code (test, lines 47-52): `assert_eq!(fresh, normalize_run_specific(&committed),
 "the committed transcript for `{id}` drifted from a fresh run — ...")`
What is wrong: at the committed baseline the evidence currency is BROKEN — the
fresh run's event stream differs (frames_accepted 30 -> 94, missing seq 7,
different buffering events) and the committed final-frame.png bytes differ. So
the 2.E1 claim "first-vertical.evidence.transcripts-and-ledger" and the module
doc "the committed artifacts under `evidence/` must match a FRESH run" are
FALSE at HEAD: the artifacts are stale and the scenario runs are
non-deterministic. Why it matters: the whole first-vertical evidence chain (the
litmus vertical) rests on these artifacts; stale evidence is fabricated
evidence by another name. What correct looks like: make the open/run-to-end
scenarios deterministic (fixed pump counts instead of wall-clock) and regen.
Confidence: HIGH — Verification: read + baseline run output.
F-002 [S2] Where: first_vertical::harness::normalize_run_specific (called at
:34/:49) as evidenced by the baseline failure payload.
Code (baseline left/right payload, labels object): `\"normalized\": \"per-run\",\n    \"normalized\": \"per-run\",` — the same key twice.
What is wrong: the normalizer renames BOTH per-run label keys
(`environment_kind` and `os`) to the SAME key `normalized`, producing a JSON
object with a duplicate key — invalid JSON (RFC 8259 names SHOULD be unique);
both sides being equally mangled hides it. Why it matters: the currency
comparison is byte-equality over content that is no longer valid JSON; any
consumer parsing the normalized transcript gets last-key-wins silently.
What correct looks like: distinct keys (e.g. normalized_1/normalized_2) or a
value redaction. Confidence: HIGH — read from baseline failure output;
implementation itself is outside this file (harness module, another agent's).
F-003 [S4] Where: evidence.rs:121-135.
Code: `assert_eq!(ledger.matches("\"verdict\": \"pass\"").count(), SCENARIO_IDS.len(), ...)` and per-id `ledger.contains(...)`.
What is wrong: substring/count matching is weaker than structural parsing — a
ledger that mentions "pass" 12 times anywhere (not per-scenario) satisfies the
count; per-id presence checks don't bind id->verdict pairs. Mild stringly
weakness in an otherwise-structural test. Confidence: HIGH — read.
F-004 [S4] Where: evidence.rs:20-23, 55, 90-91, 158-161, 193-194, 223-224 —
eight `unreachable!` on I/O/dispatch errors: panics with context strings, but
`unreachable!` for "file not found" is a misleading panic class (the state IS
reachable). Consistent with repo-wide pattern (4126 unreachable! per
audit/runs/baseline/loc-patterns.txt:28). Confidence: HIGH — read.
F-005 [S4] Where: evidence.rs:118 `assert_eq!(artifacts + 1, 13, "12
transcripts + the final frame");` — hard-coded 13 pairs with SCENARIO_IDS length
implicitly; if a 13th scenario is added the arithmetic misfires confusingly
(artifacts is SCENARIO_IDS-driven). Minor brittleness. Confidence: MEDIUM.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| green(id) | 19-30 | REAL | — | asserts every scenario check passed before use — honest gate |
| committed_transcripts_match_fresh_runs | 32-75 | REAL | — | executed; FAILED at baseline (F-001) — teeth proven |
| ledger_matches_the_artifacts | 78-147 | REAL | S4 | substring matching (F-003); passed at baseline |
| final_frame_is_current | 152-175 | REAL | — | executed; FAILED at baseline (F-001) |
| regen | 190-227 | REAL | — | #[ignore] writer; refuses to write non-green runs; rewrites 2.E1 txt |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_transcripts_match_fresh_runs | YES — it DID | baseline FAILED on real drift |
| ledger_matches_the_artifacts | YES | digest/bytes/id coverage verified passing |
| final_frame_is_current | YES — it DID | baseline FAILED on PNG byte drift |
| regen | N/A (ignored writer) | refuses non-green (asserts via ev::regen_all error path) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "committed artifacts ... must match a FRESH run" | evidence.rs:2-4 | FALSE at baseline | 2 FAILED tests (cargo-test-workspace.txt:2764/2767) |
| "evidence/transcripts-and-ledger" (2.E1 claim) | 2.E1-first-vertical.txt | SHALLOW | ledger self-consistent; transcripts stale |
| ci_3os_green PENDING | 2.E1 + ledger | REAL (honest) | no CI run recorded anywhere |
## 7. Dead code: none.
## 8. Dependencies: first_vertical::{digest, evidence, harness, scenarios}, testkit.
## 9. Open questions: who owns normalize_run_specific (harness) — flag F-002 to that agent.
