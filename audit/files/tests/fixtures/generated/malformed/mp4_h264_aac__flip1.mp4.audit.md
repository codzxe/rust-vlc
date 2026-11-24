# AUDIT: tests/fixtures/generated/malformed/mp4_h264_aac__flip1.mp4
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 13,703 bytes / b2b03f6fa5da37f3b0affdb76347786596a736e1b4d14819916292ce6bad2e26 |
| Type | ISO Media (single seeded bit-flip) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 XOR diff vs parent; rg consumers |

## 1. Verdict
Exactly one bit flipped vs parent `generated/valid/mp4_h264_aac.mp4` (python3: same length 13,703, single diff at byte 8, `0x69 ^ 0x61 = 0x08` — one bit, inside the `ftyp` minor-version field). Genuine seeded single-bit flip, matching the `__flip1` naming convention documented in wav_aiff_fixtures.rs:37-41 ("flips are seeded single-bit XORs at the recorded sites"). Consumers: crates/demux/tests/mp4_fixtures.rs:225 (`MALFORMED` array) feeding `malformed_mp4_fixtures_survive_the_fuzz_harness` (in-process `fuzz::fuzz_demux_mp4`) and `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed` — both ok in baseline (audit/runs/baseline/cargo-test-workspace.txt:2373-2374).
## 2. Findings
F-001 [S3]
Same family gap as mkv/wav: no MANIFEST.toml row (python3 scan: NOT IN MANIFEST), outside the 55-file drift gate (full_gate.txt:54), no expectations/ TOML for the mp4_h264_aac family (expectations/malformed/ contains only mp4_mjpeg_pcm__*). Confidence: HIGH — Verification: ran.
## 3. What actually works (verified)
python3 XOR diff (1 bit); baseline tests ok (cargo-test-workspace.txt:2373-2374).
