# AUDIT: crates/demux/src/asf/fixtures/wma_wmav2_mono8k.wma (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Size / type | 6942 bytes; `file` → "Microsoft ASF" (exit 0) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — committed test fixture; consumed by asf/tests.rs FIXTURES (line 919-932) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `od` header dump; `sha256sum` (2f1abf090401e5d4 — matches pin); live `ffprobe -show_entries stream=codec_name,sample_rate,channels` → `wmav2,8000,1` (exit 0); baseline cargo-test-workspace.txt:1780, 2166 |

## 1. Verdict (short-form per binary-asset rule)
A genuine ASF/WMA fixture and the anchor of the ASF differential: bytes
start with the ASF Header GUID `30 26 b2 75 8e 66 cf 11 a6 d9 00 aa 00 62
ce 6c`, declared header size 0x1EC = 492 and 5 children (offsets 16-27 of
the dump) — exactly the values the FIXTURES truth table pins
(asf/tests.rs:923-924: `header_size: 492, children: 5`). sha256 first-16
`2f1abf090401e5d4` matches the §4.4 pin at tests.rs:1486 (verified by me
with system sha256sum). Live ffprobe reads it as `wmav2, 8000 Hz, 1ch` —
matching the pinned track truth (tests.rs:925: Audio "wmav2", wfx
`Some((0x0161, 1, 8_000, 256, 16, 10))`).

## 2. Usage honesty
Consumed by `fixture_family_walks_match_the_measured_truth` (sha gate at
tests.rs:1138 + full walk assertions) and
`fixture_family_cross_checks_against_live_ffprobe` — both ran and passed in
the baseline (cargo-test-workspace.txt:1780, 2166). The file is exactly what
the tests claim (ffmpeg wmav2 product, sha-pinned); size 6942 matches the
pinned FPO file_size fact (tests.rs:926: `fpo: (6_942, 2, 3_100, 3_200)`).
No doc/matrix row cites this file beyond the test suite itself. Nothing
wrong with the asset.
