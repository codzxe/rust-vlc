# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__flip4.mp4.toml

| Field | Value |
|---|---|
| Auditor | W16-RR5 (orchestrator-completed; orig W16-A10) |
| Date | 2026-11-24 |
| File verdict | REAL (expectation data, xtask-generated) |
| Reachable from shipped entry point? | N/A (test expectation) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | sha256sum (fb8774e62ca74d8d), head (schema/derivation fields), rg -> tools/xtask/src/cmd/fixtures.rs:4815 (emitter path) |

## 1. One-paragraph verdict
xtask-generated expectation record for the flip4 (4-bit-flip) malformed
MP4 child of valid/mp4_mjpeg_pcm.mp4: schema expectation/1, class B,
generator `cargo xtask fixtures`, flip_extent 512, explicit sha256 +
size. This is the drift-gated half of the fixtures system (W16-A07
confirmed the generated-mp4 family is INSIDE the 28-row manifest, unlike
the ffmpeg-authored families).
