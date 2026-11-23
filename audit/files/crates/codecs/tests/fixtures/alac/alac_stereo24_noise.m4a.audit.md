# AUDIT: crates/codecs/tests/fixtures/alac/alac_stereo24_noise.m4a (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Type / size | ISO Media, Apple iTunes ALAC (.m4a) / 214822 bytes (mdat 214044) |
| md5 (file) | 2db35c7dc6d8ed2128e3e27f19cfe2c1 |
| Asset verdict | REAL — genuine ALAC fixture, oracle claim verified against ffmpeg 7.1.5 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> ISO Media ALAC/AAC-LC .M4A; python3 MP4 box walk; `ffprobe -show_entries ...` -> alac/48000/2ch/24bpr/1.100000s; `ffmpeg -i ... -f s24le - | md5sum` -> `1ee6321067a82b421e33038f4fc00b70` |

## 1. One-paragraph verdict
The fixture is exactly what its consumers claim. My own box walk shows 13 audio
samples (stts: 12×4096 + 1×3648 frames = 52800 frames = 1.100 s at 48 kHz; stsz
count 13, stco 1 chunk), matching the "13 packets, 1.1 s, final partial frame"
description in tests/alac_bitexact.rs:30-34 and docs/formats/audio-alac.md:43.
ffprobe confirms alac, 2 channels, 48000 Hz, bits_per_raw_sample=24. The
ffmpeg-7.1.5 interleaved s24le dump MD5 I computed matches the pinned truth
byte-for-byte: `1ee6321067a82b421e33038f4fc00b70`. The white-noise maximum-
entropy purpose (rice-escape stressor) is consistent with the mdat being ~2x
the size of the 0.7 s music fixtures despite similar content length.

## 2. What actually works (verified)
— `ffprobe -v error -show_entries stream=... ` -> codec_name=alac
sample_fmt=s32p sample_rate=48000 channels=2 duration=1.100000
bits_per_raw_sample=24.
— `ffmpeg -v error -i <fixture> -f s24le - -map 0:a | md5sum` ->
`1ee6321067a82b421e33038f4fc00b70  -` (sandbox ffmpeg 7.1.5-0+deb13u1 — the
same version the pin claims to have been measured with).

## 3. Findings
None. Every checkable claim checks out:
| Claim | Source | Result |
|---|---|---|
| 2ch / 48 000 / s24le | alac_bitexact.rs:27 | verified via ffprobe |
| 13 packets incl. final partial | alac_bitexact.rs:32-33 | verified via stsz/stts (12×4096 + 1×3648) |
| 1.1 s | alac_bitexact.rs:32 | duration=1.100000 |
| ffmpeg MD5 `1ee6321067a82b421e33038f4fc00b70` | alac_bitexact.rs:27, audio-alac.md:43 | reproduced by my own ffmpeg run |
| max-entropy rice-escape stressor | audio-alac.md:43 | plausible (noise); entropy itself UNMEASURED — not load-bearing |

## 4. Consumers
crates/codecs/tests/alac_bitexact.rs (differential truth row, audited
separately); docs/perf/raw/r1/ffmpeg-benchmark-audio-utime.txt:17 (perf
baseline record); docs/formats/audio-alac.md:43 (format doc evidence table).
