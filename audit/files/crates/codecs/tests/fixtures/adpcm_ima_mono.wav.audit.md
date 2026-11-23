# AUDIT: crates/codecs/tests/fixtures/adpcm_ima_mono.wav (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A10 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Type / size / sha256 | RIFF WAVE, IMA ADPCM (fmt tag 0x11), mono 8000 Hz / 6,238 bytes / 7b854ad888f09e83c32d94c4fa219cd231df32cdd30d9c3419f41c9d1a2c4763 (md5 d07ab73cc2f3c905fdbe82f5a8174f83) |
| File verdict | REAL (genuine ffmpeg-authored IMA-ADPCM fixture; every recorded fact reproduced live, incl. the pinned decode MD5 via an independent ffmpeg run) |
| Reachable from shipped entry point? | N/A (test fixture; WAV + adpcm_ima_wav is a shipped open path — demux wav → adpcm decoder) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `stat`; `sha256sum`/`md5sum`; `ffprobe` (adpcm_ima_wav, 8000, 1ch, 4 bps, 1.5s); python3 RIFF chunk walk (fmt 0x11/1ch/8kHz/blockalign 1024; data = 6,144 B = exactly 6 × 1024 blocks); `ffmpeg -f md5` -> 8a8146087f7573fa71963c3ae2c65f3b (== pinned truth); `adpcm_corpus_differential-b20f58c6c52dee82` -> 2/2 pass; `adpcm_fuzz_survival-4bcf0cfa52427a86` -> 4/4 pass |

## 1. One-paragraph verdict
A genuine, truthfully-named IMA-ADPCM fixture with a fully reproduced
evidence chain. `file` + ffprobe confirm the name (WAVE, IMA ADPCM
adpcm_ima_wav, mono, 8000 Hz, 1.5s); my own RIFF walk confirms the
structure the differential's header table claims — fmt tag 0x11, 1
channel, 8000 Hz, blockalign 1024, and a data chunk of exactly 6,144
bytes = 6 × 1024-byte blocks (adpcm_corpus_differential.rs:25 "6 × 1024",
worklog:8942 "6×1024 B"). The pinned truth row (TRUTHS, :64-71: frames
12,246, MD5 8a8146087f7573fa71963c3ae2c65f3b) is doubly verified: the
committed-fixture test passed live (asserting Lumen's decode MD5 + frame
count + zero first-PTS), and my independent `ffmpeg -f md5` run of the
same file reproduces the exact recorded digest — proving the pin is
genuinely ffmpeg-derived and that Lumen decodes bit-exactly against the
oracle today. Provenance recorded at worklog:8942 (authored by ffmpeg
`-c:a adpcm_ima_wav`, sine 440 Hz 1.5 s 8 kHz) — consistent with the
12,246-sample data-derived window. The fuzz-survival suite uses these
bytes as mutation substrate (4/4 pass live, 0.71s).

## 2. What actually works (verified)
- `ffmpeg -i <fixture> -c:a pcm_s16le -f md5 -` -> `MD5=8a8146087f7573fa
  71963c3ae2c65f3b` — byte-identical to the TRUTHS pin.
- `./target/debug/deps/adpcm_corpus_differential-b20f58c6c52dee82` ->
  `committed_fixtures_decode_bit_exact ... ok` (asserts frames == 12,246,
  digest == pin, first pts zero), `real_corpus_decodes_bit_exact ... ok`
  (loud SKIP observed under --nocapture: corpus absent — honest).
- `./target/debug/deps/adpcm_fuzz_survival-4bcf0cfa52427a86` -> 4/4 pass
  (deterministic substrate decode, mutated fmt headers typed, mutated
  blocks survive typed).

## 3. Findings
None. Codec, channel count, rate, block geometry, sample count, and both
MD5s (container and decoded PCM) all match the recorded facts; the
fixture is live-exercised on two suites.

## 4-9. (binary asset — short-form)
- Cited as evidence by: adpcm_corpus_differential.rs:25/:64-71 (TRUTHS
  row), adpcm_fuzz_survival.rs:112 (mutation substrate),
  docs/formats/audio-adpcm.md:45 ("mono | 12 246 | bit-exact"),
  worklog:8942 (authored provenance row). Honest use throughout.
- Untouched on disk; the decoded-PCM MD5 pin (not a container digest)
  carries the regression value — and it cross-checks against live ffmpeg.
