# tools/corpus — the Lumen adversarial corpus suite (v3)

The v3 suite is six Python tools that generate, damage, minimize, score and
benchmark the randomized real-media corpus Lumen is tested against, plus the
pre-existing `run_harness.py` judge/report layer. Everything here drives
**ffmpeg as the external oracle** — Lumen never grades its own homework.

| tool | role |
|---|---|
| `gen_corpus.py` | v3 orchestrator: plan → parallel encode → dirty/garbage passes → manifest ledger |
| `corpus_vocab.py` | the drawable universe (resolutions, fps, codecs, VFR expressions, corruption ops…) |
| `corpus_builders.py` | the 12 per-class ffmpeg command builders |
| `minimize.py` | ddmin crash reproducer minimizer (triage companion) |
| `fidelity.py` | PSNR/SSIM + audio RMS/peak differential scoring vs the oracle |
| `speed_bench.py` | decode-speed benchmark: ffmpeg baseline vs challenger runners |
| `run_harness.py` | the judge/report layer that consumes `manifest.json` (see its own header) |

## 1. Philosophy

**Adversarial by construction.** The corpus assumes the media world is
hostile: ~14–22% of clean files get a corrupted sibling (12 byte-level
operators, including cross-file splices and header size lies), 22 garbage
archetypes probe the "not a media file at all" surface, subtitle writers
embed BOMs, CRLF, zero-duration cues, NUL bytes and RTL text, and the VFR
factory produces timestamps a naive clock will mis-handle. Originals stay
clean so oracle comparison is always possible.

**Honest ledger.** Nothing is silently dropped. Encoder rejections,
over-budget skips, capability gaps and ffprobe failures of *clean* files are
all recorded as data in `manifest.json` (`rejected_draws`, `ffprobe_ok`,
`capability_probes`). A draw that ffmpeg cannot encode is a fact about
ffmpeg, not a generator failure.

**Capability discovery, not assumption.** 17 tiny real encodes are probed at
run start (10-bit profiles, prores profiles, ALAC, ADPCM variants, rotation
metadata, VFR-in-mp4, HLS-fMP4, per-container concat-copy). Draws gated on a
probe that failed land in the rejected-draws ledger with the reason. The
suite never hardcodes "this ffmpeg probably supports X" — it asks.

**Random but reproducible.** The seed comes from OS entropy by default; the
*entire* plan is frozen into `plan.json` before the first encode, so a
mid-run kill loses nothing and `--seed S` reproduces the corpus exactly.

## 2. Quickstart

```sh
# smoke: ~1–2 min on 2 cores, 400 MiB budget (measured: 115 files, 76 s)
python3 tools/corpus/gen_corpus.py --out corpus-smoke --preset smoke --jobs 2

# standard: the real corpus (420 files, 1500 MiB budget, UHD-4K ceiling)
python3 tools/corpus/gen_corpus.py --out corpus-big --preset standard --jobs 2
```

Presets (`--preset`, default `standard`):

| preset | count | budget | max res | max dur | heavy pix_fmts |
|---|---|---|---|---|---|
| `smoke` | 80 | 400 MiB | hd720 | 4.0 s | no |
| `standard` | 420 | 1500 MiB | uhd4k | 14.0 s | yes |
| `big` | 650 | 2800 MiB | uhd4k | 16.0 s | yes |
| `insane` | 1000 | 6000 MiB | uhd4k | 20.0 s | yes |

Full flag set (verified against argparse):

```
gen_corpus.py [--out DIR] [--preset smoke|standard|big|insane]
              [--count N] [--budget-mb MB] [--jobs J] [--seed S]
              [--replay manifest.json] [--ffmpeg BIN] [--ffprobe BIN]
              [--no-dirty] [--no-garbage] [--no-validate]
```

- `--jobs` defaults to `min(4, cpu_count)`; encodes run on a ThreadPool
  (subprocesses give real CPU parallelism), manifest writes are serialized.
- `--no-dirty` / `--no-garbage` skip the corruption / garbage passes;
  `--no-validate` skips the per-clean-file ffprobe check (validation is on
  by default and its result is recorded per file as `ffprobe_ok`).
