# AUDIT: crates/codecs/tests/fixtures/flac_s16_stereo_44100_seektable.flac (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| File verdict | REAL |
| Type / size | FLAC audio bitstream, 16-bit stereo 44.1 kHz, 198450 samples (4.5 s), 59380 bytes — `file` output |
| Structure (verified by my python walk) | `fLaC` magic → STREAMINFO (34 B) → VORBIS_COMMENT (44 B, vendor `Lavf61.7.103` — ffmpeg-authored) → **SEEKTABLE (108 B = 6 seek points, last-block flag)** → audio frames from byte 202. The filename's "seektable" promise is real. |
| Cited as evidence by | tests/flac_bitexact.rs, tests/flac_seek.rs, tests/flac_fuzz_survival.rs, tests/flac_vlc_differential.rs, tests/data/flac_bitexact_reference.txt (rg census) |
| Commands run | `file` → FLAC 16-bit stereo 44.1 kHz 198450 samples; python3 block walk (output above); `./target/debug/deps/flac_bitexact-0363eae80343785c` → 4 passed (exit 0); `./target/debug/deps/flac_seek-f28325ba6212a26f` → 4 passed (exit 0) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Genuine ffmpeg-authored FLAC fixture whose advertised properties all check out: magic, STREAMINFO geometry matches the filename (s16/stereo/44100), a real 6-point SEEKTABLE block is present (the feature the name advertises and flac_seek.rs needs), and both consuming suites (bit-exact decode vs reference; seek) pass under my execution. No findings.
