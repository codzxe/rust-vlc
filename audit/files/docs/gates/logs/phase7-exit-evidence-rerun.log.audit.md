# AUDIT: docs/gates/logs/phase7-exit-evidence-rerun.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / blocks | 23 / 11-line header + 12-line cargo test body |
| File verdict | REAL (genuine recorded run) but STALE AS EVIDENCE: both "ok" tests now FAIL per orchestrator baseline |
| Reachable from shipped entry point? | N/A (gate log). Cited by docs/gates/phase7-exit.md:48 (exit-gate row 5) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | read full log; grep citations (docs/gates/phase7-exit.md:48; docs/reviews/w24-f-gate-findings.md:34); baseline cross-check audit/runs/baseline/cargo-test-workspace.txt:2763-2781 (same 4 tests, 2 FAILED) |

## 1. One-paragraph verdict
This is a real cargo-test capture — and it is now a museum piece. The body
shows `final_frame_is_current ... ok` and
`committed_transcripts_match_fresh_runs ... ok` on 2026-11-10; the
orchestrator's current baseline shows the same two tests FAILED (with a
concrete transcript drift: the committed `open` transcript is stale vs a
fresh run). The log's own header is honest to a fault — it admits the exit
code was never recorded and that "exit 0" is an inference — but nothing in
the citation chain (docs/gates/phase7-exit.md row 5 "**PASS**") carries a
validity date, so the exit-gate table presents 2026-11-10 greens as current
truth while the suite is red today.

## 2. What actually works (verified)
- Body shape is a genuine cargo test capture: compile/finished lines, 4
  tests, `test result: ok. 3 passed; 0 failed; 1 ignored`, 11.57s.
- The suite identity matches the current tree exactly (same 4 test names as
  audit/runs/baseline/cargo-test-workspace.txt:2763-2767) — so the log is
  comparable, not fabricated.
- Header honesty verified verbatim (F-002 quote below): the retroactive
  annotator disclosed the missing exit code instead of inventing one.

## 3. Findings
F-001 [S2] — Stale green evidence for a gate that is red today
Where: docs/gates/logs/phase7-exit-evidence-rerun.log:17-22 (ok rows);
baseline audit/runs/baseline/cargo-test-workspace.txt:2764-2767
Code (this log):
```
test final_frame_is_current ... ok
test ledger_matches_the_artifacts ... ok
test regen ... ignored, rewrites the committed evidence artifacts
test committed_transcripts_match_fresh_runs ... ok
```
Baseline (current reality):
```
test committed_transcripts_match_fresh_runs ... FAILED
test ledger_matches_the_artifacts ... ok
test regen ... ignored, rewrites the committed evidence artifacts
test final_frame_is_current ... FAILED
```
What is wrong: 2 of the 3 "ok" tests now fail; the failing drift is
substantive (baseline stdout shows committed transcript missing event
seq 8 `progress "0.00"`, event seq 10, and `frames_accepted` 30 vs 94
— the committed evidence transcripts rotted). Why it matters: this log is
the cited proof of the Phase-7 exit gate row 5 ("**PASS** (3 passed / 1
ignored-by-design `regen`)", docs/gates/phase7-exit.md:48) — the doc
presents the stale run as the gate's evidence with no as-of date. What
correct looks like: exit-gate docs cite the LATEST rerun with a date, or
the gate is re-run (the drift mechanism itself —
`committed_transcripts_match_fresh_runs` — is the repo's own staleness
detector firing exactly here).
Confidence: HIGH — Verification: ran (grep baseline) + read both sides.
F-002 [S4] — The "Exit: exit 0" of the run is an inference, not a record
Where: docs/gates/logs/phase7-exit-evidence-rerun.log:7-9
Code:
```
# Exit: exit code not recorded at capture time — no exit marker in the body. The body is
#   complete ("test result: ok. 3 passed; 0 failed; 1 ignored"), i.e. a zero-failure cargo
#   test run (exit 0), but that is an inference from the body, not a recorded value.
```
What is wrong: the central exit-gate datum (the exit code) was not captured;
the annotation says so honestly. Corroborated by docs/reviews/
w24-f-gate-findings.md:34 ("no exit marker at all"). The inference is sound
(a complete `test result: ok` body ⇒ exit 0), so this is hygiene, not fraud.
Confidence: HIGH — Verification: read + citation cross-check.

## 4. Chunk coverage
Single 23-line file read in full (header L1-11, body L12-23). No chunks.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "first-vertical evidence suite ... PASS (3 passed / 1 ignored-by-design regen)" citing this log | docs/gates/phase7-exit.md:48 | FALSE as current truth | baseline: same suite, 2 FAILED (cargo-test-workspace.txt:2764-2767) |
| "the body below was not altered in any way" (retroactive header) | log:10-11 | REAL-UNPROVEN | no original to diff against; annotator is the fix-lane W24-K; UNVERIFIABLE by construction |
| Worktree/commit provenance (c339ee8 / a4e35e7, 2026-11-10) | log:2-4 | REAL-UNPROVEN | git history not available to me (never run git); self-consistent and specific |

## 6. Dead / duplicated / unreachable code
N/A (log data).

## 7. Dependencies & flags
Cargo test -p first-vertical --test evidence; artifacts under
tests/integration/first-vertical/evidence/ (transcripts/*.json, final-frame.png)
— the drifting inputs for the now-failing detector.

## 8. Open questions for the next agent
- Who owns docs/gates/phase7-exit.md? Its row-5 PASS needs a freshness
  verdict against the current baseline — my F-001 is the evidence for that
  doc audit.
- The transcript drift specifics (seq 8/9/10 event reshuffle,
  frames_accepted 30→94, scheduler_pushed 15→16) belong to the
  first-vertical tests/evidence.rs auditor: is the drift a product bug
  (behavior changed) or a committed-artifact rot (evidence not regenerated)?
  The baseline note says "regenerate with ... --ignored regen" — suggesting
  artifact rot, but the event-count change (9→10 events) could be real
  behavior change; UNRESOLVED here.
