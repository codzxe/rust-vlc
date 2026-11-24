# AUDIT: fuzz/corpus/fuzz_codec_adpcm/seed_adpcm_swf.bin
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 36,909 / 77def57669a82780 |
| File verdict | REAL (seed corpus copy) |
| Reachable? | YES — cargo-fuzz corpus convention (fuzz/README.md:112-116); non-empty corpus enforced by scripts/fuzz_registry_check.sh:64 (full_gate step 6) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Seed corpus file for fuzz_codec_adpcm. BYTE-IDENTICAL copy of the committed
crate fixture crates/codecs/tests/fixtures/breadth/swf_payload.bin (sha256
77def57669a82780 both, 36,909 B both — python3 compare). Documented as such
by fuzz/README.md:101-104 ("copied from the committed CC0 fixtures"). The
copy is the campaign's entry point for SWF-ADPCM payload fuzzing; the
campaign itself is UNWIRED (nightly Lead decision), so today this file's
only stable consumers are the registry gate (presence) and the crate's
breadth tests that use the twin fixture.
## 2. Verified
— python3 sha256: identical to breadth/swf_payload.bin; < 50 KB per README
claim (:101-102 "every file < 50 KB").
— Consumer: fuzz/Cargo.toml bin fuzz_codec_adpcm + corpus dir presence
(fuzz_registry_check.sh); twin consumed by
crates/codecs/tests/fixtures/breadth (ws/swf breadth suites).
## 3. Findings
F-001 [S4] duplicated bytes at two tree locations with no link or checksum
tie — if the breadth fixture is ever regenerated, this seed silently
diverges (the registry gate checks only non-emptiness, not the copy
relationship). Confidence: HIGH — Verification: executed sha256 compare.
