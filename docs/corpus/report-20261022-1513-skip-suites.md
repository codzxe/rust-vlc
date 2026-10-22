# Lumen randomized-corpus report — 2026-10-22 15:13

- corpus seed: **6033317083842721047527680438** (non-deterministic run; replay with `gen_corpus.py --replay corpus/manifest.json`)
- files: **486** (1514 MiB clean), rejected draws: 2
- composition: audio-only=63, avi-av=57, avi-video=134, dirty=62, dirty-lie-ext=16, flv-video=13, garbage=11, mkv-video=31, mov-video=6, mp4-video=33, muxed-av=26, ogv-video=4, ts-video=16, webm-video=14
- mode: **--skip-suites (persisted-results reuse)** — the Rust suites were NOT driven (no cargo, no build lock): the suite rows below are the persisted results of the last full run, re-aggregated here to validate this judge's logic — NOT a fresh measurement
- suite exit codes: probe=reused demux=reused mjpeg=reused pcm=reused (reused from the persisted last-run TSVs — not driven this run)

## Probe (bounded format detection) vs ffprobe oracle

_suites not driven (--skip-suites): the rows below are the persisted results of the last full run — aggregation validation, not a fresh measurement_

| metric | value |
|---|---|
| AVI identification (ffprobe says avi) | **224/224** |
| MP4 identification (ffprobe says mov/mp4) | **66/66** |
| garbage rejection | **10/11** (must be 100%) |
| false positives (probed a non-AVI/MP4/garbage) | 1 |
  - dirty/cut-mid-chunk-015-0011-mp4-video.mp4 probed=mp4 truth=- class=dirty
| probe latency p50/p95/max | 409µs / 684µs / 1210µs |

## AVI demux walk vs ffprobe oracle (supported fourccs: mjpeg + pcm)

_suites not driven (--skip-suites): the rows below are the persisted results of the last full run — aggregation validation, not a fresh measurement_

| metric | value |
|---|---|
| supported clean AVIs walked to EOS | **85/85** |
| stream count matches oracle | 66/85 |
| video packet count matches oracle | **85/85** |
| audio packet count matches oracle | 12/31 |
| v-packet delta vs oracle p50/p95/max | +0 / +0 / +0 packets |
| duration error (PTS span + frame period) p50/p95/max | 0.0 / 0.0 / 0.0 ms |
| dirty AVIs reached a typed terminal (no panic, no hang) | **35/35** |
| dirty AVIs that recovered damage mid-walk | 14 |
| unsupported-fourcc AVIs answered with a typed refusal | 121/127 (no panic on any) |

