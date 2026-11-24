# AUDIT: fuzz/corpus/fuzz_hls_playlist/fmp4_pl.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 269 B / c30364ee6fcb606572942eb6b2f62251575e8d16df3d329dbde60f5cf5f6f46a |
| File verdict | REAL (as data); the corpus copy itself is never read |
| Reachable from shipped entry point? | NO — fuzz corpus seed; byte-twin executed by HLS tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (corpus-seed audit)
Real fMP4 HLS media playlist: `#EXTM3U`, `#EXT-X-VERSION:7`, `#EXT-X-PLAYLIST-TYPE:VOD`, `#EXT-X-MAP:URI="init.mp4"` (the fMP4-byterange-map shape), 5 `#EXTINF` segments, `#EXT-X-ENDLIST`. It is sha256-identical to `crates/playlists/tests/fixtures/hls/fmp4/pl.m3u8`, which the stable netfuzz sweep (`crates/streaming/tests/netfuzz_sweep.rs`) loads as one of its REAL HLS seeds — the module doc explicitly states "the same files are the committed `fuzz/corpus/` seeds" (netfuzz_sweep.rs:25) but reads the playlists-crate copies, not the corpus path.

## 2. What was checked and how
- batch.py: 269 B, 18 text lines, full head verified (tags listed above).
- sha256 equality with crates/playlists/tests/fixtures/hls/fmp4/pl.m3u8: c30364ee… both (verified).
- Consumers: rg "fuzz/corpus" -> existence check + unwired campaign only; the twin is consumed by netfuzz_sweep.rs:121 (`fn hls_seeds()`) and the playlists/streaming HLS suites.
- Execution: netfuzz + HLS suites pass in baseline (audit/runs/baseline/cargo-test-workspace.txt:5641-5674).

## 3. Findings
F-001 [S3] Corpus-lane gap (shared with all fuzz/corpus seeds): the file is never read from fuzz/corpus by anything that executes; netfuzz_sweep deliberately re-reads the twin. The seed is genuine fuzz material with an honest executed twin — the gap is duplication + unwired campaign, not fakeness. Verification: ran (sha256 + rg).
