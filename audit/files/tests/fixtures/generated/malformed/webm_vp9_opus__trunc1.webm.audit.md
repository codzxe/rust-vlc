# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__trunc1.webm
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 131 B, sha256 dbe322f9a001dd29…, class-B truncation = byte-exact 131-byte prefix of generated/valid/webm_vp9_opus.webm (verified prefix identity True; WebM DocType head 1a45dfa3 9f…). Consumer: mkv_fixtures.rs:360 (survival + outcome table). No expectation TOML / MANIFEST row.

F-001 [S3]: same ledger-gap family. Confidence: HIGH — Verification: read (prefix check).