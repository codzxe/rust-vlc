# AUDIT: fuzz/corpus/fuzz_demux_adts/seed_adts_fuzz_base.aac
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 2,500 / 1027bc9b5bd43503 |
| File verdict | REAL (seed corpus copy) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Seed for fuzz_demux_adts: BYTE-IDENTICAL copy of the committed crate fixture
crates/codecs/tests/fixtures/fuzz_base_lc.aac (2,500 B, sha256
1027bc9b5bd43503 both — python3 compare). The twin feeds the stable AAC
fuzz-survival/bitexact suites; the seed feeds the unwired campaign.
## 2. Verified
— python3 sha256 compare: identical.
— Consumer: fuzz/Cargo.toml bin fuzz_demux_adts (2-seed corpus: this +
adts_lc_stereo); twin consumed by crates/codecs/tests aac suites.
## 3. Findings
F-001 [S4] unlinked duplicate (no tie between seed and source fixture).
Confidence: HIGH — Verification: executed sha256 compare.
