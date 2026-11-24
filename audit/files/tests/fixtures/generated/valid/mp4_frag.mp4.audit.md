# AUDIT: tests/fixtures/generated/valid/mp4_frag.mp4
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 14,045 bytes / 530bad7f9faa3fe095988bf7ff281254ee2bbeffade4b06bfc1431da3f57a3bc |
| Type | ISO Media, MP4 Base Media v5 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; rg consumers |

## 1. Verdict
Fragmented-MP4 valid fixture (ffmpeg `-movflags +frag_keyframe+empty_moov` per crates/demux/tests/mp4_fixtures.rs:11,54-59: `fixture: "generated/valid/mp4_frag.mp4"` with `fragmented: true`; asserted at :155-156 `mp4.is_fragmented()` == spec.fragmented). h264+aac codecs per the TRUTHS-style table. Consumers: crates/demux/tests/mp4_fixtures.rs:54,155; malformed derivatives `mp4_frag__flip1.mp4`/`mp4_frag__trunc0.mp4` in the MALFORMED array (:239-240). Baseline: `valid_mp4_fixtures_demux_fully ... ok`, `malformed_mp4_fixtures_survive_the_fuzz_harness ... ok` (cargo-test-workspace.txt:2373-2375).
## 2. Findings
F-001 [S3]
No MANIFEST.toml row, no CORPUS.md ledger row, outside the 55-file drift gate (full_gate.txt:54) — same family gap as mkv/wav/aiff/ivf fixtures. Confidence: HIGH — Verification: ran (python3 manifest scan: NOT IN MANIFEST).
## 3. What actually works (verified)
Consumer assertions are real (is_fragmented pin) and pass in baseline (cargo-test-workspace.txt:2373-2375).
