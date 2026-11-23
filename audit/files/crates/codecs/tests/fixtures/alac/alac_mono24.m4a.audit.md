# AUDIT: crates/codecs/tests/fixtures/alac/alac_mono24.m4a (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| File verdict | REAL (genuine 24-bit mono ALAC M4A; consumed by the passing baseline bitexact suite) |
| Reachable from shipped entry point? | N/A (test fixture; the mp4/m4a container lane + alac decoder it exercises are CLI-reachable) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `ffprobe`, rg citing tests (exit 0); baseline cite |

## 1. One-paragraph verdict
A real, truthfully-named asset. `file` reports "ISO Media, Apple iTunes ALAC/AAC-LC (.M4A) Audio"; live ffprobe confirms the filename's every claim: `codec_name=alac, sample_fmt=s32p, bits_per_raw_sample=24, sample_rate=48000, channels=1, duration=0.7, nb_frames=9` — mono, 24-bit (raw), 48 kHz. 58,974 bytes. It is one of five ALAC fixtures (mono16/mono24/stereo16/stereo24/stereo24_noise) consumed by `crates/codecs/tests/alac_bitexact.rs`, whose doc table (:26) pins this file's digest row (`1ch / 48_000 / s24le / MD5 109eea10ed4c881a5516143c817f1595`) and whose single test `fixtures_bitexact_vs_ffmpeg` passed in the baseline (:954-958, 0.12s — real decode of committed fixtures against recorded ffmpeg digests).

## 2. What actually works (verified)
- `file` + `ffprobe` outputs above (pasted in run log).
- Baseline cargo-test-workspace.txt:954-958: `fixtures_bitexact_vs_ffmpeg ... ok`.
- Citation chain verified (od-checked past the display mangling): alac_bitexact.rs:26 (doc table row) and :98 (fixture list).

## 3. Findings
None. Name, format, bit depth, channel count, and rate all match; digest row recorded and exercised.

## 4-9. (binary asset — short-form)
- Cited as evidence by: alac_bitexact.rs (fixtures_bitexact_vs_ffmpeg) — honest use (decode-then-digest vs recorded ffmpeg reference).
- No doc/matrix row cites this file directly beyond the test's own table.
- Untouched; no content modification.
