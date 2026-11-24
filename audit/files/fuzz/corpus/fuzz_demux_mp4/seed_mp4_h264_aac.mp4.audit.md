# AUDIT: fuzz/corpus/fuzz_demux_mp4/seed_mp4_h264_aac.mp4
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 13,703 / 3acc7ebae6f41c31 |
| File verdict | REAL (seed corpus copy) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Seed for fuzz_demux_mp4: BYTE-IDENTICAL copy of the committed class-A parent
tests/fixtures/generated/valid/mp4_h264_aac.mp4 (13,703 B, sha256
3acc7ebae6f41c31 both — python3 compare). Documented copy class
(fuzz/README.md:101-104). The twin is consumed on stable by
crates/demux/tests/mp4_fixtures.rs:230 (its trunc1 derivative). The copy
here feeds the (unwired) campaign.
## 2. Verified
— python3 sha256 compare: identical to the generated parent.
— Consumer: fuzz/Cargo.toml bin fuzz_demux_mp4; twin consumed by
crates/demux/tests/mp4_fixtures.rs.
## 3. Findings
F-001 [S4] unlinked duplicate (same class as the adpcm seeds): no checksum
tie between the seed and the parent it copies; a parent regeneration
silently diverges the seed. Confidence: HIGH — Verification: executed
sha256 compare.
