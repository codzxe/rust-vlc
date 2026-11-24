# AUDIT: tests/integration/first-vertical/evidence/transcripts/play.json
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Data fixture (committed scenario transcript, schema transcript/1) |
| Size / sha256-prefix | 4652 bytes / d9518957fac0f09a (verified locally) |
| File verdict | SHALLOW as evidence: internally detailed, but the currency gate FAILED at HEAD |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 1 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, full read, baseline citation |

## 1. Verdict
Real committed transcript for the `play` scenario (open → pause → play →
end-of-media on the AVI/MJPEG/PCM fixture): 3 commands, 21 recorded events,
full accounting block, final PNG digest. Honest mock labels
(`hardware_class: mock`, `null-audio` + `software-render`). BUT the suite
that enforces evidence currency — `committed_transcripts_match_fresh_runs`
— FAILED in the orchestrator baseline: the committed transcript for `open`
drifted from a fresh run, i.e. the committed transcript tree no longer
matches current engine behavior. The feature-matrix cites this evidence tree
(row 203: "tests/integration/first-vertical/evidence/ledger.json (2.E1,
mock-labeled)") as the avi/first-vertical evidence — stale evidence being
cited by the matrix.

## 2. What actually works (verified)
- Consumer (verbatim, tests/integration/first-vertical/tests/evidence.rs:41-53):
```
    for id in SCENARIO_IDS {
        let run = green(id);
        let fresh = normalize_run_specific(&run.transcript_bytes());
        let path = ev::transcript_path(id);
        let committed = std::fs::read_to_string(&path)
```
with the assert at :47-53 (`fresh == normalize_run_specific(&committed)` else "drifted from a fresh run").
- Transcript content spot-checks: seq 7 is absent (events jump 6→8); pause
  event at seq 12 lands between time-ticks (0.000 → 0.267); accounting says
  `frames_accepted: 120` but `video_presented: 16` with
  `scheduler_repeats: 132` — internally consistent with a mock sink
  presenting a decimated subset.
- Baseline (audit/runs/baseline/cargo-test-workspace.txt:2764-2774): test
  `committed_transcripts_match_fresh_runs ... FAILED`; panic at
  tests/integration/first-vertical/tests/evidence.rs:47:9 "the committed
  transcript for `open` drifted from a fresh run".

## 3. Findings
F-001 [S1]
Where: tests/integration/first-vertical/evidence/transcripts/ (tree incl. play.json) + docs/feature-matrix.toml row 203
What is wrong: The committed evidence tree is stale at HEAD: the fresh-run
equality test fails (baseline, first-vertical suite result "1 passed; 2
failed"). The loop fails fast on `open`, so whether play.json itself drifted
is UNVERIFIED, but the tree it belongs to demonstrably no longer matches a
fresh run. The matrix still cites this tree as the avi row's evidence.
Why it matters: evidence that no longer reproduces is not evidence; the
matrix's "implemented" claim for the first vertical leans on artifacts that
the repo's own gate rejects.
What correct looks like: regenerate the transcripts (the failure message
gives the exact command: `cargo test -p first-vertical --test evidence --
--ignored regen`) and make the currency test blocking in the release lane.
Confidence: HIGH for the tree (baseline log); UNVERIFIED for play.json
specifically (fail-fast ordering).
F-002 [S3]
Where: play.json events, seq 6→8
What is wrong: Event seq 7 is missing with no recorded reason (a swallowed
event id). The transcript records no `dropped` marker for it.
Confidence: LOW — may be a legitimate harness gap (e.g. an event that
failed a filter); flag for the scenarios.rs auditor to trace seq allocation.
