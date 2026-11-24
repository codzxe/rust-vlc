# AUDIT: tests/fixtures/generated/valid/mkv_h264_aac.mkv
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 26,105 B / 42ece741f5d2986c1fde4066b8a3fe6d5c83b4e435ba843edc77ffa237dbcbdc / Matroska data (batch manifest) |
| Registration | NOT in MANIFEST.toml (`rg mkv_h264_aac tests/fixtures/generated/MANIFEST.toml` → no hit; 28-file AVI/MP4 set); no drift gate; no committed generator |
| Consumers (rg) | crates/media-probe/tests/registry_roundtrip.rs; crates/demux/tests/mkv_fixtures.rs (valid-parent arm) |
| Verdict | REAL usage, SHALLOW provenance. F-001 [S3]: unregistered corpus — a valid Matroska parent with real consumers but no sha256 ledger row, invisible to `cargo xtask fixtures --check`; CORPUS.md's "drift is detected" claim does not reach it. (Sibling malformed mkv_h264_aac__* derivatives are consumed by mkv_fixtures.rs L352-358 but equally unregistered.)
