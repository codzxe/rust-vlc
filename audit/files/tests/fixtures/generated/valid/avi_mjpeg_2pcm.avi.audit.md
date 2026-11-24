# AUDIT: tests/fixtures/generated/valid/avi_mjpeg_2pcm.avi
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 497,160 B / 14d3a0e9a842e66710524dcbd6ccd17caf462541a2d8335327854698479868f3 / RIFF AVI, 320×240 10fps MJPEG + 2×PCM stereo (batch manifest: audit/runs/W16-A02.fixture-manifest.tsv) |
| Registration | MANIFEST.toml row (class A, op parent, same sha256 VERIFIED against my computed hash; drift-gated by `cargo xtask fixtures --check`) |
| Consumers (rg) | crates/playback-engine/tests/track_switch{,_fixture}.rs, tests/soak/scenario.rs, scenarios_b/host.rs + scenario_08, soak scenarios.toml ×4; expectations/valid/avi_mjpeg_2pcm.avi.toml; structure.json sibling |
| Doc evidence | docs/perf/baseline.md L14+L80 (open→media.info-ready 20 events, 1.034ms); docs/soak/raw/*.md fixtures lists |
| Verdict | REAL — registered, hash-verified, multi-consumer class-A parent (CC0 per CORPUS.md). No findings.