- `--ffmpeg` / `--ffprobe` override the binaries; the `FFMPEG` / `FFPROBE`
  environment variables do the same.
- The generator **refuses** to write into a non-empty `--out` that has no
  `plan.json` (pass a fresh directory, or use `--replay`).

Measured smoke reference (seed `20261023`, jobs 2, ffmpeg 7.1.5):
115 files = 79 clean + 13 dirty + 1 extension-lie + 22 garbage;
**0 clean-file ffprobe failures**, 21/21 classes present, 1 plan-time
rejected draw (`unsupported-draw`), 0 encode failures, 75.7 s wall.

## 3. What lands on disk

```
corpus-big/
  plan.json        # the frozen plan — resume anchor (all argvs + aux content)
  manifest.json    # the v3 ledger (see §5)
  media/0044-vfr-av.mkv ...          # clean files, NNNN-<class>.<ext>
  hls/0034/master.m3u8 + v0.m3u8 ... # HLS trees (master + tier playlists + segments)
  dirty/head-size-lie-003-0160-mp4-video.mp4 ...   # corrupted COPIES (never in place)
  garbage/ebml-size-lie-16.bin ...   # the 22 non-file archetypes
  _tmp/            # per-entry scratch, removed when the run completes
```

## 4. Class catalog (36 clean classes + 3 derived)

Stratification pins 84 (class, family) combinations (62 v3 + 22 v4;
plus 9 mandatory resolutions) so every surface appears at least once per
corpus; random composition fills the rest. Classes map 1:1 onto the 27
builders in `corpus_builders.py`.

