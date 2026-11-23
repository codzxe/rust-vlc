# AUDIT: crates/codecs/tests/fixtures/flac_s24_stereo_48000.flac (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | FLAC audio bitstream, 24-bit stereo 48 kHz, 19200 samples (`file` + ffprobe concur; ffprobe sample_fmt s32 = the 24-in-32 container) / 44112 bytes / d9968c8399532f30d2ab4c5af8a78267af847d9f6453c2e8df6cd50077b526d2 |
| File verdict | REAL (byte-exact decode evidence, executed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> FLAC 24 bit stereo 48 kHz 19200 samples; `ffprobe` -> flac/s32/48000/2ch; `sha256sum`; `./target/debug/deps/flac_bitexact-0363eae80343785c --nocapture` -> 4 passed / 0 failed, 0.21s |

## 1. Verdict
Real 24-bit FLAC fixture, one of four lanes in the FLAC byte-exact suite. The suite that consumes it (`fixtures_decode_byte_exact_against_the_ffmpeg_reference`, flac_bitexact.rs:50) decodes each fixture with the in-house FLAC decoder and compares the output bytes against an ffmpeg-derived reference — it ran green when I executed it, so this asset is live differential evidence for the high-bit-depth (s24) decode path, not decoration. Also referenced by flac_fuzz_survival.rs and flac_vlc_differential.rs (built binaries present).

## 2. Test usage (verified by execution)
`./target/debug/deps/flac_bitexact-0363eae80343785c --nocapture` -> **4/4 pass** including `fixtures_decode_byte_exact_against_the_ffmpeg_reference` (this fixture is FIXTURES[2], flac_bitexact.rs:45) and `seektable_carrier_exposes_six_verified_points`. The corpus lane (`corpus_flacs_decode_byte_exact_or_reject_typed`) skipped loudly (corpus absent) — does not affect this in-repo fixture's verification.

## 3. Matrix evidence
The FLAC decoder row's "24-bit" coverage claim rests on exactly this fixture (the other three committed lanes are s16). ffprobe independently confirms the 24-bit/48k/stereo facts, so the lane's premise is genuine.
