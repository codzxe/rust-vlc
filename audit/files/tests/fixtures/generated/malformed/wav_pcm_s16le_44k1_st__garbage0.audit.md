# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__garbage0
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| File verdict | REAL (fixture consumed by a pinned-outcome demux test) — but OUTSIDE the MANIFEST drift gate (see MANIFEST audit) |
| Reachable from shipped entry point? | NO — test fixture; consumer: crates/demux/tests/wav_aiff_fixtures.rs (path built by format! at L453) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | sha256 batch (audit/runs/W16-A08.fingerprints.txt); rg consumer sweep |

Binary fixture, 39,454 bytes, sha256 f10e6af3f98229b9f9b6c36ac61bd2785da19b25bf625f69a7953bb110d27d72, `file`: RIFF WAVE PCM 16-bit stereo 44100 (no extension).
Handmade-vs-generated: NOT in tests/fixtures/generated/MANIFEST.toml (28 entries cover only the avi_mjpeg_pcm/mp4_mjpeg_pcm families); per wav_aiff_fixtures.rs:37-41 the wav family was produced by "deterministic byte surgery" — verbatim: `// garbage0 appends 4096 bytes past the container` `// extent). Their pinned outcomes are the measured demuxer behavior,` — i.e. session-generated + pinned, not xtask-managed, NOT drift-gated.
F-001 [S4]: byte-verified derivation (python3): garbage0 == parent (35,358 B)
byte-identical prefix + 4,096 appended bytes (first bytes 90 95 a6 61 …) — exactly
the documented "garbage0 appends 4096 bytes past the container extent". The file
is correct; but it is NOT in tests/fixtures/generated/MANIFEST.toml (28 entries
cover only the avi_mjpeg_pcm/mp4_mjpeg_pcm families) — any silent mutation of
it is undetectable by `cargo xtask fixtures --check`.
Test honesty: wav_aiff_fixtures.rs asserts exact terminal/reasons per derivative (WavDerivative table) — a missing or re-mutated file fails; honest. Baseline: these demux tests passed (cargo-test-workspace.txt; demux wav tests ok — no failures listed for wav_aiff_fixtures).
Verdict: REAL fixture, real adversarial value (trailing-garbage-after-EOF surface).
