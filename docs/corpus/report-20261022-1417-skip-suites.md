# Lumen randomized-corpus report — 2026-10-22 14:17

- corpus seed: **6033317083842721047527680438** (non-deterministic run; replay with `gen_corpus.py --replay corpus/manifest.json`)
- files: **486** (1514 MiB clean), rejected draws: 2
- composition: audio-only=63, avi-av=57, avi-video=134, dirty=62, dirty-lie-ext=16, flv-video=13, garbage=11, mkv-video=31, mov-video=6, mp4-video=33, muxed-av=26, ogv-video=4, ts-video=16, webm-video=14
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

## MJPEG decode vs ffmpeg oracle (real throughput + fidelity)

_suite skipped (--skip-suites): no rows; section intentionally empty_

| resolution | files | frames | decode errors | concealed | PSNR min (dB) | PSNR mean (dB) | max abs | exact bytes | throughput |
|---|---|---|---|---|---|---|---|---|---|

_PSNR across independent JPEG IDCT implementations is expected well above 45 dB; byte-exactness is not expected cross-implementation. `exact bytes` is the bit-equal fraction vs ffmpeg's decode._

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
| mjpeg AVIs with oracle dims (w×h) | 99/99 |
| oracle dims equal generator intent | 99/99 |
| oracle pix_fmt families (mjpeg AVIs) | yuv420=63, yuv444=36 |
| oracle pix_fmt family equal generator intent | 36/63 (intent recorded for 63/99 — the av draw path persists no pix_fmt) |
| pcm AVIs with oracle sample_fmt | 65/67 |
| oracle sample families (pcm AVIs) | s16=19, u8=19, s24=14, f32=13 |
| oracle channel layouts (pcm AVIs) | 2=16, 1=14, 6=13, 4=12, 8=6, 3=4 |
| oracle sample rates (pcm AVIs) | 11025=10, 32000=10, 16000=9, 44100=7, 88200=6, 96000=6, 22050=6, 48000=5, 8000=4, 44056=1, 37800=1 |
| oracle channels+rate equal generator intent | 65/65 |
| mjpeg/pcm streams outside AVI (mov/mkv/wav…) | 4 video / 28 audio (out of scope until their demuxers land — those rows carry no-path markers) |

### Lumen vs oracle match rates

| metric | match | population |
|---|---|---|
| video dims match | N/A — not-emitted (suite gap) (0/99 comparable) | 99 mjpeg AVIs with oracle dims |
| pix_fmt family match | N/A — not-emitted (suite gap) (0/99 comparable) | 99 with oracle pix_fmt |
| audio sample_fmt match | N/A — not-emitted (suite gap) (0/65 comparable) | 65 pcm AVIs with oracle sample_fmt |
| channels+rate match | N/A — not-emitted (suite gap) (0/65 comparable) | 65 |

Lumen-side decoded format/dims: **not-emitted (suite gap)** — the mjpeg/pcm corpus suites measure counts/PSNR/bytes but do not write the decoded `DecodedFrame`'s format (`DecodedFormat::I420`/`Rgba8`) or width/height, nor the PCM output format, into their result rows. Registered follow-up (suite emission): extend `mjpeg-results.tsv` with `out_format`/`out_w`/`out_h` (first decoded frame's DecodedFormat + dims) and `pcm-results.tsv` with `out_desc`/`out_rate`/`out_ch`; this harness already consumes those columns when present — no further driver change needed.

### Mismatch listing (file, expected, got)

- media/0000-avi-video.avi [avi-video]: expected 720x576 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0003-avi-av.avi [avi-av]: expected 720x575 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0007-avi-video.avi [avi-video]: expected 176x144 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0015-avi-video.avi [avi-video]: expected 320x240 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0025-avi-av.avi [avi-av]: expected 720x576 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0029-avi-video.avi [avi-video]: expected 1280x720 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0031-avi-video.avi [avi-video]: expected 32x32 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0036-avi-video.avi [avi-video]: expected 640x480 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0041-avi-video.avi [avi-video]: expected 720x576 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0060-avi-video.avi [avi-video]: expected 16x16 yuvj420p (family yuv420), got not-emitted (suite gap)
- … and 89 more video rows (full ledger: decoded-rows.tsv)
- media/0003-avi-av.avi [avi-av]: expected s32 (family s24) 11025Hz x8, got not-emitted (suite gap)
- media/0021-avi-av.avi [avi-av]: expected s16 (family s16) 88200Hz x1, got not-emitted (suite gap)
- media/0025-avi-av.avi [avi-av]: expected s16 (family s16) 88200Hz x8, got not-emitted (suite gap)
- media/0027-avi-av.avi [avi-av]: expected s16 (family s16) 88200Hz x1, got not-emitted (suite gap)
- media/0035-avi-av.avi [avi-av]: expected u8 (family u8) 48000Hz x4, got not-emitted (suite gap)
- media/0061-avi-av.avi [avi-av]: expected s16 (family s16) 8000Hz x1, got not-emitted (suite gap)
- media/0073-avi-av.avi [avi-av]: expected s16 (family s16) 96000Hz x3, got not-emitted (suite gap)
- media/0074-avi-av.avi [avi-av]: expected s16 (family s16) 96000Hz x2, got not-emitted (suite gap)
- media/0081-avi-av.avi [avi-av]: expected u8 (family u8) 32000Hz x6, got not-emitted (suite gap)
- media/0098-avi-av.avi [avi-av]: expected u8 (family u8) 16000Hz x1, got not-emitted (suite gap)
- … and 55 more audio rows (full ledger: decoded-rows.tsv)

#### Generator-intent vs oracle drift (real mismatches, measurable today)

27 file(s) where the oracle's decoded facts disagree with the generator's declared parameters — dirty-class corruption is one legitimate source; encoder-side format drift (a gray-intent draw encoded as yuvj444p) is another:
- media/0000-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0015-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0031-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0036-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0064-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0070-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0117-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0177-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- … and 19 more

## Suite health

- probe suite rc=skipped, demux rc=skipped, mjpeg rc=skipped, pcm rc=skipped — SUITES SKIPPED (--skip-suites): oracle-side validation run, not a measurement run
- harness wall: 168s
- decoded-rows ledger: `/home/z/my-project/corpus/_harness/decoded-rows.tsv` (486 rows, 498 suite-gap cells) — registered follow-up: suites emit `out_format`/`out_w`/`out_h` (mjpeg) and `out_desc`/`out_rate`/`out_ch` (pcm)
