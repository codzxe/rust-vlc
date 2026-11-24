# AUDIT: tests/fixtures/generated/valid/mkv_multitrack.mkv
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 52,208 bytes / 822ee5df87a2afd0a2239f483403e74cd827496fe92309ff922701eb17b65737 |
| Type | Matroska data |
| File verdict | REAL-UNPROVEN (bytes real; ffmpeg-authored provenance, oracle differential unverifiable here) |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; rg consumers |

## 1. Verdict
ffmpeg-authored valid Matroska with 4 tracks: h264 + aac + flac + subrip (crates/demux/tests/mkv_fixtures.rs:12: `- mkv_multitrack.mkv — h264 + aac + flac + subrip (4 tracks)`). Consumers: mkv_fixtures.rs:127 `("mkv_multitrack.mkv", 4)` (track-count assertion) and :157. `file` reports "Matroska data". Baseline mkv_fixtures suite ran ok (cargo-test-workspace.txt:2353). The packet-count/audio-payload differentials in that suite call live `ffprobe`/`ffmpeg` and SKIP (early return) when the oracle is absent (mkv_fixtures.rs:120-121, 149-150; testkit/src/lib.rs:124-148) — skip==pass: those tests count as "ok" without comparing anything on an oracle-less machine.
## 2. Findings
F-001 [S3]
Where: mkv_fixtures.rs:120-121 with crates/testkit/src/lib.rs:145-155
Code (testkit): `pub fn skip_no_oracle(what: &str) { eprintln!("oracle: ffmpeg/ffprobe not on PATH — skipping {what} ...") }`
What is wrong: oracle-gated differentials are vacuous-green on machines without ffmpeg — documented posture (doc comment admits "neither the local gate nor CI may HARD-require ffmpeg"), so honesty is preserved but coverage silently collapses to zero differential. Confidence: HIGH — Verification: read (both files).
## 3. What actually works (verified)
sha256/size recorded; track-count consumer asserted in a suite that ran in baseline (cargo-test-workspace.txt:2353).
