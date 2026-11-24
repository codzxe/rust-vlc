# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__flip4.avi
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| File verdict | REAL (machine-generated fixture, sha256-verified against MANIFEST, consumed by 3 test lanes) |
| Reachable from shipped entry point? | NO — test fixture; consumers: fixtures_matrix.rs:355, recovery_matrix.rs:20, tools/mem-audit (W16 file) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 batch (audit/runs/W16-A08.fingerprints.txt); rg consumer sweep |

Binary fixture, 425,516 bytes, sha256 004ba5af67591b8d235e950ec729b0139329fb206877cdab8ca92030516f6d2b, `file`: RIFF AVI 320x240 30.00 fps MJPEG + PCM stereo 44100.
Generated: `cargo xtask fixtures` MANIFEST entry `generated/malformed/avi_mjpeg_pcm__flip4.avi` op "flip4" — sha256+size verified by my batch script (28/28 MANIFEST entries OK).
Consumers and pinned expectations (verbatim):
- fixtures_matrix.rs:351-353: `// flip4: the hdrl LIST size field and the RIFF size lie (plus two` `// advisory bytes) — the declared extent explodes past the window,`
- recovery_matrix.rs:20: `| avi flip4 | clean Ended | ≥1 non-fatal RecoveredDamage (the flip hit a chunk field; the clamp+resync recovered) |`
- tools/mem-audit/src/report.rs also references avi flip4-family paths (path-level check only, not exercised here).
Test honesty: same as stszlie0 — must()-gated path + exact verdicts; not vacuous.
Verdict: REAL adversarial fixture (lying RIFF/LIST sizes exercising clamp+resync).
