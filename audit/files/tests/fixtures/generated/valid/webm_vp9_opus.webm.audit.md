# AUDIT: tests/fixtures/generated/valid/webm_vp9_opus.webm
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 27,259 B, sha256 8b5c6ba741cb843b…, valid WebM (EBML head 1a45dfa3 9f, DocType webm). Verified byte-IDENTICAL to fuzz/corpus/fuzz_demux_mkv/seed_webm_vp9_opus.webm — committed twice. Consumers: crates/demux/tests/mkv_fixtures.rs (the WebM branch — packet-count and bit-exactness lanes, :13/:128/:156) + fuzz_demux_mkv corpus. VP9 payload demuxes but has NO decoder (worklog 5.B honest record) — the fixture tests demux, not decode.

F-001 [S4]: duplicated bytes (fuzz seed mirror) with no pin keeping the pair in sync. Confidence: HIGH — Verification: read (sha compare).