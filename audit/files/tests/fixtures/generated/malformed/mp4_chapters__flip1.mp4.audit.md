# AUDIT: tests/fixtures/generated/malformed/mp4_chapters__flip1.mp4
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 24,839 B, sha256 0cd32becf638ef71…, class-B derivative of generated/valid/mp4_chapters.mp4 (same size). Verified EXACTLY one differing byte at offset 24,799 (0x00 -> 0xff; XOR 0xff = 8 bits — NOT a single-bit flip despite the `flip1` name; the avi/mp4_mjpeg_pcm flip convention is Fisher-Yates bit flips, but this family has no in-tree generator to define its convention — provenance UNVERIFIED). No expectation TOML, no MANIFEST row for this family. Consumer: crates/demux/tests/mp4_fixtures.rs:235 (MALFORMED list -> malformed_mp4_fixtures_survive_the_fuzz_harness: absence-of-panic/hang/unbounded-alloc contract + typed/recovered outcome table).

F-001 [S3]: 8-bit byte difference shipped under a `flip1` name whose corpus-plan convention (§1 B2) defines as a single seeded bit flip — a naming/provenance inconsistency in an un-ledgered family. Confidence: HIGH — Verification: read (byte-diff).