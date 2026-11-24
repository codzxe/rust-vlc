# AUDIT: fuzz/corpus/fuzz_codec_aac/seed_adts_fuzz_base.aac
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 2,500 B / 1027bc9b5bd43503dd257236d9b3603f89219c33f6ab3a0cc11e32c184d1029c / MPEG ADTS AAC v4 LC 22.05kHz mono (batch manifest) |
| Generated vs handmade | Real ADTS AAC frames per file(1); name "fuzz_base" suggests the stable AAC fuzz-survival base input. Provenance undocumented — UNVERIFIED authorship. |
| Consumer | cargo-fuzz campaign only (fuzz_codec_aac target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002). Note: crates/codecs/tests/aac_fuzz_survival.rs exists (baseline L904) but its input source was not verified to be THIS seed (UNVERIFIED). |
| Verdict | REAL bytes, SHALLOW reach. F-001 [S3]: never consumed by any committed run.
