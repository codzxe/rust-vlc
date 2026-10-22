# Lumen randomized-corpus report — 2026-10-22 20:02

- corpus seed: **58007138736976908607880252061** (non-deterministic run; replay with `gen_corpus.py --replay corpus/manifest.json`)
- files: **485** (1292 MiB clean), rejected draws: 19
- composition: audio-only=56, avi-av=43, avi-video=82, cover-audio=10, dirty=55, dirty-lie-ext=7, drift-av=13, dynres-video=12, flv-video=8, fmp4-video=7, garbage=22, hls-av=6, meta-bomb-av=7, mkv-video=30, mov-video=4, mp4-video=18, multi-av=5, multivideo-av=15, muxed-av=26, ogv-video=3, raw-video=16, ts-video=8, vfr-av=23, webm-video=9
- mode: **--skip-suites** — the Rust suites were NOT driven (oracle-side validation run): the suite sections below carry no rows by construction, and every Lumen-side decoded-format cell is a suite-gap row
- suite exit codes: probe=skipped demux=skipped mjpeg=skipped pcm=skipped (skipped — not driven)

## Per-class coverage (v3 class vocabulary, manifest join)

_Generator-intent coverage of the v3 corpus classes — a manifest-side join, no new probing. `files` counts manifest entries per class; `ffprobe_ok` is the generator's own per-file ffprobe validation (v3 manifests — a `false` there is a real defect signal: the generator authored a file its own oracle rejects; `-` where the manifest records none: dirty/garbage classes and pre-v3 manifests); `mean bytes` averages the manifest's per-entry byte totals (`-` when absent). Vocabulary classes that drew zero files this run are listed as zero rows — an absent surface stays visible instead of silently missing._

| class | files | ffprobe_ok | mean bytes |
|---|---|---|---|
| audio-only | 56 | 56/56 (100%) | 1,255,737 |
| avi-av | 43 | 43/43 (100%) | 6,264,715 |
| avi-video | 82 | 82/82 (100%) | 8,120,238 |
| cover-audio | 10 | 10/10 (100%) | 37,966 |
| dirty | 55 | - | 5,411,814 |
| dirty-lie-ext | 7 | - | 262,956 |
| drift-av | 13 | 13/13 (100%) | 5,375,804 |
| dynres-video | 12 | 12/12 (100%) | 1,337,364 |
| flv-video | 8 | 8/8 (100%) | 2,392,402 |
| fmp4-video | 7 | 7/7 (100%) | 149,239 |
| garbage | 22 | - | 4,585 |
| hls-av | 6 | 6/6 (100%) | 570,822 |
| meta-bomb-av | 7 | 7/7 (100%) | 236,528 |
| mkv-video | 30 | 30/30 (100%) | 973,284 |
| mov-video | 4 | 4/4 (100%) | 3,369,880 |
| mp4-video | 18 | 18/18 (100%) | 2,057,199 |
| multi-av | 5 | 5/5 (100%) | 5,475,440 |
| multivideo-av | 15 | 15/15 (100%) | 2,007,789 |
| muxed-av | 26 | 26/26 (100%) | 771,895 |
| ogv-video | 3 | 3/3 (100%) | 71,095 |
| raw-video | 16 | 16/16 (100%) | 1,307,464 |
| ts-video | 8 | 8/8 (100%) | 776,581 |
| vfr-av | 23 | 23/23 (100%) | 1,679,646 |
| webm-video | 9 | 9/9 (100%) | 1,604,669 |

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
| audio packet count matches oracle (chunk-granular, W3-16) | **0/0** |
| audio packet count vs oracle raw packets (informational; granularity differs) | 0/0 coincide |
| audio payload bytes match oracle | **0/0** |
| dirty AVIs reached a typed terminal (no panic, no hang) | **0/0** |
| dirty AVIs that recovered damage mid-walk | 0 |
| unsupported-fourcc AVIs answered with a typed refusal | 0/0 (no panic on any) |

_Duration metric (W2-08, W1-03's prescription): span = last video PTS − first video PTS + one frame period, period = 1e9·strh.scale/strh.rate parsed from the file's own first `vids` strh — the oracle's duration includes the last frame's display period, so the previous span-only form under-reported by exactly one frame period (−40.0/−16.7/−8.3 ms p50/p95/max on this corpus's 2026-10-22 13:33 run)._

_Audio packet count (W3-16 prescription): ffprobe's AVI demuxer re-packetizes PCM chunks into pieces of block_align×1,024 B (block_align ≤ 31) or exactly one sample frame (block_align ≥ 32) — measured on ffprobe 7.1.5 with hand-built AVIs. Lumen emits one packet per physical `##wb` chunk (the container's own unit). Packet-count parity is definable only where the granularities coincide; audio conformance is judged on payload bytes, which match the oracle exactly._

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
| mjpeg AVIs with oracle dims (w×h) | 75/76 |
| oracle dims equal generator intent | 72/75 |
| oracle pix_fmt families (mjpeg AVIs) | yuv444=29, yuv420=25, yuv422=21 |
| oracle pix_fmt family equal generator intent | 66/75 (intent recorded for 75/75 — the av draw path persists no pix_fmt) |
| pcm AVIs with oracle sample_fmt | 52/54 |
| oracle sample families (pcm AVIs) | s24=26, s16=11, u8=9, f32=6 |
| oracle channel layouts (pcm AVIs) | 2=16, 1=14, 3=8, 4=6, 8=5, 6=3 |
| oracle sample rates (pcm AVIs) | 22050=11, 32000=9, 96000=8, 11025=6, 16000=4, 48000=4, 8000=4, 176400=2, 88200=2, 37800=1, 44100=1 |
| oracle channels+rate equal generator intent | 52/52 |
| mjpeg/pcm streams outside AVI (mov/mkv/wav…) | 8 video / 25 audio (out of scope until their demuxers land — those rows carry no-path markers) |

### Lumen vs oracle match rates

| metric | match | population |
|---|---|---|
| video dims match | N/A — not-emitted (suite gap) (0/75 comparable) | 75 mjpeg AVIs with oracle dims |
| pix_fmt family match | N/A — not-emitted (suite gap) (0/75 comparable) | 75 with oracle pix_fmt |
| audio sample_fmt match | N/A — not-emitted (suite gap) (0/52 comparable) | 52 pcm AVIs with oracle sample_fmt |
| channels+rate match | N/A — not-emitted (suite gap) (0/52 comparable) | 52 |

Lumen-side decoded format/dims: **not-emitted (suite gap)** — the mjpeg/pcm corpus suites measure counts/PSNR/bytes but do not write the decoded `DecodedFrame`'s format (`DecodedFormat::I420`/`Rgba8`) or width/height, nor the PCM output format, into their result rows. Registered follow-up (suite emission): extend `mjpeg-results.tsv` with `out_format`/`out_w`/`out_h` (first decoded frame's DecodedFormat + dims) and `pcm-results.tsv` with `out_desc`/`out_rate`/`out_ch`; this harness already consumes those columns when present — no further driver change needed.

