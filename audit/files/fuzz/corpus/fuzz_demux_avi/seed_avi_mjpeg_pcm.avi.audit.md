# AUDIT: fuzz/corpus/fuzz_demux_avi/seed_avi_mjpeg_pcm.avi
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 49,152 B, sha256 02189ecf6aef2d86…, valid RIFF/AVI head (52494646247e0600 = RIFF + AVI 1.0 style size). Verified byte-exact PREFIX of the class-A parent tests/fixtures/generated/valid/avi_mjpeg_pcm.avi (425,516 B) — a size-capped seed (first 48 KiB, header + early movi) so the fuzzer starts from real structure without a 416 KiB input. Consumers: fuzz_demux_avi target via cargo-fuzz corpus convention + nightly audit.yml fuzz tier; no code references the path (by design — libFuzzer reads the dir).
F-001 [S4]: seed is a raw prefix cut mid-structure (no idx1 present in the seed — index salvage paths start uncovered from this seed; the fuzz_lite AVI sweeps cover them instead). Confidence: HIGH — Verification: read (byte-diff).