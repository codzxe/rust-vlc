# AUDIT: fuzz/corpus/fuzz_demux_ps/seed_crossformat_mp4.mp4
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 13,703 B, sha256 3acc7ebae6f41c31…, ftyp box head (0000002066747970). Verified byte-IDENTICAL to tests/fixtures/generated/valid/mp4_h264_aac.mp4. Name says what it is: a cross-format seed — an MP4 fed to the MPEG-PS demuxer target so the fuzzer starts at the typed-refusal boundary (PS parser must reject MP4 bytes, not panic). The PS corpus pairs it with seed_crossformat_hls_ts.ts (a TS file) — the cross-format trio is deliberate coverage of mislabeled-input survival.
F-001 [S4]: duplicated bytes (identical to valid fixture) with no pin keeping the mirror pair in sync. Confidence: HIGH — Verification: read (sha compare).