### Mismatch listing (file, expected, got)

- media/0001-avi-video.avi [avi-video]: expected 3840x2160 yuvj420p (family yuv420), got not-emitted (suite gap)
- media/0012-avi-video.avi [avi-video]: expected 32x32 yuvj422p (family yuv422), got not-emitted (suite gap)
- media/0013-avi-video.avi [avi-video]: expected 1023x768 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0017-avi-video.avi [avi-video]: expected 1280x720 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0021-avi-av.avi [avi-av]: expected 360x640 yuvj422p (family yuv422), got not-emitted (suite gap)
- media/0022-avi-av.avi [avi-av]: expected 320x240 yuvj422p (family yuv422), got not-emitted (suite gap)
- media/0026-avi-video.avi [avi-video]: expected 720x480 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0030-avi-video.avi [avi-video]: expected 64x48 yuvj422p (family yuv422), got not-emitted (suite gap)
- media/0035-avi-av.avi [avi-av]: expected 1280x720 yuvj444p (family yuv444), got not-emitted (suite gap)
- media/0041-avi-av.avi [avi-av]: expected 352x288 yuvj422p (family yuv422), got not-emitted (suite gap)
- … and 65 more video rows (full ledger: decoded-rows.tsv)
- media/0021-avi-av.avi [avi-av]: expected s32 (family s24) 32000Hz x1, got not-emitted (suite gap)
- media/0022-avi-av.avi [avi-av]: expected flt (family f32) 22050Hz x1, got not-emitted (suite gap)
- media/0035-avi-av.avi [avi-av]: expected s32 (family s24) 32000Hz x2, got not-emitted (suite gap)
- media/0040-avi-av.avi [avi-av]: expected u8 (family u8) 16000Hz x4, got not-emitted (suite gap)
- media/0041-avi-av.avi [avi-av]: expected u8 (family u8) 96000Hz x2, got not-emitted (suite gap)
- media/0055-avi-av.avi [avi-av]: expected s32 (family s24) 11025Hz x2, got not-emitted (suite gap)
- media/0058-avi-av.avi [avi-av]: expected s32 (family s24) 32000Hz x4, got not-emitted (suite gap)
- media/0063-avi-av.avi [avi-av]: expected s32 (family s24) 48000Hz x1, got not-emitted (suite gap)
- media/0104-avi-av.avi [avi-av]: expected s32 (family s24) 32000Hz x4, got not-emitted (suite gap)
- media/0116-avi-av.avi [avi-av]: expected u8 (family u8) 8000Hz x2, got not-emitted (suite gap)
- … and 42 more audio rows (full ledger: decoded-rows.tsv)

#### Generator-intent vs oracle drift (real mismatches, measurable today)

12 file(s) where the oracle's decoded facts disagree with the generator's declared parameters — dirty-class corruption is one legitimate source; encoder-side format drift (a gray-intent draw encoded as yuvj444p) is another:
- media/0128-avi-video.avi [avi-video]: dims intent 1080x1919 vs oracle 1080x1918
- media/0137-avi-video.avi [avi-video]: dims intent 719x576 vs oracle 718x576
- media/0214-avi-av.avi [avi-av]: dims intent 352x287 vs oracle 352x286
- media/0017-avi-video.avi [avi-video]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0063-avi-av.avi [avi-av]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0191-avi-av.avi [avi-av]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0266-avi-av.avi [avi-av]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- media/0297-avi-av.avi [avi-av]: pix intent gray (family gray) vs oracle yuvj444p (family yuv444)
- … and 4 more

## Suite health

- probe suite rc=skipped, demux rc=skipped, mjpeg rc=skipped, pcm rc=skipped — SUITES SKIPPED (--skip-suites): oracle-side validation run, not a measurement run
- harness wall: 52s
- decoded-rows ledger: `/home/z/my-project/corpus-big/_harness/decoded-rows.tsv` (485 rows, 429 suite-gap cells) — registered follow-up: suites emit `out_format`/`out_w`/`out_h` (mjpeg) and `out_desc`/`out_rate`/`out_ch` (pcm)
