# Lumen randomized-corpus report — 2026-10-22 15:57

- corpus seed: **63117490995983871645529930018** (non-deterministic run; replay with `gen_corpus.py --replay corpus/manifest.json`)
- files: **310** (1095 MiB clean), rejected draws: 1
- composition: audio-only=38, avi-av=39, avi-video=87, dirty=42, dirty-lie-ext=8, flv-video=6, garbage=11, mkv-video=21, mov-video=4, mp4-video=20, muxed-av=19, ogv-video=2, ts-video=8, webm-video=5
- mode: **--skip-suites** — the Rust suites were NOT driven (oracle-side validation run): the suite sections below carry no rows by construction, and every Lumen-side decoded-format cell is a suite-gap row
- suite exit codes: probe=skipped demux=skipped mjpeg=skipped pcm=skipped (skipped — not driven)

## Probe (bounded format detection) vs ffprobe oracle

_suite skipped (--skip-suites): no rows; section intentionally empty_

| metric | value |
|---|---|
| AVI identification (ffprobe says avi) | **0/0** |
| MP4 identification (ffprobe says mov/mp4) | **0/0** |
| garbage rejection | **0/0** (must be 100%) |
| false positives (probed a non-AVI/MP4/garbage) | 0 |

## AVI demux walk vs ffprobe oracle (supported fourccs: mjpeg + pcm)

_suite skipped (--skip-suites): no rows; section intentionally empty_

| metric | value |
|---|---|
| supported clean AVIs walked to EOS | **0/0** |
| stream count matches oracle | 0/0 |
| video packet count matches oracle | **0/0** |
| audio packet count matches oracle | 0/0 |
| dirty AVIs reached a typed terminal (no panic, no hang) | **0/0** |
| dirty AVIs that recovered damage mid-walk | 0 |
| unsupported-fourcc AVIs answered with a typed refusal | 0/0 (no panic on any) |