| class | what it attacks / proves | example `family` |
|---|---|---|
| `avi-video` | AVI 1.0 demux across 7 codecs (mjpeg, mpeg4, mpeg2video, huffyuv, ffv1, wmv1, wmv2) | `["avi","mpeg4"]` |
| `avi-av` | A+V mux combos incl. 6 PCM flavors (s16/s24/s32/u8/f32/s16be) | `["avi","mjpeg+pcm_s24le"]` |
| `mp4-video` | ISO-BMFF progressive: h264/h265/mpeg4/prores | `["mp4","h265"]` |
| `mkv-video` | EBML/Matroska incl. vp8/vp9/av1/ffv1 | `["mkv","av1"]` |
| `mov-video` | QuickTime variant; carries the rotate-metadata probe | `["mov","mjpeg"]` |
| `ts-video` | MPEG-TS PES/PAT/PMT | `["ts","h264"]` |
| `flv-video` | FLV container (flv1/h264) | `["flv","flv1"]` |
| `webm-video` | WebM subset | `["webm","vp8"]` |
| `ogv-video` | Ogg page stream (theora — see the 32×32 floor, §9) | `["ogv","theora"]` |
| `audio-only` | 17 audio codecs × 12 audio containers, odd sample rates | `["wav","pcm_s32le"]` |
| `muxed-av` | classic A+V pairs across 9 container/codec combos | `["mp4","h264+aac"]` |
| `vfr-av` | timestamp chaos: 6 VFR expression modes (§8) | `["mkv","vp9+opus"]` |
| `multi-av` | multi-audio (≤3) + subtitles (≤2) + chapters (≤12), per-stream rate/channel shaping | `["mkv","h264+aac+subs+chapters"]` |
| `dynres-video` | mid-stream resolution change via concat-copy (capability-probed per container) | `["ts","h264"]` |
| `raw-video` | containerless annex-B H.264/HEVC — no timestamps at all | `["raw","hevc"]` |
| `fmp4-video` | fragmented MP4 (`frag_keyframe+empty_moov+default_base_moof`, random 0.2–2 s fragments) | `["mp4","h264"]` |
| `hls-av` | HLS packaging: 1–3 tier master playlist + segments (TS or fMP4) | `["hlsf","h264+aac"]` |
| `cover-audio` | attached-picture cover art (mp3/flac/m4a + mjpeg) | `["flac","flac+mjpeg"]` |
| `multivideo-av` | two simultaneous video tracks with different codecs/resolutions | `["avi","mpeg4+mjpeg+pcm_s16le"]` |
| `drift-av` | 45–110 s clips sized for A/V clock-drift measurement | `["mp4","h264+aac"]` |
| `meta-bomb-av` | metadata bombs: 512 B–64 KiB values (CJK or ASCII) across 2–6 tags | `["mkv","h264+aac"]` |
| `rot-display` | display-matrix rotation 90/180/270 + flip combos (encoder-assisted `-display_rotation` copy route; flips via tkhd byte-patch, §9) | `["mp4","h264"]` |
| `odd-dims` | 1×1 … 101×101 odd/sub-16 geometry, yuv422/444 where the encoder permits (odd-preserving color-family sources) | `["avi","mjpeg"]` |
| `maxtracks-avi` | AVI with 16–32 streams (video + audio + `-f data` data streams) | `["avi","max"]` |
| `bframe-storm` | h264 bf 3–16 / h265 bf 3–4 + b-pyramid reorder storms | `["mp4","h264"]` |
| `packed-bs` | libxvid packed b-frames, native mpeg4 bf reordering, avcC profile/level oddities | `["mp4","avcc"]` |
| `param-midchange` | mid-stream SPS/profile/bitrate change via per-segment encode + concat copy | `["ts","h264"]` |
| `vbv-burst` | bitrate spikes: concat step (floor→spike) or single-pass VBV (maxrate ≫ b:v, tiny bufsize) | `["mp4","h264"]` |
| `fps-extremes` | 1–2 fps slideshow ↔ 120/180/240 fps tiny-res burst | `["mp4","h264"]` |
| `sr-chaos` | 5512…384000 Hz + odd channel layouts (pan-built 2.1…22.2) | `["wav","pcm_s16le"]` |
| `chapter-bomb` | 1000–3000 FFMETADATA chapters on a seconds-long clip | `["mkv","h264+aac"]` |
| `zeroframe-stream` | video track declared, zero samples (`select=0`; mkv/ts keep it, mp4 drops it — probed) | `["mkv","h264+aac"]` |
| `still-24h` | single-frame (or 1-frame-per-600–7200 s) 6–48 h still, bounded bytes (image2 fractional-framerate loop) | `["mp4","h264"]` |
| `odd-chunk-avi` | movi re-packed as 1-byte-payload chunks + JUNK pad storms (byte post-op; idx1 rebuilt or JUNKed) — every 1-byte chunk is a packet on the demux side | `["avi","mjpeg+pcm_s16le"]` |
| `hevc-in-avi` | the lie-class: HEVC stream renamed to an AVI video fourcc (`h265`/`H265`/…) | `["avi","h265"]` |
| `dirty` *(derived)* | 12 byte-level corruption ops on copies of clean files | — |
| `dirty-lie-ext` *(derived)* | clean bytes, lying extension | — |
| `riff-size-lie` *(dirty-class)* | honest AVI whose RIFF size field lies (oversize / 0xFFFFFFFF / tiny / zero) — lands in `dirty/`, excluded from the clean gate | `["avi","mjpeg+pcm_s16le"]` |
| `garbage` *(derived)* | 22 non-file archetypes (empty, magic-only, size-lies, all-FF, zip/png/jpeg/gif magic…) | — |

## 5. Manifest schema (v4)

`manifest.json` is the single ledger. Top-level keys:

| key | meaning |
|---|---|
| `generator`, `tool_version` | `"tools/corpus/gen_corpus.py"`, `4` |
| `seed` | the run seed (OS entropy, 96 bits, unless `--seed`/`--replay`) |
| `generated_at`, `wall_seconds`, `jobs` | run bookkeeping |
| `count`, `bytes_total` | files written / bytes harvested |
| `bytes_planned_est` | plan-time estimate actually gated against `--budget-mb` |
| `capability_probes` | the 29 probe results (see §7) — the anti-hardcode record |
| `rejected_draws` | every draw not built: `no-builder`, `unsupported-draw`, `over-budget` (with `est_bytes`), or the ffmpeg error line |
| `files[]` | per-file records (below) |
| `per_class` | `{class: {count, bytes}}` aggregates (final write only) |

