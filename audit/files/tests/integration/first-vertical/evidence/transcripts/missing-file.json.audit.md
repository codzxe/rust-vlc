# AUDIT: tests/integration/first-vertical/evidence/transcripts/missing-file.json
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 1,089 B / a0db6aa3be4ed370… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (negative-path command transcript; schema checked) |
| Reachable from shipped entry point? | N/A (evidence data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: `transcript/1` JSON for scenario `missing-file`: labels
(local/linux/mock, null-audio/software-render), two commands — session.open
erroring `[input.inaccessible] ... No such file or directory (os error 2)` for
`generated/valid/no-such-file.avi`, then media.get-current-time erroring
`[session.not-ready] no session open` — final_state Idle, position 0. Content
read in full (above); the error strings are the real typed-error shapes the
dispatch layer produces.

Consumer: tests/integration/first-vertical/tests/evidence.rs (the transcript
suite — `transcripts/missing-file.json` is compared against fresh runs; the
baseline cargo-test-workspace.txt shows the first-vertical tests passing, incl.
`committed_transcripts_match_fresh_runs` style checks — cite
audit/runs/baseline/cargo-test-workspace.txt "test result: ok" entries and the
2 failures listed in AGENT-SPEC §13 are the SAME first-vertical tests:
`committed_transcripts_match_fresh_runs` and `final_frame_is_current` FAILED in
the committed baseline — i.e., at the recorded baseline these transcripts did
NOT match fresh runs). Honesty: real negative-path evidence, no fabrication.
UNVERIFIED: exact baseline failing-test names live in
audit/runs/baseline/cargo-test-workspace.txt (grep for first-vertical failed
lines) — recorded as a cross-check for the evidence.rs audit below.
