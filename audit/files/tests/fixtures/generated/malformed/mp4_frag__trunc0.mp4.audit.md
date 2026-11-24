# AUDIT: tests/fixtures/generated/malformed/mp4_frag__trunc0.mp4
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 1,400 B, sha256 b94745592d1203d9…, class-B truncation = byte-exact 1,400-byte prefix of generated/valid/mp4_frag.mp4 (14,045 B; verified prefix identity True). Head shows ftyp isom5/isom6/mp41 + moov — cut lands mid-moov of the fragmented MP4, exercising the fragmented-box truncation path. Consumer: crates/demux/tests/mp4_fixtures.rs:238 (MALFORMED survival suite). No expectation TOML / MANIFEST row.

F-001 [S3]: same ledger-gap family. Confidence: HIGH — Verification: read (prefix check).