# AUDIT: fuzz/corpus/fuzz_codec_pcm/seed_pcm_s16le.bin
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 2,048 B, sha256 56f181b7d38424af…, raw little-endian s16 samples (head 00c089c012c19bc1 — plausible zero-crossing waveform bytes, non-repeating). Raw ES seed for the PCM codec target. Sibling seeds: seed_pcm_f32le.bin, seed_pcm_u8.bin (format matrix). Consumers: fuzz_codec_pcm target via corpus convention.
