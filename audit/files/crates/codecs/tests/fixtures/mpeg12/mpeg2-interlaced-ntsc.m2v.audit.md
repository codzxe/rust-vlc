# AUDIT: crates/codecs/tests/fixtures/mpeg12/mpeg2-interlaced-ntsc.m2v (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Type / size | `file` → "MPEG sequence, v2, MP@ML interlaced Y'CbCr 4:2:0 video, 29.97 fps"; 50,993 bytes |
| Cited by | crates/codecs/tests/mpeg12_corpus.rs:217 (facts lane), docs/formats/video-mpeg12.md:61 and :89 (differential evidence table) |
| Verdict | REAL — committed fixture, actively exercised |

## 1. Verdict
Genuine ffmpeg-authored MPEG-2 elementary stream, not a placeholder. Header
bytes verified: `00 00 01 b3 0c 00 6c 34` → sequence_header_code 0xB3,
192×108 (0x0C0 / 0x06C), aspect_ratio 3, frame-rate code 4 (30000/1001) —
exactly the lane values the corpus test asserts.

## 2. What actually works (verified)
— verified by `./target/debug/deps/mpeg12_corpus-e7a93b397405d06f
--nocapture` → `ROW mpeg2-interlaced-ntsc.m2v: PASS 192x108
profile=Some("Main") level=Some(8) chroma=yuv420p rate=30000/1001
interlaced=true pics=18 slices=144 mb_addr=64 seqs=2` — the fixture is on
a live, non-skipping test path (fixture_dir() = committed
tests/fixtures/mpeg12, mpeg12_corpus.rs:38-40, no corpus-env skip).

## 3. Findings
None. No doc/matrix row cites this file with claims beyond what the test
run shows (interlaced=true is asserted and produced).
