# AUDIT: tests/fixtures/generated/malformed/mkv_h264_aac__flip1.mkv
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 26,105 B, sha256 12151d0963d7fb5a…, class-B derivative of generated/valid/mkv_h264_aac.mkv (same size, parent sha 42ece741…). Verified EXACTLY one differing byte at offset 474: 0x64 -> 0x74 (XOR 0x10 — exactly ONE bit, mid-file inside a Matroska element around the track-name region). Consumer: crates/demux/tests/mkv_fixtures.rs:355 (malformed_derivatives_are_typed_or_recover_never_panics) + worklog 5.B fuzz-lite record (18 derivatives). No expectation TOML / MANIFEST row.

F-001 [S3]: same family-wide ledger gap (no TOML, no manifest row). Confidence: HIGH — Verification: read (byte-diff).