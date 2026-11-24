# AUDIT: fuzz/corpus/fuzz_codec_mp3/seed_mp3_48k.mp3
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 12,564 / 45cadbb515ac40a9 |
| File verdict | REAL (seed corpus copy) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Seed for fuzz_codec_mp3: BYTE-IDENTICAL copy of the committed crate fixture
crates/codecs/tests/fixtures/mono48.mp3 (12,564 B, sha256 45cadbb515ac40a9
both — python3 compare; NOT cbr128.mp3 / vbr_q4.mp3, both distinct). The
twin feeds the stable MP3 bitexact/corpus suites; the seed feeds the
unwired campaign.
## 2. Verified
— python3 sha256 compare: identical to mono48.mp3; distinct from the other
two MP3 fixtures.
— Consumer: fuzz/Cargo.toml bin fuzz_codec_mp3 (3-seed corpus: 48k,
mpeg2_24k, vbr); twin consumed by crates/codecs/tests (mp3 suites).
## 3. Findings
F-001 [S4] unlinked duplicate (no tie between seed and source fixture).
Confidence: HIGH — Verification: executed sha256 compare.
