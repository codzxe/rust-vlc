# AUDIT: fuzz/corpus/fuzz_codec_mjpeg/seed_mp4_mjpeg_pcm.mp4
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 49,152 B / 7c69b4b46674b5f91a82d62f79de8f8cf40515f30a0c766f46a251cc0f6a4a7e / ISO Media MP4 v1 (batch manifest) |
| Generated vs handmade | VERIFIED derived: python prefix-compare → seed == parent[:49152] True against tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4 (424,452 B MANIFEST-registered class-A parent) — an exact 48 KiB prefix truncation. |
| Consumer | cargo-fuzz campaign only (fuzz_codec_mjpeg target, nightly audit.yml tier — zero committed run evidence; see audit/files/fuzz/fuzz_targets/fuzz_demux_avi.rs.audit.md F-002). |
| Verdict | REAL seed (byte-verified parent prefix). F-001 [S3]: exercised by no committed run.
