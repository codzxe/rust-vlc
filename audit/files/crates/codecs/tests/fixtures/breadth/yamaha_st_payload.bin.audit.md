# AUDIT: crates/codecs/tests/fixtures/breadth/yamaha_st_payload.bin
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | binary (35840 bytes) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; adpcm_breadth_differential.rs:65 row + :303 shape check) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file ("data" — raw payload, expected); stat (35840 = 35 × 1024); od header; direct run of the built breadth suite -> 4/4 ok |

## 1. One-paragraph verdict
A raw ADPCM payload fixture (no container — by design), whose documented geometry is exact: 35840 bytes = 35 blocks × 1024 bytes, matching the consuming suite's row "`adpcm_yamaha` stereo 22 050 | `yamaha_st_payload.bin` | 35 × 1024 | ffmpeg enc, NUT-carried" (adpcm_breadth_differential.rs:18). The first bytes (`00 77 77 77 77 75 …`) are consistent with a Yamaha stereo ADPCM block header pair. I executed the built adpcm_breadth suite directly: 4/4 pass — the row's documented decode-MD5 pin (`fcf6aecad5c50e142a74235c43624f55`) is asserted by that suite and held.

## 2. What actually works (verified)
- Geometry: 35840 == 35 × 1024 (the suite's :303 shape check `("yamaha_st_payload.bin", 35, 1024)` — passed in my run).
- `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b --nocapture` → `test result: ok. 4 passed; 0 failed` — the yamaha lane's pinned differential MD5 asserted and held.
- Baseline corroborates (adpcm_breadth suite green in cargo-test-workspace.txt).

## 3. Findings
None. Checked: size == documented block shape; header bytes plausible for adpcm_yamaha stereo; consuming suite executed green with its MD5 pin; consumers (breadth suite only).

## 4-9. (binary asset — tables N/A; consuming suite's pins hold; no dead code; no deps; no open questions)
