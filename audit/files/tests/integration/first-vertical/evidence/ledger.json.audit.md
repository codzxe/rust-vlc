# AUDIT: tests/integration/first-vertical/evidence/ledger.json
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 298 lines / data (12 scenario rows, 13 artifact pins, 4 hardware labels) |
| File verdict | REAL (structurally honest ledger) with a STALE-EVIDENCE defect (see F-001) |
| Reachable from shipped entry point? | NO (test-evidence artifact); consumed by tests/integration/first-vertical/tests/evidence.rs (regen + drift guards) and tests/class_b.rs scenario wiring; cited by evidence/2.E1-first-vertical.txt |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | python3 sha256 verify of all 13 pinned artifacts; rg consumers; baseline citation |

## 1. One-paragraph verdict
A well-formed evidence ledger (schema evidence/1, run_id, commit sha + honest "artifact commit lands one commit later" note, environment labels, per-scenario verdict/checks/claims, byte+sha pins for all 13 artifacts, and an unusually honest hardware_labels section that explicitly labels audio/video/clock as mock with named real_path stubs). I verified all 13 pinned artifacts byte-for-byte (sha256 + size match the on-disk files). The defect: the committed evidence has DRIFTED from fresh runs — the repo's own drift guards (evidence.rs:37 `committed_transcripts_match_fresh_runs`, :153 `final_frame_is_current`) FAIL in the committed baseline, so this ledger's transcripts no longer describe current engine behavior and the full gate is red on the test section.

## 2. What actually works (verified)
— python3: `artifacts: 13, mismatches: NONE — all 13 pinned artifacts byte-verified` (each sha256 and byte size in ledger.json:209-274 matches the file on disk, including evidence/transcripts/corrupt-truncated-midplay.json = b437a630afa6255b…, my independently computed hash).
— Scenario table: 12 scenarios, verdicts all "pass", 92 checks total (python3 sum).

## 3. Findings
F-001 [S2]
Where: tests/integration/first-vertical/evidence/ledger.json (whole artifact set) — proven by audit/runs/baseline/cargo-test-workspace.txt:2764,2767
Code (baseline output, verbatim):
```
test committed_transcripts_match_fresh_runs ... FAILED
test final_frame_is_current ... FAILED
```
```
assertion `left == right` failed: the committed transcript for `open` drifted from a fresh run — regenerate with: cargo test -p first-vertical --test evidence -- --ignored regen
```
```
assertion `left == right` failed: the committed final-frame.png drifted from a fresh run
```
What is wrong: the ledger pins transcripts and final-frame.png regenerated at commit f4ef9e71e (ledger.json:5) — the current tree no longer reproduces them byte-for-byte (the drift guards in tests/integration/first-vertical/tests/evidence.rs:47 and :162 fail; full_gate.txt:33 shows `test: FAILED` for this section). Why it matters: this file IS the committed evidence used to support first-vertical claims; evidence that does not match the code it claims to evidence is exactly the stale-evidence pattern the drift guard exists to catch — and it has caught it. Any matrix row citing "2.E1 first-vertical evidence" currently cites evidence of a PAST tree. What correct looks like: run `cargo test -p first-vertical --test evidence -- --ignored regen` (as the panic instructs) and re-commit the ledger + transcripts. Confidence: HIGH — Verification: committed baseline run (cargo not installed in this sandbox; reason: no-cargo). Drift CONTENT unverified — the baseline diff truncates before the first differing byte; the visible prefixes are identical, so the drift may be small (e.g. normalization/ordering), but byte-drift is proven.

## 4. Per-function table
N/A (data file). Structure verified: schema/run_id/commit/started_at/environment/commands/labels/scenario_table(12)/artifacts(13)/hardware_labels(4).

## 5. Test honesty
The guard tests are honest by construction: they re-run the scenarios and compare bytes; they currently fail (proving they can fail — no tautology). The ledger itself makes falsifiable claims (per-scenario check counts, artifact hashes — I verified the hashes; the scenario claims are exercised by tests/class_b.rs / recovery_matrix.rs which pass in baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 12 scenarios all pass | ledger.json:31-207 | REAL (behavior) / STALE (bytes) | scenario tests pass in baseline; transcript equality FAILS (F-001) |
| hardware_class=mock with named real_path stubs | ledger.json:276-295 | REAL | honest self-labeling; matches AGENT-SPEC §13 environment facts (no audio/GPU in sandbox) |
| ci_3os_green: PENDING, workflows exist but no runner executed | ledger.json:292-295 | REAL | honest disclosure; .github/workflows-disabled/ naming consistent |

## 7. Dead / duplicated / unreachable code
None (data).

## 8. Dependencies & flags
None (JSON). Consumers: tests/evidence.rs (drift guards + regen), tests/class_b.rs + src/scenarios.rs (corrupt-truncated-midplay scenario), evidence/2.E1-first-vertical.txt (human-stable form).

## 9. Open questions for the next agent
- What exactly drifted? Run the regen and diff the old vs new transcript to classify (behavior change vs normalization). If behavior changed, the scenario_table claims need re-review, not just re-pinning.
