# AUDIT: crates/codecs/tests/fixtures/mpeg12/mpeg2-main-ipb.m2v (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | MPEG-2 sequence (MP@ML progressive 4:2:0, 320x240 25fps per `file`) / 146996 bytes / 9d9eb5cc7108936d175b323b6f4c21ace50261a598ee785445eea1e4278470d9 |
| File verdict | REAL (fixture exercised by a passing live cross-check test) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> MPEG sequence v2 MP@ML 320x240 25fps; `sha256sum`; `od` first bytes -> `00 00 01 b3` sequence_header + `00 00 01 b5` extension + `00 00 01 b8` GOP; `ffprobe -count_frames` -> 25 frames; `./target/debug/deps/mpeg12_corpus-e7a93b397405d06f fixture_raw_es_facts_match_ffprobe --nocapture` -> 1 passed, 0.67s |

## 1. Verdict
Real, well-formed MPEG-2 Main@Main elementary stream used as genuine differential evidence. Header bytes verify as a proper Annex-A start-code stream (`00 00 01 b3 14 00 f0 23` = 320x240, 4:2:0).

## 2. Test usage (verified by execution)
`crates/codecs/tests/mpeg12_corpus.rs` `fixture_raw_es_facts_match_ffprobe` (lane table at mpeg12_corpus.rs:193) parses this file with the in-house facts parser and cross-checks against live ffprobe. I ran it: **PASS** — output row: `mpeg2-main-ipb.m2v: PASS 320x240 profile=Main level=8 chroma=yuv420p rate=25/1 interlaced=false pics=25 slices=375 mb_addr=64 seqs=3 changes=false`. ffprobe independently reports 25 frames, Main profile, 25/1 — the fixture's facts claims are live-verified, not baked strings.

## 3. Matrix evidence
`docs/formats/video-mpeg12.md:56-60` cites it in the 10/10 differential table ("mpeg2-main-ipb.m2v (Main@8, I+P+B census)") — consistent with the executed row above (profile=Main, level=8, 25 pics). The doc's decode stance ("decode refuses with ErrorKind::Unsupported at mpeg12.decode.blocked") matches the fixture's role: facts/demux evidence only, no decode claim — no inflation detected for this asset.
