# AUDIT: tests/fixtures/hls/ts_live/t.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Text fixture (ffmpeg-authored HLS segment list, second encoding lane) |
| Size / sha256-prefix | 186 bytes / f6641033b77d6d39 (verified locally) |
| File verdict | REAL — but PROVENANCE-ONLY: never read by any test |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, full text dump, rg all consumers |

## 1. Verdict
Real ffmpeg output (the 880Hz/22.05kHz lane's segment list). Its EXTINF
durations (2.507767 for t00/t01) ARE reused verbatim in the hand-authored
live windows of crates/streaming/tests/hls_live_e2e.rs — but the FILE
itself is never opened by any test: the live suite serves inline playlist
strings and only the .ts segments from disk. rg over crates/tests/apps/docs/
tools/scripts/fuzz/ui finds the filename only in comments
(hls_live_e2e.rs:20,23). Also skipped by the compat walker (`.m3u8` is in
exclude_suffixes, compat-matrix.json). So this file is generation
evidence, dead as input.

## 2. What actually works (verified)
- Verbatim (hls_live_e2e.rs:23-27):
```
//! The committed `s.m3u8`/`t.m3u8` are ffmpeg's own output (the
//! generation evidence — a finite encode always ends with
//! `#EXT-X-ENDLIST`, which is why the live windows below are
//! hand-authored); their measured EXTINF durations are reused
//! verbatim.
```
- Duration reuse verified: t.m3u8 `#EXTINF:2.507767,\nt00.ts` ==
  hls_live_e2e.rs:90 `#EXTINF:2.507767,\nt00.ts\n` (window W2).

## 3. Findings
F-001 [S3]
Where: tests/fixtures/hls/ts_live/t.m3u8 (file) — no reading consumer.
What is wrong: Committed fixture whose bytes are never consumed by any
executable path; its only role is documented provenance (its numbers were
hand-copied into the test source). Nothing detects silent corruption of
this file — a drifted t.m3u8 would still pass every gate.
Why it matters: a "fixture" that cannot fail is not evidence; the
provenance claim ("durations reused verbatim") is unenforced and can rot.
What correct looks like: either parse t.m3u8/s.m3u8 in the test to derive
the windows' durations, or move them to docs as provenance and drop them
from tests/fixtures.
Confidence: HIGH — Verification: rg over all source dirs (only comment
hits), compat exclude_suffixes read.
