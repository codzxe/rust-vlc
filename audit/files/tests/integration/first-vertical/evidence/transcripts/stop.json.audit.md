# AUDIT: tests/integration/first-vertical/evidence/transcripts/stop.json
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Lines / functions | 214 / transcript artifact (schema transcript/1) |
| File verdict | REAL (recorded evidence — STALE vs code per committed baseline) |
| Reachable? | NO (evidence artifact); consumer: first-vertical evidence.rs currency tests |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:0 |
| Commands run | Read; python3 (final-frame digest cross-check); rg baseline |

## 1. One-paragraph verdict
Recorded session transcript for the `stop` scenario over
generated/valid/avi_mjpeg_pcm.avi: open → stop (accepted) → get-info →
close → get-current-time, with the honest `session.not-ready` typed error
after close. 15 wire events walk Idle→Opening→Buffering→Playing→Stopped→Idle
with one seq gap (7 — the engine-private FirstFrame, documented contract in
harness.rs:526-539). Labels honestly record the producing machine
(local/linux/mock). The artifact's currency is broken: the baseline run
(2026-11-23) FAILED `committed_transcripts_match_fresh_runs` — the evidence
was last regenerated 2026-11-15 (ledger.json started_at) and the code has
drifted since (open: frames_accepted 30→94). Whether this specific transcript
still matches a fresh run is UNVERIFIED (the suite failed at `open` first;
no cargo here).

## 2. What actually works (verified)
— Read: 5 commands, 15 events, observables final_state Idle / position 0 /
session null — a coherent, honestly-terminated stop walk (matches the
MP4-smoke litmus shape from AGENT-SPEC §13: Idle→…→Stopped→Idle).
— Python3: pause.json's final_png digest == evidence/final-frame.png on disk
(698af4ee…) — artifact-set internal consistency holds.
— ledger.json (evidence/1) records the regen run (exit 0, run_id
2e1-local-f4ef9e71e) — provenance recorded.

## 3. Findings
F-001 [S2]
Where: whole committed evidence set (this file + siblings), surfaced by
tests/integration/first-vertical/tests/evidence.rs:47
Code (audit/runs/baseline/cargo-test-workspace.txt:2764-2774):
`test committed_transcripts_match_fresh_runs ... FAILED`
`assertion 'left == right' failed: the committed transcript for 'open' drifted from a fresh run — regenerate with: cargo test -p first-vertical --test evidence -- --ignored regen`
What is wrong: evidence regenerated 2026-11-15, baseline 2026-11-23 shows
drift (frames_accepted 30 vs 94 fresh) — the committed transcripts (this file
included) no longer prove current engine behavior, and the regen was not run
before the baseline.
Why it matters: these transcripts are the first-vertical's evidence
artifacts; stale accounting (frames, scheduler) understates current decode.
What correct looks like: run the ignored regen and re-commit.
Confidence: HIGH for the set-level staleness (baseline FAILED); UNVERIFIED
for this exact file's diff (suite failed before reaching it).
