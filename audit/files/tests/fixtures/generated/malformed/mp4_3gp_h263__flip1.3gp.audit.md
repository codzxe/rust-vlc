# AUDIT: tests/fixtures/generated/malformed/mp4_3gp_h263__flip1.3gp
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 9,520 B, sha256 3ba8e2d2d2473034…, class-B derivative of generated/valid/mp4_3gp_h263.3gp (same size). Verified EXACTLY one differing byte at offset 8: 0x33 ('3') -> 0x3b — the first byte of the ftyp MAJOR BRAND, i.e. the brand string "3gp4" becomes ";gp4" (multi-bit XOR 0x08? no — 0x33^0x3b=0x08, single bit flip: 0x33=00110011, 0x3b=00111011, XOR=00001000 — exactly ONE bit). Consumer: crates/demux/tests/mp4_fixtures.rs:233 (fuzz-harness survival + outcome pinning). No expectation TOML / MANIFEST row (family uncovered by the ledger).

F-001 [S3]: no expectation TOML / ledger row (doc 10 §5.4 claims one per fixture; 59/81 malformed uncovered) — pinning exists only in the demux test table. Confidence: HIGH — Verification: read.