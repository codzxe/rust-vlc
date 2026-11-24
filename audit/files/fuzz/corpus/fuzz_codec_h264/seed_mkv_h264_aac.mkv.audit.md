# AUDIT: fuzz/corpus/fuzz_codec_h264/seed_mkv_h264_aac.mkv
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 26,105 B, sha256 42ece741f5d2986c…, EBML head (1a45dfa3). Verified byte-IDENTICAL to tests/fixtures/generated/valid/mkv_h264_aac.mkv (same sha) — the committed class-A parent doubles as the H.264 codec fuzz seed (also mirrored in fuzz_demux_mkv corpus: same sha there too — the parent is committed in THREE places).
F-001 [S4]: triple-committed duplication of the same 26,105 bytes (valid fixture + codec corpus + demux corpus). Cargo-fuzz requires per-target corpus dirs so mirroring is the convention, but nothing (manifest or script) pins the mirrors to stay identical — drift would silently degrade seed coverage. Confidence: HIGH — Verification: read (sha compare).