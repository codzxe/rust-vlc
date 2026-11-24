# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__trunc0
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 12 B / 738e75e497c951ad53a83592656afa463a5c9d82efafa81a1bdb20f36ec90cb5 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by crates/demux/tests/wav_aiff_fixtures.rs (dynamic derivative walk + fuzz-harness totality test) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (data audit)
The minimal `trunc0` WAV: exactly 12 bytes — `RIFF` + size field still claiming 35358 + `WAVE`, zero chunks. The consuming test pins it as a typed truncation failure at `start()` (the WAV_DERIVATIVES table) and also feeds it through `demux::wav::fuzz::fuzz_demux_wav` in the totality test; both pass in the committed baseline.

## 2. What was checked and how
- batch.py: 12 B, sha256 738e75e4…, RIFF/WAVE magic, `riff_decl=35358` vs actual 12 — lying-size truncation class; PAIRS: pure prefix of the valid parent (0 diff bytes, delta -35346).
- Consumers: crates/demux/tests/wav_aiff_fixtures.rs:449+ derivative table (`trunc0` entry), :707-716 `every_wav_and_aiff_fixture_survives_the_fuzz_harnesses` (includes trunc0 bytes).
- Execution: audit/runs/baseline/cargo-test-workspace.txt:2436-2452 — wav_aiff_fixtures 8/8 ok (both `wav_mutated_family_outcomes_are_pinned_exactly` and the fuzz-harness totality test).
- Generated-vs-handmade: xtask-generated; NOT in MANIFEST.toml (python cross-check).

## 3. Findings
F-001 [S3] Not covered by the MANIFEST.toml drift gate (wav family absent from the 28 pinned entries). Verification: ran (python manifest cross-check).
