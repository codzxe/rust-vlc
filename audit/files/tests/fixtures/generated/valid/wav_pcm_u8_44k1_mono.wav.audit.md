# AUDIT: tests/fixtures/generated/valid/wav_pcm_u8_44k1_mono.wav
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 8898 B / e1a813dfc37f8ef95d6e50d2877332d468f5b01ee1ca077cd588c568f0a4e7ed |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by crates/demux/tests/wav_aiff_fixtures.rs (ffmpeg truth table) + crates/transcode/tests/wav_roundtrip.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (data audit)
Genuine valid WAV: PCM u8 (format tag 1, 1 channel, 44100 Hz, 8-bit), chunks fmt(16)/LIST(26)/data(8820) — 0.2 s of u8 mono audio; RIFF declared size matches the file exactly (8898 == 8898, batch.py `riff_decl=8898`). It is the u8 format lane in the WAV fixtures truth table and a transcode round-trip input; both suites pass in the baseline.

## 2. What was checked and how
- batch.py: 8898 B, sha256 e1a813df…, fmt {tag:1, channels:1, rate:44100, bits:8, align:1, byterate:44100}; data chunk 8820 B == 0.2 s.
- Consumers: crates/demux/tests/wav_aiff_fixtures.rs (TRUTHS table — `every_wav_fixture_matches_the_ffmpeg_truth`), crates/transcode/tests/wav_roundtrip.rs (round-trip lane).
- Execution: audit/runs/baseline/cargo-test-workspace.txt:2436-2452 — `every_wav_fixture_matches_the_ffmpeg_truth ... ok` (8/8); transcode suites ok (baseline, e.g. wav_roundtrip passed — see PACKAGE transcode section).
- Generated-vs-handmade: xtask-generated; NOT in MANIFEST.toml (python cross-check).

## 3. Findings
F-001 [S3] Not covered by the MANIFEST.toml drift gate, and outside the oracle_check.sh sidecar perimeter ({.avi,.mp4} only) — this WAV's external oracle lives only in the in-test ffprobe/md5 truth table. Verification: ran (manifest cross-check) + read (oracle_check.sh perimeter).
