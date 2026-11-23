# AUDIT: crates/codecs/tests/fixtures/mpeg12/mpeg1-ipb.m1v (binary asset, parts 1-8 → one audit)
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Type / size / sha256 | MPEG sequence v1 (MPEG-1 elementary stream, progressive 4:2:0, 320x240, 4:3, 25 fps per `file`) / 144076 bytes / c0e56642db5eed08e5a4a1a277ff44d8a41b0545f07eeeb87dd1718bcc5f0a5c |
| File verdict | REAL (fixture, genuinely consumed) |
| Reachable from shipped entry point? | N/A (test fixture; not shipped in binaries) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. Blunt verdict
A genuine 140 KiB MPEG-1 IPB elementary-stream fixture, referenced by exactly one test file and two doc evidence tables; the test that consumes it runs green live.

## 2. Evidence
- `file` → "MPEG sequence, v1, progressive Y'CbCr 4:2:0 video, 320x240, 4:3, 25 fps" (matches the test lane tuple below).
- Referenced by: `crates/codecs/tests/mpeg12_corpus.rs:229` — lane tuple `("mpeg1-ipb.m1v", "mpeg1video", 320, 240, "unknown", None, "yuv420p", "25/1", false)`; the loop asserts descriptor, dimensions, chroma, rate, interlaced, `picture_count >= 10`, `slice_count >= 10`, `bad_headers == 0`, and for `ipb` files `picture_type_mask == 0b0111` (I+P+B census) + motion vectors present. These are real assertions against fixture bytes — the fixture is load-bearing.
- Doc/matrix evidence: `docs/formats/video-mpeg12.md:62` (differential table, "10/10 PASS ... mpeg1-ipb.m1v" vs live ffprobe 7.1.5) and `:89` (fixture inventory, "committed, ffmpeg 7.1.5-authored"); `docs/worklog/archive-2026-10.md:9619` (differential row PASS).
- Ran: `timeout 100 ./target/debug/deps/mpeg12_corpus-e7a93b397405d06f --test-threads=2` → `3 passed; 0 failed` (0.77s) — the fixture was read and asserted during this run.

## 3. Notes
- The mpeg12 lane is parser-only (no macroblock decode) — the fixture proves sequence/slice-layer facts, not decode; that gap is documented by the module itself (video-mpeg12.md "Gaps: No macroblock-layer decode").
- No license/provenance concern recorded: docs state ffmpeg 7.1.5-authored test fixture.
