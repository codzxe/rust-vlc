# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc3.avi (parts 1-13, L1-9722)
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 212,512 bytes / 5c1a6d05eeeec7f0b7e52233b5ad2323e541c3f7067bbed14465f5d8b5647f4a |
| Type | RIFF AVI, 320x240, 30.00 fps, MJPEG + PCM stereo 44100 |
| File verdict | REAL (verified class-B truncation derivative) |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 prefix check; rg consumers; MANIFEST sha compare |

## 1. Verdict
Committed-generated class-B derivative of `generated/valid/avi_mjpeg_pcm.avi` (parent 425,516 B). python3: `t3 == p[:212512]` → exact parent prefix. MANIFEST.toml pins sha256=5c1a6d05... and size 212_512 — both MATCH (computed digest identical). Consumer: crates/demux/tests/fixtures_class_b.rs (class-B matrix; `derivative("trunc3")` style) and tests/integration/first-vertical/tests/recovery_matrix.rs (trunc0-4 family), plus expectations/malformed/avi_mjpeg_pcm__trunc3.avi.toml. Baseline: fixtures_class_b.rs ran ok (audit/runs/baseline/cargo-test-workspace.txt:2242). The truncation cuts mid-movi at a recorded structure boundary (cut offset recorded in the expectation TOML).
## 2. What actually works (verified)
python3 prefix equality; MANIFEST.toml sha256/size match; baseline fixtures_class_b run (cargo-test-workspace.txt:2242 + ok results).
