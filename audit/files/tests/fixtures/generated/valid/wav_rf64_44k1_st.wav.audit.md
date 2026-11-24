# AUDIT: tests/fixtures/generated/valid/wav_rf64_44k1_st.wav
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 35,394 B / 8478de97ce85374b10989f9c63a0d6855b14a5ff3d185a26078680894c08d4f0 / MBWF/RF64 audio, stereo 44100 Hz (batch manifest — RF64 header verified by `file`) |
| Registration | NOT in MANIFEST.toml (28-file AVI/MP4 set; `rg wav_rf64 MANIFEST.toml` → no hit); no drift gate; no committed generator |
| Consumers (rg) | crates/media-probe/tests/registry_roundtrip.rs; crates/demux/tests/wav_aiff_fixtures.rs (RF64 arm of the wav family) |
| Verdict | REAL usage, SHALLOW provenance. F-001 [S3]: unregistered corpus (same class as wav_pcm_s16le_44k1_st__flip1 — no ledger row, no sha256 pin, `cargo xtask fixtures --check` cannot see it; CORPUS.md's drift claim does not reach this file). The file itself is a genuine RF64 container (file(1) identifies the format), exercised by the probe registry round-trip and demux tests.
