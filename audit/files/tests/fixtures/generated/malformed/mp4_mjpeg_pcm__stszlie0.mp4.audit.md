# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__stszlie0.mp4
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| File verdict | REAL (machine-generated fixture, sha256-verified against MANIFEST, consumed by 2 test lanes) |
| Reachable from shipped entry point? | NO — test fixture; consumed by crates/media-probe/tests/fixtures_matrix.rs:489,550 and tests/integration/first-vertical/tests/recovery_matrix.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 batch script (audit/runs/W16-A08.fingerprints.txt); rg consumer sweep |

Binary fixture, 424,452 bytes, sha256 c7d9ece0d4e564cda2025b616ccf305abd0d540e9cdbe214450e3ce424097185, `file`: ISO Media, MP4 Base Media v1.
Generated: `cargo xtask fixtures` (MANIFEST.toml:1-8, entry `generated/malformed/mp4_mjpeg_pcm__stszlie0.mp4`, class "A", op "stsz-count-lie"); my batch script verified sha256+size against MANIFEST — 28/28 entries OK, this one included (not handmade).
Consumers and pinned expectations (verbatim):
- fixtures_matrix.rs:483-486: `// stszlie0: the video stsz sample_count is 2^31−1 (offset 827) —` … `assert_eq!(derivation.op, "stsz-count-lie");`
- recovery_matrix.rs:28: `| mp4 stszlie0 | clean Ended (the stsz lie recovered by resync) | ≥1 non-fatal RecoveredDamage |`
Test honesty: consumer tests assert exact verdicts (probe Hit score 95, damage ≥1) — a corrupted/absent fixture fails the test (fixture_path().must()), not vacuously green.
Verdict: REAL fixture with real adversarial value (the lie is inside the 64 KiB sniff window but in an unopened box).
