# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__flip1
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 35,358 B / ec4ba2a27198821b1733921a545bfc7e57d1b3cd4106bb255095ee3abe1dcd80 / RIFF WAVE PCM s16le 44.1k stereo (batch manifest) |
| Registration | NOT in MANIFEST.toml (file_count=28, AVI/MP4-MJPEG families only — `rg wav_pcm_s16le_44k1_st__flip1 tests/fixtures/generated/MANIFEST.toml` → no hit); no drift gate; no committed generator |
| Consumers (rg) | crates/demux/tests/wav_aiff_fixtures.rs L453 `format!("generated/malformed/wav_pcm_s16le_44k1_st__{}", deriv.name)` (class-B error-contract loop) |
| Provenance claim | wav_aiff_fixtures.rs L37-43: derivatives "generated ... by deterministic byte surgery (verified this session: ... flips are seeded single-bit XORs at the recorded sites)" — prose only, no committed tool/ledger |
| Verdict | SHALLOW provenance, real usage. F-001 [S3]: unregistered corpus — no sha256 ledger row, not covered by `cargo xtask fixtures --check` (verify_tree only checks generate_all() output); CORPUS.md's blanket "drift is detected by cargo xtask fixtures --check" claim does not reach this file. Byte drift here is detectable only indirectly via the demux test's outcome constants.
