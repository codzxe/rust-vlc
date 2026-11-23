# AUDIT: crates/codecs/tests/fixtures/vorbis_silence_48000_stereo.ogg (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A05 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Type / size / sha256 | Ogg Vorbis, stereo, 48000 Hz ("~128000 bps" per `file`) / 4,431 bytes / ef6d6270b0ce7edac026fc92db3434065b6d5369041c977cae3d4e467bd10ef4 (md5 472ef48c2dde22fe31d35187af2f2606) |
| File verdict | REAL (every filename claim verified, including the silence content itself: max|sample| = 0.0 across the entire decode) |
| Reachable from shipped entry point? | N/A (test fixture; Ogg/Vorbis is a shipped open path — pipeline_host vorbis arm) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `stat`; `sha256sum`/`md5sum`; `ffprobe` (vorbis, 48000, 2ch, 1.000s); python3 Ogg page walk (3 pages, id+comment+setup headers present); ffmpeg f32le decode piped to python -> 95,744 samples, max abs 0.0, 0 nonzero; `vorbis_bitexact-e8223d5cf8b5f437` -> 5/5 pass; `vorbis_fuzz_survival-d8fc9b5caa4a5ec1` -> 1/1 pass; `vorbis_vlc_differential-3a9245a4339ccea0` -> 1/1 pass (pending-infra branch, no VLC in sandbox, honestly named) |

## 1. One-paragraph verdict
A genuine, truthfully-named fixture: `file` and ffprobe confirm Ogg Vorbis
stereo 48 kHz (duration 1.000s), and I proved the "silence" claim at the
sample level — ffmpeg's full decode is 95,744 f32 samples with max
absolute value exactly 0.0 and zero nonzero samples. The Ogg structure is
the documented single-data-page shape (3 pages: headers + one data page +
EOS), which is precisely why the bitexact suite uses it as the
flush/EOS-window boundary lane (vorbis_bitexact.rs:387-400: asserts the
all-Missing pts series and emission == the raw pin — "the pinned honest
shape"). The fixture is consumed by three suites, two of which I executed
live: vorbis_bitexact 5/5 (the silence lane is one of the 4
ffmpeg-pinned fixtures; per the authored record the lane is BIT-EXACT —
worklog archive-2026-10.md:8857, lumen MD5 == ffmpeg MD5
9f3a19b1740ac999088756035046c146, 0 delta) and vorbis_fuzz_survival 1/1
(the silence bytes are among the mutated/truncated inputs). The
vlc_differential test passed its honestly-named pending-infra branch (no
VLC binary in this sandbox; the assert degenerates to compared==0 by
design, vorbis_vlc_differential.rs:244). Format-doc evidence citation
verified: docs/formats/audio-vorbis.md:45 lists this file as
"2/48000 | 0 | **BIT-EXACT**".

## 2. What actually works (verified)
- ffmpeg f32 decode -> 95,744 samples, max abs 0.0 (silence proven, not
  assumed from the name).
- `./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437` -> 5 passed /
  0 failed (fixtures pinned vs the ffmpeg reference, incl. this lane).
- `./target/debug/deps/vorbis_fuzz_survival-d8fc9b5caa4a5ec1` -> 1
  passed / 0 failed (silence bytes fuzzed; no panics).

## 3. Findings
None. Codec, channel count, rate, silence content, page structure, and
the recorded bit-exact lane all match; the fixture is live-exercised.

## 4-9. (binary asset — short-form)
- Cited as evidence by: vorbis_bitexact.rs:34/:387 (flush-boundary +
  ffmpeg-reference pin), vorbis_fuzz_survival.rs:44 (mutation input),
  vorbis_vlc_differential.rs:131 (VLC oracle row, pending-infra here),
  docs/formats/audio-vorbis.md:45 (BIT-EXACT row), worklog:8857 (authored
  differential record). Honest use throughout.
- Untouched on disk; no in-tree byte digest of the container itself (the
  reference data pins decode outputs, the repo's standard posture).
