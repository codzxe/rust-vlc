# AUDIT: tests/integration/first-vertical/evidence/transcripts/pause.json
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Lines / functions | 177 / transcript artifact (schema transcript/1) |
| File verdict | REAL (recorded evidence — STALE vs code per committed baseline) |
| Reachable? | NO (evidence artifact); consumer: first-vertical evidence.rs currency tests |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:0 |
| Commands run | Read; python3 digest cross-check vs evidence/final-frame.png |

## 1. One-paragraph verdict
Recorded `pause` scenario: open → pause; events walk to Playing then
Paused at seq 15 (pause accepted at request 2 while gen 0 — the accepted
response precedes the state change, an honest async ordering). Carries the
full accounting block (frames_accepted 120, video_presented 16,
scheduler_pushed 47 / presented 32 / repeats 48, staging 16 frames ×
320×240×4 B = 4,915,200 exactly — arithmetic is self-consistent) and a
final_png record whose digest matches the committed evidence/final-frame.png
byte-for-byte (python3: 307,541 B, sha256 698af4ee…). Same set-level
staleness as stop.json: baseline (2026-11-23) FAILED the currency test; last
regen 2026-11-15; and `final_frame_is_current` FAILED too — so the PNG whose
digest this transcript pins is itself flagged stale by the committed run.

## 2. What actually works (verified)
— Read: 2 commands, 15 events (seq gap 7 = engine-private FirstFrame per
harness.rs:526-539), observables Paused / position 1,033,333 µs / session 1.
— python3: final_png.sha256 == sha256(evidence/final-frame.png); staging
bytes = 16 × 320×240×4 exactly.
— Baseline: audit/runs/baseline/cargo-test-workspace.txt:2763-2767 —
`ledger_matches_the_artifacts ... ok` (artifact-set consistency) while the
currency tests FAILED.

## 3. Findings
F-001 [S2]
Where: this file + final-frame.png, surfaced by evidence.rs:162
Code (baseline :2767, :2781): `test final_frame_is_current ... FAILED`
What is wrong: the pinned final frame (and by the same drift, this
transcript's accounting) is stale vs the 2026-11-23 code; recorded
frames_accepted 120 belongs to the 2026-11-15 tree.
Why it matters: the "final frame is current" guard exists exactly to keep
this evidence honest; it is red in the committed baseline.
What correct looks like: regen + re-commit.
Confidence: HIGH (baseline FAILED); UNVERIFIED for the exact fresh-vs-pinned
diff of this scenario.
