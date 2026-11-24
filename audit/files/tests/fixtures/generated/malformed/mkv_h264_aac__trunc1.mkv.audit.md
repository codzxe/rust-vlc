# AUDIT: tests/fixtures/generated/malformed/mkv_h264_aac__trunc1.mkv
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 145 B, sha256 e1d7797e0d689983…, class-B truncation = byte-exact 145-byte prefix of generated/valid/mkv_h264_aac.mkv (verified prefix identity True). 145 B covers the EBML header + DocType + early top-level elements (head 1a45dfa3 a3428681 0142f781 0142f281 0442f381) — the truncation lands before any Segment payload, exercising the typed truncated-EBML refusal. Consumer: mkv_fixtures.rs:351 (survival + typed/recover outcome table — trunc0/1 rows pin `demux.mkv.truncated`-family outcomes). No expectation TOML / MANIFEST row.

F-001 [S3]: same ledger-gap family. Confidence: HIGH — Verification: read (prefix check).