_Duration metric (W2-08, W1-03's prescription): span = last video PTS − first video PTS + one frame period, period = 1e9·strh.scale/strh.rate parsed from the file's own first `vids` strh — the oracle's duration includes the last frame's display period, so the previous span-only form under-reported by exactly one frame period (−40.0/−16.7/−8.3 ms p50/p95/max on this corpus's 2026-10-22 13:33 run)._

## MJPEG decode vs ffmpeg oracle (real throughput + fidelity)

_suite skipped (--skip-suites): no rows; section intentionally empty_

| resolution | kind | files | frames | decode errors | concealed | PSNR min (dB) | PSNR mean (dB) | max abs | exact bytes | throughput | demux terminal | demux rec |
|---|---|---|---|---|---|---|---|---|---|---|---|---|

_PSNR across independent JPEG IDCT implementations is expected well above 45 dB; byte-exactness is not expected cross-implementation. `exact bytes` is the bit-equal fraction vs ffmpeg's decode._
_W2-08 (W1-01's prescription): `kind` splits clean corpus classes from dirty siblings (manifest-class join) — dirty concealment can no longer pollute clean-class PSNR minima. Fidelity columns (PSNR min/mean, max abs, exact bytes) exclude files with any concealed frame; per-frame exclusion needs suite-emitted clean-only PSNR columns (registered follow-up). `demux terminal`/`demux rec` come from the demux walk of the same file, so packet-count deltas are attributable._

## PCM decode vs ffmpeg oracle (byte-exact expected)

_suite skipped (--skip-suites): no rows; section intentionally empty_

| metric | value |
|---|---|
| comparable PCM cases (audio stream demuxed) | 0/0 (0 not demuxable — see demux section) |
| comparable cases byte-exact vs ffmpeg | **0/0** |
| cases with byte mismatches | 0 (any mismatch is a defect signal) |
| cases with shorter demuxed payload than oracle (len notes) | 0 |

## Decoded format fidelity vs oracle (decoded output format + dimensions)

Per-file ledger: `corpus/_harness/decoded-rows.tsv` — oracle columns from ffprobe `-show_streams` (video pix_fmt/width/height; audio sample_fmt/channels/sample_rate), Lumen columns from the corpus suites' decoded-output rows. Match-rate scope: the supported-format subset, AVI files whose fourccs Lumen decodes (mjpeg video + pcm audio), clean and dirty — the same population as the demux section's supported set.

### Oracle-side decoded formats (measured)

| metric | value |
|---|---|
| mjpeg AVIs with oracle dims (w×h) | 68/69 |
| oracle dims equal generator intent | 68/68 |
| oracle pix_fmt families (mjpeg AVIs) | yuv420=46, yuv444=22 |
| oracle pix_fmt family equal generator intent | 29/39 (intent recorded for 39/68 — the av draw path persists no pix_fmt) |
| pcm AVIs with oracle sample_fmt | 45/47 |
| oracle sample families (pcm AVIs) | u8=13, f32=13, s24=10, s16=9 |
| oracle channel layouts (pcm AVIs) | 2=18, 1=10, 8=9, 6=6, 4=1, 3=1 |
| oracle sample rates (pcm AVIs) | 8000=9, 16000=8, 88200=7, 11025=5, 22050=4, 44100=4, 48000=3, 32000=3, 96000=1, 176400=1 |
| oracle channels+rate equal generator intent | 45/45 |
| mjpeg/pcm streams outside AVI (mov/mkv/wav…) | 4 video / 12 audio (out of scope until their demuxers land — those rows carry no-path markers) |

### Lumen vs oracle match rates

| metric | match | population |
|---|---|---|
| video dims match | N/A — not-emitted (suite gap) (0/68 comparable) | 68 mjpeg AVIs with oracle dims |
| pix_fmt family match | N/A — not-emitted (suite gap) (0/68 comparable) | 68 with oracle pix_fmt |
| audio sample_fmt match | N/A — not-emitted (suite gap) (0/45 comparable) | 45 pcm AVIs with oracle sample_fmt |
| channels+rate match | N/A — not-emitted (suite gap) (0/45 comparable) | 45 |

Lumen-side decoded format/dims: **not-emitted (suite gap)** — the mjpeg/pcm corpus suites measure counts/PSNR/bytes but do not write the decoded `DecodedFrame`'s format (`DecodedFormat::I420`/`Rgba8`) or width/height, nor the PCM output format, into their result rows. Registered follow-up (suite emission): extend `mjpeg-results.tsv` with `out_format`/`out_w`/`out_h` (first decoded frame's DecodedFormat + dims) and `pcm-results.tsv` with `out_desc`/`out_rate`/`out_ch`; this harness already consumes those columns when present — no further driver change needed.

### Mismatch listing (file, expected, got)

- media/0000-avi-video.avi [avi-video]: expected 854x480 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0007-avi-video.avi [avi-video]: expected 239x180 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0010-avi-video.avi [avi-video]: expected 16x16 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0015-avi-video.avi [avi-video]: expected 16x16 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0016-avi-video.avi [avi-video]: expected 854x480 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0019-avi-av.avi [avi-av]: expected 120x90 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0020-avi-video.avi [avi-video]: expected 720x480 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0021-avi-av.avi [avi-av]: expected 800x599 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0025-avi-video.avi [avi-video]: expected 176x144 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0028-avi-av.avi [avi-av]: expected 480x360 yuvj420p (family yuv420), got not-emitted (suite gap)
- … and 58 more video rows (full ledger: decoded-rows.tsv)
- media/0012-avi-av.avi [avi-av]: expected u8 (family u8) 11025Hz x1, got not-emitted (suite gap)
- media/0019-avi-av.avi [avi-av]: expected s32 (family s24) 16000Hz x2, got not-emitted (suite gap)
- media/0021-avi-av.avi [avi-av]: expected u8 (family u8) 48000Hz x2, got not-emitted (suite gap)
- media/0024-avi-av.avi [avi-av]: expected s16 (family s16) 8000Hz x2, got not-emitted (suite gap)
- media/0028-avi-av.avi [avi-av]: expected flt (family f32) 22050Hz x1, got not-emitted (suite gap)
- media/0031-avi-av.avi [avi-av]: expected u8 (family u8) 8000Hz x6, got not-emitted (suite gap)
- media/0044-avi-av.avi [avi-av]: expected s32 (family s24) 88200Hz x2, got not-emitted (suite gap)
- media/0050-avi-av.avi [avi-av]: expected u8 (family u8) 32000Hz x2, got not-emitted (suite gap)
- media/0053-avi-av.avi [avi-av]: expected s32 (family s24) 8000Hz x8, got not-emitted (suite gap)
- media/0062-avi-av.avi [avi-av]: expected u8 (family u8) 11025Hz x2, got not-emitted (suite gap)
- … and 35 more audio rows (full ledger: decoded-rows.tsv)

#### Generator-intent vs oracle drift (real mismatches, measurable today)

10 file(s) where the oracle's decoded facts disagree with the generator's declared parameters — dirty-class corruption is one legitimate source; encoder-side format drift (a gray-intent draw encoded as yuvj444p) is another:
- media/0032-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0114-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0141-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0171-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0214-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0229-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0249-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0250-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- … and 2 more

## Suite health

- probe suite rc=skipped, demux rc=skipped, mjpeg rc=skipped, pcm rc=skipped — SUITES SKIPPED (--skip-suites): oracle-side validation run, not a measurement run
- harness wall: 36s
- decoded-rows ledger: `/home/z/my-project/corpus-big/_harness/decoded-rows.tsv` (310 rows, 348 suite-gap cells) — registered follow-up: suites emit `out_format`/`out_w`/`out_h` (mjpeg) and `out_desc`/`out_rate`/`out_ch` (pcm)
