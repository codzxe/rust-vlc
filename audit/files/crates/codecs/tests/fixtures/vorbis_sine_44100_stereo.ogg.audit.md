# AUDIT: crates/codecs/tests/fixtures/vorbis_sine_44100_stereo.ogg (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | Ogg Vorbis stereo 44100 Hz ~128 kbps (`file` + ffprobe concur) / 8422 bytes / 5e9ce549692bdb67dffac15ee095716ffdef9824461582ac1269daa9b0203170 |
| File verdict | REAL (live-ffmpeg differential evidence, executed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `ffprobe` -> vorbis/44100/2ch; `sha256sum`; `./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437 --nocapture` -> 5 passed / 0 failed, 0.82s |

## 1. Verdict
Real Ogg Vorbis fixture, lane 1 of 4 in the Vorbis bit-exact suite. The consuming test (`fixtures_are_pinned_against_the_ffmpeg_reference`, vorbis_bitexact.rs:31-36) runs a LIVE ffmpeg differential (ffmpeg present on PATH): it decoded 66 150 samples × 2ch through the in-house decoder and compared against ffmpeg's f32le dump with a recorded float tolerance. Executed row output: `DIFF-ROW vorbis_sine_44100_stereo.ogg: samples=66150 ch=2 rate=44100 lumen_window_md5=658af0c3… ffmpeg_md5_f32le=7e36fc4d… max_delta=3.7e-8 (bound 1e-7)` — PASS. Also feeds vorbis_vlc_differential / fuzz_survival / bench suites (binaries present).

## 2. Test usage (verified by execution)
`./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437 --nocapture` -> **5/5 pass** including the live-oracle differential for this fixture (max_delta 3.7e-8 vs bound 1e-7), the silence lane (delta exactly 0, md5s identical), the comments/tag lane (this fixture asserted TITLE-less: bitexact.rs:194-195), and the encoder-delay trim lane. The corpus row skips loudly by design ("absent corpus is a SKIP with a message, never a silent pass") — does not affect this in-repo fixture.

## 3. Matrix evidence
The vorbis decoder row's evidence chain includes this fixture; the differential is live (not a committed constant), the strongest form in this repo alongside the h264/adpcm lanes. No inflation for this asset.
