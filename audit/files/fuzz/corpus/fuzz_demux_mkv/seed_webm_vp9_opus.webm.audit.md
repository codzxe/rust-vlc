# AUDIT: fuzz/corpus/fuzz_demux_mkv/seed_webm_vp9_opus.webm
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 27,259 B, sha256 8b5c6ba741cb843b…, EBML/webm head (1a45dfa39f). Verified byte-IDENTICAL to tests/fixtures/generated/valid/webm_vp9_opus.webm — same bytes committed twice (tests fixture + fuzz seed). Consumers: fuzz_demux_mkv target (corpus convention, nightly tier).
F-001 [S4]: duplicated bytes without a linkage check (same as the mkv seed: no manifest pins the mirror pair identical). Confidence: HIGH — Verification: read (sha compare).