_Duration metric (W2-08, W1-03's prescription): span = last video PTS − first video PTS + one frame period, period = 1e9·strh.scale/strh.rate parsed from the file's own first `vids` strh — the oracle's duration includes the last frame's display period, so the previous span-only form under-reported by exactly one frame period (−40.0/−16.7/−8.3 ms p50/p95/max on this corpus's 2026-10-22 13:33 run)._

## MJPEG decode vs ffmpeg oracle (real throughput + fidelity)

_suites not driven (--skip-suites): the rows below are the persisted results of the last full run — aggregation validation, not a fresh measurement_

| resolution | kind | files | frames | decode errors | concealed | PSNR min (dB) | PSNR mean (dB) | max abs | exact bytes | throughput | demux terminal | demux rec |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny16 | clean | 6 | 2418 | 0 | 0 | 65.5 | 128.0 | 1 | 99.30% | 77.9 MB/s, 200498 fps | eos:6 | 10 |
| tiny32 | clean | 4 | 1707 | 0 | 0 | 66.8 | 195.6 | 1 | 99.59% | 32.3 MB/s, 52526 fps | eos:4 | 6 |
| tiny32 | dirty | 1 | 68 | 0 | 0 | 0.0 | 0.0 | 0 | - | 23.3 MB/s, 53628 fps | eos:1 | 3 |
| tiny32odd | clean | 2 | 734 | 0 | 0 | 0.0 | 0.0 | 0 | - | 24.6 MB/s, 24621 fps | eos:2 | 2 |
| tiny64odd | clean | 2 | 442 | 0 | 0 | 68.7 | 73.5 | 1 | 99.68% | 36.9 MB/s, 25085 fps | eos:2 | 2 |
| tiny64odd | dirty | 1 | 78 | 0 | 1 | 0.0 | 0.0 | 0 | - | 23.4 MB/s, 26298 fps | eos:1 | 4 |
| tiny64 | clean | 3 | 464 | 0 | 0 | 67.4 | 69.6 | 1 | 99.26% | 13.3 MB/s, 17991 fps | eos:3 | 3 |
| tiny64 | dirty | 1 | 73 | 0 | 1 | 0.0 | 0.0 | 0 | - | 12.1 MB/s, 13372 fps | eos:1 | 4 |
| qqvga | clean | 6 | 1597 | 0 | 0 | 65.1 | 193.8 | 1 | 99.73% | 22.4 MB/s, 7535 fps | eos:6 | 8 |
| qqvga | dirty | 3 | 534 | 0 | 0 | 71.3 | 477.2 | 1 | 99.93% | 15.9 MB/s, 36277 fps | eos:1 start-error:2 | 264 |
| qvga | clean | 5 | 722 | 0 | 0 | 66.3 | 96.7 | 1 | 99.63% | 21.5 MB/s, 12850 fps | eos:5 | 7 |
| qvga | dirty | 1 | 41 | 0 | 1 | 0.0 | 0.0 | 0 | - | 12.6 MB/s, 17322 fps | eos:1 | 4 |
| qcif | clean | 6 | 1537 | 0 | 0 | 69.2 | 72.8 | 1 | 99.53% | 34.9 MB/s, 2925 fps | eos:6 | 8 |
| wqvgaodd | clean | 1 | 110 | 0 | 0 | 76.6 | 76.6 | 1 | 99.86% | 11.6 MB/s, 5597 fps | eos:1 | 3 |
| wqvga | clean | 4 | 713 | 0 | 0 | 68.7 | 71.1 | 1 | 99.51% | 21.1 MB/s, 1267 fps | eos:4 | 6 |
| vgaodd | clean | 1 | 84 | 0 | 0 | 82.6 | 82.6 | 1 | 99.96% | 8.6 MB/s, 4651 fps | eos:1 | 1 |
| vga | clean | 7 | 1189 | 0 | 0 | 69.4 | 72.5 | 1 | 99.70% | 15.8 MB/s, 1053 fps | eos:7 | 9 |
| vga | dirty | 1 | 202 | 0 | 0 | 0.0 | 0.0 | 0 | - | 2.5 MB/s, 1621 fps | eos:1 | 1 |
| cifodd | clean | 2 | 772 | 0 | 0 | 67.5 | 68.9 | 1 | 99.15% | 27.2 MB/s, 973 fps | eos:2 | 4 |
| w360podd | clean | 1 | 122 | 0 | 0 | 68.5 | 68.7 | 1 | 99.12% | 29.8 MB/s, 392 fps | eos:1 | 1 |
| w360p | clean | 1 | 522 | 0 | 0 | 0.0 | 0.0 | 0 | - | 2.2 MB/s, 768 fps | eos:1 | 3 |
| hvgaodd | clean | 2 | 437 | 0 | 0 | 1000.0 | 1000.0 | 0 | 100.00% | 15.0 MB/s, 357 fps | eos:2 | 4 |
| vga4x3odd | clean | 1 | 597 | 0 | 0 | 0.0 | 0.0 | 0 | - | 4.7 MB/s, 375 fps | eos:1 | 1 |
| vga4x3 | clean | 2 | 501 | 0 | 0 | 75.5 | 76.5 | 1 | 99.85% | 4.9 MB/s, 576 fps | eos:2 | 2 |
| vga4x3 | dirty | 1 | 191 | 0 | 1 | 0.0 | 0.0 | 0 | - | 2.4 MB/s, 424 fps | eos:1 | 4 |
| ntsc | clean | 4 | 750 | 0 | 0 | 69.0 | 305.8 | 1 | 99.73% | 12.8 MB/s, 1022 fps | eos:4 | 8 |
| palodd | clean | 2 | 360 | 0 | 0 | 69.0 | 71.3 | 1 | 99.49% | 41.1 MB/s, 151 fps | eos:2 | 6 |
| pal | clean | 3 | 484 | 0 | 0 | 69.1 | 536.3 | 1 | 99.82% | 17.6 MB/s, 257 fps | eos:3 | 5 |
| svga | clean | 3 | 366 | 0 | 0 | 68.9 | 380.3 | 1 | 99.58% | 30.2 MB/s, 261 fps | eos:3 | 3 |
| qhdp | clean | 2 | 198 | 0 | 0 | 69.7 | 79.0 | 1 | 99.70% | 18.6 MB/s, 387 fps | eos:2 | 2 |
| qhdp | dirty | 1 | 39 | 0 | 0 | 87.6 | 87.6 | 1 | 99.99% | 10.7 MB/s, 1587 fps | eos:1 | 4 |
| xga | clean | 2 | 154 | 0 | 0 | 69.1 | 71.3 | 1 | 99.50% | 28.8 MB/s, 91 fps | eos:2 | 2 |
| hd720odd | clean | 1 | 32 | 0 | 0 | 69.1 | 72.2 | 1 | 99.60% | 25.7 MB/s, 70 fps | eos:1 | 1 |
| hd720 | clean | 6 | 695 | 0 | 0 | 69.1 | 76.6 | 1 | 99.72% | 34.1 MB/s, 109 fps | eos:6 | 10 |
| hd720 | dirty | 2 | 147 | 0 | 1 | 70.2 | 70.3 | 1 | 99.40% | 38.1 MB/s, 75 fps | eos:2 | 7 |
| sxga | clean | 1 | 65 | 0 | 0 | 0.0 | 0.0 | 0 | - | 11.9 MB/s, 42 fps | eos:1 | 1 |
| sxga | dirty | 1 | 65 | 0 | 0 | 0.0 | 0.0 | 0 | - | 11.8 MB/s, 42 fps | eos:1 | 1 |
| fhd | clean | 3 | 379 | 0 | 0 | 1000.0 | 1000.0 | 0 | 100.00% | 1.7 MB/s, 63 fps | eos:3 | 3 |
| uhd4k | clean | 2 | 92 | 0 | 0 | 1000.0 | 1000.0 | 0 | 100.00% | 22.2 MB/s, 12 fps | eos:2 | 2 |
| uhd4k | dirty | 1 | 48 | 0 | 0 | 0.0 | 0.0 | 0 | - | 23.2 MB/s, 6 fps | eos:1 | 1 |

_PSNR across independent JPEG IDCT implementations is expected well above 45 dB; byte-exactness is not expected cross-implementation. `exact bytes` is the bit-equal fraction vs ffmpeg's decode._
_W2-08 (W1-01's prescription): `kind` splits clean corpus classes from dirty siblings (manifest-class join) — dirty concealment can no longer pollute clean-class PSNR minima. Fidelity columns (PSNR min/mean, max abs, exact bytes) exclude files with any concealed frame; per-frame exclusion needs suite-emitted clean-only PSNR columns (registered follow-up). `demux terminal`/`demux rec` come from the demux walk of the same file, so packet-count deltas are attributable._
Per-case attribution ledger (decode ⋈ demux ⋈ oracle packets): `/home/z/my-project/corpus/_harness/mjpeg-verdicts.tsv` (99 rows).

## PCM decode vs ffmpeg oracle (byte-exact expected)

_suites not driven (--skip-suites): the rows below are the persisted results of the last full run — aggregation validation, not a fresh measurement_

| metric | value |
|---|---|
| comparable PCM cases (audio stream demuxed) | 41/65 (24 not demuxable — see demux section) |
| comparable cases byte-exact vs ffmpeg | **18/41** |
| cases with byte mismatches | 0 (any mismatch is a defect signal) |
| cases with shorter demuxed payload than oracle (len notes) | 23 |

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

- probe suite rc=reused, demux rc=reused, mjpeg rc=reused, pcm rc=reused — SUITES NOT DRIVEN (--skip-suites): judgement re-aggregated from the persisted last-run TSVs (probe-results.tsv, demux-results.tsv, mjpeg-results.tsv, pcm-results.tsv) — not a fresh measurement
- harness wall: 0s
- decoded-rows ledger: `/home/z/my-project/corpus/_harness/decoded-rows.tsv` (486 rows, 498 suite-gap cells) — registered follow-up: suites emit `out_format`/`out_w`/`out_h` (mjpeg) and `out_desc`/`out_rate`/`out_ch` (pcm)
- mjpeg verdicts ledger (per case: decode ⋈ demux terminal/recovered ⋈ oracle packets): `/home/z/my-project/corpus/_harness/mjpeg-verdicts.tsv` (99 rows)
