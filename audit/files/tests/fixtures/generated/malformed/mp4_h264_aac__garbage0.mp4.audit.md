# AUDIT: tests/fixtures/generated/malformed/mp4_h264_aac__garbage0.mp4
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 17,799 B / af87ac03227cd7855ddeee65baf46ebbdf527cbd4118383181e2df481a168c13 / ISO Media MP4 v1 (batch manifest) |
| Registration | NOT in MANIFEST.toml (28 rows cover avi_mjpeg_pcm/avi_mjpeg_2pcm/mp4_mjpeg_pcm families only; `rg mp4_h264_aac MANIFEST.toml` → no hit); no drift gate; no committed generator |
| Consumers (rg) | crates/demux/tests/mp4_fixtures.rs L228 (`generated/malformed/mp4_h264_aac__garbage0.mp4` in the class-B refusal pool) |
| Verdict | SHALLOW provenance, real usage. F-001 [S3]: unregistered corpus — the H.264/AAC MP4 derivative family is invisible to `cargo xtask fixtures --check` (which verifies only the MJPEG/PCM first-vertical families); no sha256 ledger row exists for these bytes.