Per-file record (clean example):

```json
{
  "path": "0044-vfr-av.mkv", "rel": "media/0044-vfr-av.mkv",
  "class": "vfr-av", "family": ["mkv", "vp9+opus"],
  "bytes": 12345, "sha256": "…",
  "params": {"res_name": "vga", "width": 320, "height": 240, "fps": 12,
              "pix_fmt": "yuv420p", "duration": 6.7, "audio_rate": 48000,
              "audio_channels": 2, "audio_codec": "opus",
              "features": {"vfr_mode": "sinus-jitter", "gop": {"g": 12, "bf": 2}}},
  "ffprobe_ok": true,
  "cmd": ["ffmpeg", "-hide_banner", …],   // exact last-pass reproducer
  "argvs": [ … ]                          // all passes (dynres/hls are multi-pass)
}
```

Dirty records add `corruption` (`{"op": "head-size-lie", "target": "ftyp-size"}`,
`{"op":"splice-other","spliced_from":"0160-mp4-video.mp4",…}`) and
`derived_from`; HLS records add `segments` (every file in the tree); garbage
records carry `params.kind`. v4 adds a per-file `post` array on
classes built through byte-level post-ops (`odd-chunk-avi` rechunk notes,
`riff-size-lie` lie params, `rot-display` tkhd-patch params). The v2/v3
join keys — `rel` / `class` / `family` / `params` / `corruption` +
top-level `seed` — are unchanged, so `run_harness.py` (and anything else
joining on the manifest) is back-compatible; v4 only *adds* fields.

## 6. Determinism: seed, plan freeze, resume, replay

- The full plan — every entry's argvs, aux file contents (base64) and a
  per-entry `aux_seed` — is drawn and written to `plan.json` **before any
  encoding starts**. A reaper kill mid-run loses nothing.
- **Resume**: re-run the same command against the same `--out`; the seed is
  recovered from the partial manifest and the identical plan continues
  (the manifest flushes every 12 completions).
