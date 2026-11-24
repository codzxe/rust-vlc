# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc2.avi
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 324 B / 9ec527a65c02aea7ff280aa05bd401d9e52722205c2c0ebb00199064e87b7aee / RIFF AVI 320×240 30fps MJPEG+PCM (truncated) (batch manifest) |
| Registration | MANIFEST.toml row (class B, op trunc2, parent avi_mjpeg_pcm.avi) — sha256 VERIFIED: manifest row (sed -n '68,74p') equals my computed hash; drift-gated |
| Consumers (rg) | crates/codecs/tests/fuzz_survival.rs (class-B survival pool); crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs |
| Verdict | REAL — registered, hash-verified, error-contract fixture with two real consumers. No findings.
