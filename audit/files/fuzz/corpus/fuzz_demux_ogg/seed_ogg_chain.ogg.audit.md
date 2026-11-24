# AUDIT: fuzz/corpus/fuzz_demux_ogg/seed_ogg_chain.ogg
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 7,041 B, sha256 33cec020c090df71…, OggS page head (4f676753 0002). Verified byte-IDENTICAL to tests/fixtures/generated/valid/ogg_chain_two_links.ogg (the two-link chained-stream parent). Seed exercises the chain/discontinuity path from the first fuzzer input.
F-001 [S4]: duplicated bytes, no sync pin (same family as the mkv/webm/mp4 mirrors). Confidence: HIGH — Verification: read (sha compare).