- **Reproduce by seed**: `--seed 20261023` re-rolls the same corpus.
- **Replay a manifest**: `--replay corpus-big/manifest.json --out corpus-copy`
  re-runs that manifest's recorded seed verbatim. Replaying a **v3**
  manifest (tool_version < 4) restricts the drawable universe to the v3
  classes, so the recorded seed reproduces exactly the corpus the v3 tool
  would have drawn for that count (proven W5-16: plan/argvs/params
  byte-identical vs the v3 tool; file-level sha256 noise equals the v3
  tool's own run-to-run noise — muxer timestamps + encoder threading).
- Per-entry aux seeding makes each file's content independent of scheduler
  order; completion order cannot change content.  (Two v3-inherited
  caveats, measured: file BYTES carry muxer-timestamp/encoder-threading
  noise run-to-run — v3 itself showed 37/70 differing sha256s across two
  identical invocations; and the dirty pass SAMPLES in completion order,
  so which clean files get corrupted varies run-to-run.  The plan, draws,
  argvs and params are exactly reproducible; the noise is not.)

```sh
python3 tools/corpus/gen_corpus.py --replay corpus-big/manifest.json \
       --out corpus-replay --jobs 2
```

## 7. Capability probes (29, run at start, recorded in the manifest)

v3: `x264_10bit`, `x264_422`, `x264_444`, `x265_10bit`, `vp9_10bit`,
`svtav1_10bit`, `prores`, `prores444`, `alac`, `adpcm_ms`, `adpcm_ima`,
`rotate_meta`, `vfr_mp4`, `hls_fmp4`, `concat_copy:mkv`, `concat_copy:ts`,
`concat_copy:mp4`.  v4 adds: `xvid`, `hevc_avi`, `mpeg2_mp4`,
`rot_copy:mp4`, `rot_copy:mov`, `rot_copy:mkv`, `rot_tkhd`, `odd1x1`,
`zeroframe:mkv`, `zeroframe:ts`, `zeroframe:mp4`, `still_frac`.

Each is a tiny real encode (or encode+ffprobe verification, for
`rotate_meta`). Probe truths measured on this box (ffmpeg 7.1.5,
Debian): all encoder/pix-fmt probes true; `vfr_mp4`, `hls_fmp4` and all
three `concat_copy:*` true; all 12 v4 probes true except
**`zeroframe:mp4` false** (the mp4 muxer silently drops a zero-sample
video track — mkv/ts keep it, so those are the drawn containers) and the
historical **`rotate_meta` false** (movenc drops the `rotate=` metadata;
rotation is covered since v4 by the `rot_copy:*`/`rot_tkhd` routes — §9).

## 8. Companion tools

### minimize.py — crash triage (ddmin)

When a corpus file crashes/hangs Lumen, don't stare at 45 MB — reduce it to
the bytes that matter. Two stages: **truncate** (binary-search the minimal
reproducing prefix) then **ddmin** (chunk removal with backtracking + byte
tail-trim). Flake control re-runs the final file N times and reports STABLE
or FLAKY with the observed rate — never silently accepted.

```sh
# 1. wrap the reproduction in a predicate that exits 0 when the bug is present
#    (or use --expect-rc any-nonzero if your command crashes with rc != 0)
python3 tools/corpus/minimize.py \
    --in corpus-big/media/0248-mp4-video.mp4 \
    --predicate 'lumen-check-crash {file}' \
    --out crash-0248.min.mp4 \
    --verify 3 --json min-0248.json
```

Flags (verified against argparse): `--in` (required), `--predicate`
(required; `{file}` and `{size}` substituted), `--out` (default
`<name>.<ext>.min.<ext>`), `--expect-rc` (default `0`; `any-nonzero`; or a
specific code), `--max-tests` (400), `--stage {both,truncate,ddmin}`,
`--chunk-align` (16), `--verify` (3), `--timeout` (60 s per predicate call),
`--json` report. Exit codes: 0 stable, 1 flaky, 2 original doesn't reproduce.

Measured (worklog 4.CORPUS-V3, synthetic crash): 40 KB → **217 B**
(99.46% reduction, 59 predicate calls, 3/3 stable). Re-validated for this
README with a synthetic grep-trigger: 40 960 B → 12 B (99.97%, 38 calls,
3/3 STABLE, 0.8 s).

### fidelity.py — decode-correctness scoring

PSNR/SSIM via **ffmpeg's own filters** (the oracle computes the score) plus
a pure-Python, no-numpy audio RMS/peak scorer over interleaved PCM
(s16le/s32le/f32le/u8). Verdict bands on average PSNR:

| avg PSNR | verdict |
|---|---|
| ≥ 48 dB | imperceptible |
| 42–48 dB | excellent |
| 35–42 dB | good |
| 28–35 dB | noticeable |
| < 28 dB | **FAIL** (exit code 1 — CI-wirable) |

```sh
# video: two rawvideo streams
python3 tools/corpus/fidelity.py --a lumen.raw --b ref.raw --video \
       --pix-fmt yuv420p --size 1280x720 --fps 30 --json fid.json

# audio: two raw PCM streams
python3 tools/corpus/fidelity.py --a lumen.pcm --b ref.pcm --audio \
       --fmt s16le --rate 48000 --channels 2
```

`--video` requires `--size WxH`. Flags: `--a --b --video --audio --pix-fmt
(yuv420p) --size --fps (25.0) --fmt {s16le,s32le,f32le,u8} --rate (48000)
--channels (2) --json`. Measured (worklog): identical files → inf dB /
SSIM 1.0; LSB-noise → 68.95 dB (imperceptible); audio clean → 100%
exact-equal, noisy → −19.3 dBFS RMS. Note: the module docstring mentions a
`--media-a/--media-b` auto-decode mode that argparse does **not** implement
yet — do not use it (registered in §10).

### speed_bench.py — ffmpeg speed parity

The user target "reach ffmpeg's speed" as a measured number: ffmpeg
decode-to-null (`-v error -nostdin -i FILE -f null -`) is the baseline
runner; any number of `--runner NAME='CMD {file}'` hooks are challengers
(the Lumen decode CLI when it lands). Best-of-N wall time (warmup decode
discarded, fresh process per timing), per-class median/p95 tables,
frames/s and realtime multiples via ffprobe, and a ratio-vs-ffmpeg column.
Timeouts are recorded as DNF, never averaged in; classes with < 3 files are
marked low-N.

```sh
# ffmpeg-only baseline over a sample of the corpus
python3 tools/corpus/speed_bench.py --corpus corpus-big --count 40 \
       --repeat 3 --md bench.md --json bench.json

# with a challenger runner
python3 tools/corpus/speed_bench.py --corpus corpus-big --count 40 \
       --runner lumen='lumen decode-null {file}'
```

Flags: `--corpus` (required), `--count` (60), `--stride` (1 — sample every
Nth clean file), `--repeat` (3), `--timeout` (180), `--runner` (repeatable),
`--classes` (comma filter, e.g. `avi-video,mkv-video`), `--json`, `--md`.

## 9. ffmpeg reality notes (measured, don't re-debug)

Eleven constraints found the hard way during v3's adversarial smoke cycles
(ffmpeg 7.1.5; each verified by isolation):

| # | reality | v3's handling |
|---|---|---|
| 1 | `allyuv`/`allrgb` lavfi sources are fixed-size — no `size=` option, so they can't join the random-resolution pool | excluded from `VIDEO_SOURCES` (`corpus_vocab.py`) |
| 2 | libx265 refuses to open with `bf > 4`: "Lookahead depth must be greater than the max consecutive bframe count" | h265 `bf` pool capped at 0–4 (`draw_gop`) |
| 3 | `setpts` sub-frame jitter is **truncated** by the source's `1/fps` timebase — sinus-jitter came out perfectly CFR | `settb=1/90000` inserted *before* the VFR `setpts` expression |
| 4 | `-t` placed after an input lands in the **next input's** option group → infinite encodes from infinite sources (mandelbrot et al.) when `-shortest` is absent | `-t` always emitted output-side (`out_t`) |
| 5 | a bare `-ac` between inputs leaks across option groups ("Multiple -ac options" warnings, broken headers) | `with_ac=False` for multi-track builds; channels shaped per output stream via `-ac:a:i` |
| 6 | the mp4-family muxer refuses MPEG-2.5-rate mp3: "muxing mp3 at 8000hz is not standard" | `MP3_MP4_RATES` = MPEG-1/2 rates only (16 k–48 k) for mp4/m4a targets |
| 7 | the native ac3 encoder accepts only 32 / 44.1 / 48 kHz | ac3 rate pool override |
| 8 | libtwolame (mp2) accepts MPEG-1/2 rates only | mp2 rate pool override |
| 9 | the wavpack encoder is named `wavpack`, **not** `libwavpack` | `AUDIO_CODECS["wavpack"].enc = "wavpack"` |
| 10 | `-hls_fmp4_init_filename` must be **relative to the segment dir** (an absolute path makes the muxer look inside the segment dir and fail: "Failed to open segment"), while `-hls_segment_filename` may be absolute | init filename emitted as a bare `init{i}.mp4` |
| 11 | libtheora below 32×32 writes broken streams **with rc = 0** — the encode "succeeds", the decode side rejects it | `_theora_min` bumps to the 32×32 floor + the per-clean-file ffprobe validation net catches any sibling case |

Plus one positive capability truth: `concat -c copy` with a mid-stream
resolution change **works** for mkv/ts/mp4 (probed per container;
`dynres-video` is gated on it).

### 9a. v4 escalation realities (measured W5-16, ffmpeg 7.1.5)

Twelve more constraints/behaviors found while building the 15 escalation
classes (each verified by isolation on this box):

| # | reality | v4's handling |
|---|---|---|
| 12 | the movi **LIST size must be repatched** after re-chunking: a stale (smaller) size makes every rebuilt idx1 entry land "outside movi" and demuxers truncate the walk (measured: ffmpeg decode failed with "No JPEG data found"; ffprobe still said `avi`) | `post_avi_rechunk` patches LIST size + RIFF size (header fields like `dwSuggestedBufferSize` stay lying — on purpose) |
| 13 | the v3 `rotate=` metadata route is **dead** on ffmpeg 7.x (movenc drops it; probe `rotate_meta`=N) — but `-display_rotation` on the **input** + `-c copy` writes real display matrices in movenc AND matroskaenc | `rot-display` pure rotations ride that route (probed per container); `rotate_meta` stays recorded-false |
| 14 | ffmpeg 7.1.5 **refuses `-display_hflip`/`-display_vflip` as CLI options** — flip combos have no encoder route | flips ride the `tkhd` byte-patch route (16.16 fixed-point matrix at body+40/+52, composed with the rotation quadrant; probe `rot_tkhd` verifies ffprobe sees the matrix) |
| 15 | `testsrc2` **rounds odd sizes down to even internally** (a 1×1 draw fails as 0×0) — only the color-family sources preserve odd geometry | `odd-dims` uses `color`/`rgbtestsrc`/`smptebars`/`gradients`/`testsrc`/`nullsrc` (`ODD_SOURCES`) |
| 16 | mpeg4/mjpeg/ffv1 **accept odd dimensions** (1×1, 33×17, 7×13 all verified — no silent crop); h264 needs yuv444p for odd dims (4:2:0 silently crops) | odd-dims codec pool + h264 pinned to 444 (probe-gated) |
| 17 | the AVI muxer **refuses subtitle streams** ("Not yet implemented") but accepts **data streams** via `-f data` inputs, muxing them with an empty fourcc (ffprobe: `codec_type=unknown`) | `maxtracks-avi` builds its 16–32 streams from video + audio + data bins only |
| 18 | the mp4 muxer **silently drops a zero-sample video track** (rc=0, track gone); mkv keeps the empty track and ts still declares it in the PMT | `zeroframe-stream` probes per container (`zeroframe:mp4`=N) and draws mkv/ts only |
| 19 | **`-b-pyramid 1` is accepted by both libx264 and libx265** on this build (x265 logs `b-pyramid / weightp / weightb : 1 / 0 / 0`) | `bframe-storm` emits it for both codecs; h265 `bf` stays ≤ 4 (x265 lookahead, v3 #2) |
| 20 | ffprobe's **format probe can tie** on mp3+APIC cover files: `h263 score:25` vs `mp3 score:25`, content-dependent → occasional "Invalid data found" on a file ffmpeg decodes fine (observed 1-in-3 v3 replays; v3-inherited, not new in v4) | recorded as a known flake; the clean gate records it honestly when it fires — re-roll the seed for an all-green smoke |
| 21 | audio-container classes (e.g. `sr-chaos` drawing `wav`/`mka`) must resolve their extension via the audio-container table — `CONTAINERS["wav"]` is a KeyError crash | `draw_plan` routes `sr-chaos` through `AUDIO_CONTAINERS` first |
| 22 | `tuple | set` is a TypeError — the dirty-pass class filter crashed the first full v4 run after the clean phase (the predecessor never completed a run) | set-literal class filter; full run green since |
| 23 | a 6–48 h still from ONE frame needs `image2 -loop 1` at a **fractional framerate** (`-framerate 1/172800`); a plain `-r 1` loop would emit 172 800 frames.  movenc additionally writes **32-bit track durations** in the track timescale (16384 here): single-frame cadences past 2³¹/16384 = 131 072 s (~36.4 h) fail with "Application provided duration … is invalid" (36 h OK, 40 h fails, measured; mkv unlimited) | `still-24h` two-pass (1 PNG → loop), `est_hint` bounded, probe `still_frac` verifies duration > 10 000 s; long mp4/mov draws switch to the coarse slideshow cadence |

## 10. Registered gaps (honest, follow-ups)

- **PSNR per-frame series** parses empty in `fidelity.py` summary mode
  (observed live: `frames=0`, `min=None` while the summary average is
  correct) — the regex needs fixing before per-frame worst-frame reporting.
- **`fidelity.py` auto-decode mode** (`--media-a/--media-b` in the
  docstring) is not implemented in argparse — decode to raw first.
- **Lumen decode-CLI runner** for `speed_bench.py` awaits the
  decode-null CLI card; today only the ffmpeg baseline row exists.
- **Rotation coverage** — RESOLVED in v4: `rot-display` writes real
  display matrices via the encoder-assisted `-display_rotation` + copy
  route (mp4/mov/mkv) and the tkhd byte-patch route (flip combos); the v3
  `rotate=` metadata route remains dead (§9 #13).
- **DASH packaging** (MPD manifests) is not yet a class — HLS/fMP4 is.
- The dirty pass rate is 14–22% *of clean files* (minimum 6) — if a
  specific corruption op needs guaranteed presence per run, add it to
  `STRATIFICATION` (none of the 12 ops is currently pinned).

## 11. Vocabulary bounds at a glance

Everything below is *drawable*, not decided: every concrete value is rolled
per file by the seeded RNG (`corpus_vocab.py` is the bounds, not the plan).

- **29 resolutions**, 16×16 → 3840×2160, including 3 portrait modes
  (360×640, 720×1280, 1080×1920) and a square (512×512); ~18% of
  odd-allowed draws get a deliberately odd width or height (−1 px).
- **21 frame rates**, 0.5 → 240 fps.
- **15 video codecs** (mjpeg, mpeg4, mpeg2video, huffyuv, ffv1, wmv1, wmv2,
  h264, h265, vp8, vp9, av1, theora, flv1, prores) × 8 video containers
  (avi, mp4, mov, mkv, webm, ts, flv, ogv) + raw annex-B H.264/HEVC.
- **17 audio codecs** (pcm s16/s24/s32/u8/f32/s16be, flac, alac, mp3, aac,
  opus, vorbis, ac3, wavpack, mp2, adpcm_ms, adpcm_ima) × 12 audio
  containers; standard rates + an odd-rate pool (5512, 37800, 44056,
  47250, 176400, 384000 Hz).
- **VFR expression factory** — 6 modes: `sinus-jitter`, `staircase`,
  `duty-gaps`, `start-offset`, `telecine-ish`, `chaos-nonmono` (the last
  may legitimately be muxer-rejected; that's ledger data).
- **GOP anarchy** — g ∈ 1…999, b-frame pools per codec, b-pyramids,
  open-GOPs.
- **7 anamorphic SARs** (16:15, 32:27, 12:11, 10:11, 40:33, 59:54, 64:45)
  and a 5-entry rotation pool (gated on `rotate_meta`).
- **v4 escalation bounds** — 8 display transforms (3 rotations + flips via
  `ROT_TRANSFORMS`); odd dims 1–15 and 17–101 (odd-only ranges); 16–32 AVI
  streams; h264 bf 3–16 / h265 bf 3–4 with b-pyramids; avcC
  profile→level ladders; midchange bitrates 30–120 k ↔ 0.9–4 M;
  VBV floor 24–80 k / spike 1.5–6 M / buf 8–120 kB; fps 1–2 ↔ 120–240;
  sr-chaos 5512–384000 Hz × 12 layouts (2.1 … 22.2); 1000–3000 chapters;
  6–48 h stills (single-frame or 600–7200 s slideshow cadence); RIFF lie
  modes (oversize/max/tiny/zero); rechunk splits {1,1,1,2} with JUNK
  storms; HEVC-in-AVI fourcc lies (5 tags).
- **Adversarial writers** for SRT (BOM, CRLF, out-of-order indices,
  zero-duration and beyond-duration cues, `-->` inside text), WebVTT, ASS
  (positional overrides, karaoke tags, random styles) and FFMETADATA
  chapters (3–12, randomly cut).
- **12 corruption ops** (truncate, zero-mid, flip-bytes, smash-head,
  tail-garbage, cut-mid-chunk, lie-ext, swap-halves, dup-range,
  splice-other, head-size-lie, bit-drift-region) and **22 garbage
  archetypes**.

*Scope note: the randomized corpus (`corpus-big/`, gitignored) is the
primary adversarial asset; the committed `tests/fixtures/` tree is the
pinned regression set (byte-stable, hash-checked) — see
`tests/fixtures/CORPUS.md` and `docs/inventory/corpus-plan.md` §"v3
status".*
