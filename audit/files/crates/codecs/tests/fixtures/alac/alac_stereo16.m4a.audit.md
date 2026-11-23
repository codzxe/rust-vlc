# AUDIT: crates/codecs/tests/fixtures/alac/alac_stereo16.m4a (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | ISO Media M4A, ALAC 2ch 44100 Hz 16-bit (ffprobe: codec_name=alac, bits_per_raw_sample=16) / 42907 bytes / f0662338bd3e159818b6decc741ede8292fb51270af288387f422d6fa2a7dacd |
| File verdict | REAL (bit-exact decode evidence, executed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> ISO Media Apple iTunes ALAC/AAC-LC .M4A; `ffprobe` -> alac/44100/2ch/16-bit; `sha256sum`; `./target/debug/deps/alac_bitexact-c46b612c74b2627b --nocapture` -> 1 passed / 0 failed, 0.12s |

## 1. Verdict
Real ALAC fixture, lane 1 of 5 in the ALAC bit-exact suite. The consuming test (`fixtures_bitexact_vs_ffmpeg`, alac_bitexact.rs:69-78 TRUTHS[0]) decodes this file through the in-house ALAC decoder and requires the interleaved s16le dump's MD5 to equal the committed ffmpeg-measured `e33ff912590e04d68117017a1a2ccd23`, plus channel/rate/packet-count (8) /duration (0.7 s) assertions — it ran green when I executed it. Also feeds `alac_bench.rs` and `alac_fuzz_survival.rs` (perf numbers committed under docs/perf/raw/r1/).

## 2. Test usage (verified by execution)
`./target/debug/deps/alac_bitexact-c46b612c74b2627b --nocapture` -> **1/1 pass** (`fixtures_bitexact_vs_ffmpeg`, covers all five fixtures including this one, 0.12s — real decode work). No skip guards in this suite; everything runs.

## 3. Matrix evidence
This is the base-width (16-bit, no extra-bits splice) lane backing the alac decoder's bit-exact claims; the 24-bit/noise siblings cover the harder paths. The committed MD5 + live pass = consistent, no inflation for this asset.
