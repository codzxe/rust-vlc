# AUDIT: fuzz/corpus/fuzz_codec_flac/seed_flac_s16_st.flac
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 14,581 / ac3f81b4cd3fc9c0 |
| File verdict | REAL (seed corpus copy) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Seed for fuzz_codec_flac: BYTE-IDENTICAL copy of the committed crate fixture
crates/codecs/tests/fixtures/flac_s16_stereo_44100.flac (14,581 B, sha256
ac3f81b4cd3fc9c0 both — python3 compare). Twin feeds the stable FLAC
bitexact/seek suites; the seed feeds the unwired campaign.
## 2. Verified
— python3 sha256 compare: identical; < 50 KB budget.
— Consumer: fuzz/Cargo.toml bin fuzz_codec_flac (3-seed corpus: mono16,
s16_st, s24); twin consumed by crates/codecs/tests (flac suites).
## 3. Findings
F-001 [S4] unlinked duplicate (no tie between seed and source fixture).
Confidence: HIGH — Verification: executed sha256 compare.
