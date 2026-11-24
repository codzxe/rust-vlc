# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__trunc2
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 64 B / e1e463ed48b22e64ce0483ceaf9a79fdf7bd154d8a6501cfb91aa1b6e37c6651 / RIFF WAVE PCM s16le 44.1k stereo (header-only truncation; batch manifest) |
| Registration | NOT in MANIFEST.toml (28-file AVI/MP4 set; no wav rows); no drift gate; no committed generator |
| Consumers (rg) | crates/demux/tests/wav_aiff_fixtures.rs L453 (class-B error-contract loop over `wav_pcm_s16le_44k1_st__{deriv}`) |
| Verdict | SHALLOW provenance, real usage. F-001 [S3]: unregistered corpus — 64-byte header-truncation probe with no ledger row, invisible to `cargo xtask fixtures --check`; CORPUS.md's drift claim does not reach it.
