# Phase-5 records audit — 5.Q2-V (wave 9 redo)

- **Card:** 5.Q2-V — records + differential + blocked-registry audit (doc 13 §8,
  wave 5.Q; AC: "audit report with zero missing rows")
- **Agent:** P5-Q2VR (redo — the original run was lost to sandbox-rollback
  incident #3, worklog 5.RESTART-2)
- **Date:** 2026-10-25 · **Base:** main db90dbc · **Type:** docs + matrix
  backfill — zero codec code
- **Method:** the parity tool (doc 02 §4.6 `tools/feature-audit`, card 8.F1
  debt) is ABSENT, so the documented-honest path is a committed reproducer
  script: `scripts/q2v_matrix_backfill.py` encodes all 38 status transitions
  (baseline db90dbc → target) with evidence citations, applies them
  idempotently, and re-verifies the file (drift check, exit 1) plus the
  R3/R4/R5/R6-shaped rules on every touched row.

## 1. Records inventory — zero missing rows

`ls docs/formats/` = **40 records** (37 pre-existing + 3 authored by this
card: `sub-srt.md`, `sub-ass-ssa.md`, `sub-vtt.md` — the 5.M-wave text-subtitle
formats that had landed without record surfaces). Every Phase-5 format row in
the matrix is accounted for below (83 rows across the six format categories).
Legend: status = desktop columns (win/linux/macos; `web` is `not_started`
everywhere except blocked/unsupported families — wasm32 untested, honestly).
I=implemented, P=partial, B=blocked, U=unsupported, —=not_started.

### 1.1 Containers (21 rows, 12 records)

| Matrix row | St | Record | Differential evidence (Task ID + result) | Gap / next step |
|---|---|---|---|---|
| container-avi | I | — (Phase-2 row, 2.B2 evidence in-matrix) | 2.B2 class-B AC matrix | pre-existing, untouched |
| container-matroska-webm | P | container-matroska.md | 5.D2a 34-lane differential | open() wiring seam open; TrackUID unretained |
| container-mp4-mov-3gp | I | container-mp4.md | 5.C1 (record §Corpus, 30-lane table) | 3GP quirk breadth [VERIFY] |
| container-ogg | I | container-ogg.md | 5.B1 (record §Corpus) | chain header-stash 28v31 divergence recorded |
| container-mpegts | I | container-mpegts.md | 5.I1a+5.I1b (5 ts lanes) | multi-program TS synthetic-only |
| container-mpegps | I | container-mpegps.md | 5.I2 (fixture-only; ZERO ps corpus lanes) | nav/span divergences recorded |
| container-wav | I | container-wav.md | 5.A2 (record §Corpus) | RF64 >4 GB untested |
| container-aiff | I | container-aiff.md | 5.A2 (fixture-only; ZERO aiff lanes) | uLaw breadth [VERIFY] |
| container-adts-aac | I | container-adts.md | 5.C2b: 0059/0110 PASS, dirty 350-vs-351 recorded | — |
| container-mp3-raw | I | container-es.md | 5.I3a | keyframe-IDR-only + 0145 divergences |
| container-h264-annexb | I | container-es.md | 5.F1a+5.I3a | — |
| container-hevc-annexb | I | container-es.md | 5.G1+5.I3a | — |
| container-ivf | I | container-ivf.md | 5.I3b (fixture-only; ZERO ivf lanes) | VP8 typed v0 refusal; 5 malformed-shape divergences |
| container-flac-native | — | none owed (no demux module exists — 11-module census, 5.REC-1) | n/a | **G-6**: native `.flac` carriage undecided (probe→codec path vs demux module) |
| container-mpeg-es-video | — | none (m1v/m2v NOT implemented — container-es.md states it) | n/a | honest not_started |
| container-asf-wmv | — | container-asf.md (decision record, 5.K3-I) | n/a | Phase-6 split-card proposal in-file |
| container-au / -realmedia / -midi-smf / -archive-zip / -census-long-tail | — | none (midi-smf covered by audio-midi.md: honest not_started) | n/a | later/P3, correctly not_started |

### 1.2 Video codecs (16 rows, 11 records)

| Matrix row | St | Record | Differential evidence | Gap / next step |
|---|---|---|---|---|
| codec-video-av1 | I | video-av1.md | 5.E1/5.E2: conformance 15 rows pixel-MD5 PASS; 8-bit bit-exact ×3 | 10/12-bit typed [VERIFY-P1]; asm-off 0.309× vs libdav1d |
| codec-video-mjpeg | I | video-mjpeg.md | 2.B4: 120/120 hash-match; 5.H3: 17-lane, 2 bit-exact, I420 ≤±1 LSB | fancy-upsampling divergence; web col Phase-7 |
| codec-video-h264 | P | video-h264.md | 5.F1a/b: 19/19 corpus; 0019 26/26 + 0014 185/185 slices | hw seam only; software decode absent (§4.2); Phase-7 adapters |
| codec-video-hevc | P | video-hevc.md | 5.G1: parser 3/3 lanes byte-verified; 5.G2: 571/0 | Main-10/4:4:4 corpus absence |
| codec-video-mpeg1-2 | B | video-mpeg12.md | 5.H1: 10/10 sequence-layer differential | Phase-6 gated admission |
| codec-video-mpeg4-asp | B | video-mpeg4asp.md | none (typed refusal — correct) | Phase-6 admission path in-file |
| codec-video-vp9 | B | video-vp8-vp9.md | none (candidate named, unvalidated) | validation spike = Phase-6 entry gate |
| codec-video-vp8 | B | video-vp8-vp9.md | none (no validated engine) | Phase-6+ |
| codec-video-theora / -h264-software / -hevc-software / -legacy-blocked | B | theora/h264/hevc/screen-codecs records | n/a | pre-existing blocked, reasons verified intact |
| codec-video-wmv-vc1 | -/B/U | video-wmv-vc1.md | none (hw-first deferred) | win/linux hw path later |
| codec-video-prores-dnxhd / -ffv1 / -raw-fourcc | — | prores-dnxhd.md / ffv1.md / none | n/a | `later` per 5.P2b-I — correctly not_started |

### 1.3 Audio codecs (19 rows, 12 records)

| Matrix row | St | Record | Differential evidence | Gap / next step |
|---|---|---|---|---|
| codec-audio-pcm | I | — (Phase-2, in-matrix) | 2.B5 bit-exact AC | pre-existing |
| codec-audio-mp3 | I | audio-mp3-mp2.md | 5.A3 (record §Results) | **G-3**: MP3 corpus PCM-MD5 table [VERIFY-P2] lost to deadline — re-print owed |
| codec-audio-mp2 | I | audio-mp3-mp2.md | 5.J3b: ±1 LSB on 4 rows (NOT MD5-exact — two lossy syntheses) | ±1 LSB is the honest ceiling |
| codec-audio-aac-lc | I | audio-aac.md | 5.C2a/b: 6-row 1-LSB table; 0094 6ch typed partial | dirty-lane 350-vs-351 recorded |
| codec-audio-flac | I | audio-flac.md | 5.A1: 4 fixtures byte-exact + STREAMINFO MD5; 539× realtime | wasm32 [VERIFY-P1] |
| codec-audio-alac | I | audio-alac.md | 5.J3a: 5/5 bit-exact | 32-bit row impossible w/ this ffmpeg |
| codec-audio-vorbis | I | audio-vorbis.md | 5.B2: silence BIT-EXACT, others ≤6e-8 | — |
| codec-audio-adpcm | I | audio-adpcm.md | 5.J2a/b: 7 descriptors, 8/8 breadth bit-exact | adpcm_oki: NO ffmpeg reference (typed) |
| codec-audio-truehd-mlp | B | audio-truehd.md | 5.K1 scratch: 4/4 bit-exact, 4,065,640 samples | **G-4**: bitstream-io ToBits OOM; R-10/R-43 unblock decision |
| codec-audio-he-aac | B | audio-aac.md §HE | none (half-rate-LC detection duty) | unchanged, correct |
| codec-audio-opus / -ac3-eac3 / -dts / -wma / -wavpack / -speex / -musepack / -midi-synth | B | opus/ac3/dts/wma/midi records | carriage landed (container rows) | pre-existing blocked, verified intact |
| codec-audio-census-tail | — | none | n/a | later |

### 1.4 Subtitle formats (17 rows, 4 records — G-1 below)

| Matrix row | St | Record | Differential evidence | Gap / next step |
|---|---|---|---|---|
| subtitle-srt | I | **sub-srt.md (NEW)** | 5.M1: 6/6 ffmpeg 7.1.5 differential incl. UTF-16 both endians | X1:/Y1: pixel positioning = damage kind (R4) |
| subtitle-ass-ssa | P | **sub-ass-ssa.md (NEW)** | 5.M2a: 3/3 real-ffmpeg ASS docs exact; 5.M2b: 9/9 grid pixel-exact | **G-2**: ALL override-tag styling deferred (R-09); libass oracle impossible (C) |
| subtitle-vtt | I | **sub-vtt.md (NEW)** | 5.M3: 3 real ffmpeg fixtures + settings_heavy, ledger exact | X-TIMESTAMP-MAP unapplied (Phase-6 HLS) |
| subtitle-dvb-subs | I | **none — G-1** | 5.N2: NONE (ffmpeg refuses text→bitmap; bit tables cross-checked line-by-line vs ffmpeg RLE semantics) | **G-1**: record owed; corpus lane absent |
| subtitle-vobsub | I | **none — G-1** | 5.N2: NONE (same class) | **G-1**: record owed |
| subtitle-encoding-detection | P | (covered by sub-srt.md) | 5.M1: BOM matrix UTF-8/16LE/16BE | subsdec-class heuristics not started |
| subtitle-pgs / -cc-608-708 | — | none | n/a | 5.N1/5.N3 REDONE in wave 9 (lost to rollback) — rows honestly not_started until they land |
| subtitle-teletext / -kate | B | kate/teletext coverage | n/a | pre-existing blocked, correct |
| subtitle-microdvd / -subviewer / -sami / -ttml / -forced / -autoload / -census-tail | — | none | n/a | correctly not_started (phase 5–8 tails) |

### 1.5 Text render (3 rows) + playlist formats (7 rows)

| Matrix row | St | Evidence | Gap / next step |
|---|---|---|---|
| textrender-subtitle-rasterizer | P | 5.L1/5.L2 compositing (176-cue × 9-alignment roundtrip; serialize 4.06 ms / parse 10.3 ms on 15,840 cues) + 5.M2b placeholder glyph metrics | **G-5**: real font stack NOT started (freetype replacement unchosen); text cues render as placeholder blocks |
| textrender-osd / -subsource-overlays | — | none | correctly not_started |
| plfmt-m3u-m3u8 | P | 4.D1 + 5.O1a: 3 ffmpeg fixtures byte-identical, EXTINF vs ffprobe 9.046543/9.046539, RFC 3986 23+19 pins, 38/0 | master playlists Phase 6; KEY/MAP recorded-not-resolved |
| plfmt-pls / -xspf / -cue | I | 5.O1b golden truth-sets 46/0; 5.O2 frame→ns truth, zero-gap joins, 4600-track stress | no docs/formats record owed (doc 13 5.O* Owns name none) — matrix + worklog are the surface |
| plfmt-asx / -b4s / -census-tail | — | none | later, correct |

## 2. Matrix backfill (the 38 transitions)

Baseline db90dbc had 12 stale-matrix notes in-file (5.REC-1/5.REC-2
"amendment recorded for the Lead") plus the entire 5.M/5.N/5.O surface —
**38 rows** backfilled by `scripts/q2v_matrix_backfill.py --apply`
(re-run `--check` to verify; `--explain` prints the table):

- **26 → implemented** (11 containers, 8 audio codecs, av1, srt, vtt, dvb,
  vobsub, pls, xspf, cue) — desktop columns only; `web` honestly not_started.
- **7 → partial** (matroska, h264, hevc, ass-ssa, encoding-detection,
  textrender-rasterizer, m3u-m3u8) — notes describe the working subset (R6).
- **5 → blocked ×4** (truehd-mlp pending-unblock, mpeg1-2, mpeg4-asp, vp9,
  vp8) — blocked_reason cites record + card + next step (R5).

Cell counts: not_started 842→723, implemented 9→87, partial 0→21,
blocked 121→141. Full-file R3/R4/R5/R6 re-validated (violations: NONE);
tomllib parse green (the CI structural check's parser).

## 3. Honest-gap register (ranked)

- **G-1** DVB + VobSub (5.N2) landed with NO docs/formats record — this
  audit's scope fixed the three 5.M-wave records only; the bitmap-subtitle
  records are owed (S each; evidence all in worklog 5.N2). Also no
  reference-encoder differential is possible (ffmpeg 7.1.5 refuses
  text→bitmap transcoding) — line-by-line semantic cross-check is the
  documented ceiling.
- **G-2** ASS styling subset deferred (R-09): every override tag dropped+
  counted; placeholder block glyphs; libass differential impossible (§9).
- **G-3** MP3 corpus PCM-MD5 table [VERIFY-P2] lost to a 5.A3 deadline —
  re-print from test logs owed (audio-mp3-mp2.md records it).
- **G-4** TrueHD: bitstream-io 4.10.0 ToBits OOM (root-caused; 4/4 scratch
  bit-exact proves the decoder) — unblock decision at R-10/R-43.
- **G-5** Text rasterizer: no font stack; crate choice [VERIFY-P0] stands.
- **G-6** Native-FLAC container row: no demux module (11-module census);
  carriage path undecided.
- **Family-wide:** VLC 3.0.23 oracle PENDING-INFRA (ffmpeg 7.1.5 is the
  always-on oracle — no vlc binary in the sandbox); libFuzzer CI campaigns
  unwired (Lead-owned; cargo-fuzz absent, 5.Q1); wasm32 columns untested;
  MP2 ±1 LSB ceiling; AV1 10-bit typed; big-endian hosts unmeasured.

## 4. Verification (this session, in-tree)

- `python3 scripts/q2v_matrix_backfill.py --check` → OK (38 transitions,
  252 rows parse, rules hold).
- `cargo test -p subtitles --all-features` re-run on this tree:
  **230 passed / 0 failed** (lib 175 + 13 integration suites incl.
  `srt_canonical_differential` 6, `ass_ffmpeg_ac` 4, `vtt_acceptance` 4,
  `dvb_vobsub_fuzz_survival` 2 + doctests 2) — the record surface the three
  NEW records cite, verified live, not from memory.
- Records authored cite only worklog Task IDs (5.M1/5.M2a/5.M2b/5.M3/5.N2)
  — nothing transcribed from memory.

**Verdict: zero missing rows.** Every Phase-5 format row has a disposition
(record / justified absence); every stale matrix row is backfilled with
evidence; every blocked row carries a reason and a next step.
