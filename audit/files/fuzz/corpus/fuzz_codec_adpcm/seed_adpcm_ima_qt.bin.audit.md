# AUDIT: fuzz/corpus/fuzz_codec_adpcm/seed_adpcm_ima_qt.bin
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 11,764 B / 6558c0bedc9d23c574fea216018a94cc87bfda1c7678c2c2a95eb1c5612bb3b3 / raw data (batch manifest) |
| Generated vs handmade | Raw QuickTime IMA-ADPCM frames (no container — `file` says "data"); provenance undocumented — UNVERIFIED authorship. Sibling seeds (ima_ws, swf, yamaha, raw_facts_skeleton) cover the other ADPCM flavors. |
| Consumer | cargo-fuzz campaign only (fuzz_codec_adpcm target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002). Note: crates/codecs/tests/adpcm_fuzz_survival.rs ran in baseline (L937) but its input source was not verified to be THIS seed (UNVERIFIED). |
| Verdict | REAL bytes, SHALLOW reach. F-001 [S3]: never consumed by any committed run.
