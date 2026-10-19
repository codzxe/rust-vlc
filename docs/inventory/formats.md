# Formats inventory — Linux (probe-verified vs census-only)

Task 0.B4-R (retry of 0.B4, parsing-only: no new oracle sessions were run).
Machine-readable twin: `formats.json` (schema `lumen-phase0/formats-inventory/1.0`,
82 families). Per-probe verbatim evidence:
`../baselines/raw/linux/17-format-probes.txt`. Census: `modules-linux.json`.

Oracle: pinned VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd), Debian 13 debs
extracted at `tmp/pinning/extract/`, driven headless:
`timeout 25 vlc -I dummy --play-and-exit --aout dummy --vout dummy -vv <fixture>`.
Evidence base: 61 log files (60 VLC sessions + 1 python http.server daemon log)
in `probe-work/logs/`, of which 58 contain module-selection lines; `status.txt`
records 57 exit codes (all 0 except `rtp_ts` = 124, a live-stream timeout).

## 1. Headline counts

| kind | families | probed | census-only |
|---|---|---|---|
| container | 21 | 18 | 3 |
| audio-codec | 18 | 13 | 5 |
| video-codec | 12 | 9 | 3 |
| subtitle | 12 | 8 | 4 |
| playlist | 7 | 6 | 1 |
| protocol | 12 | 5 | 7 |
| **total** | **82** | **59** | **23** |

Probed = a preserved `-vv` log shows the module selection for that family.
Census-only = the module registers in the pinned build (`modules-linux.json`)
but no fixture was played against it.

### P0 families (must-work target set)

- **Containers:** matroska-webm, mp4-mov-3gp, ogg, mpegts, wav, flac-native,
  adts-aac, mp3-raw, h264-annexb
- **Audio:** pcm-family, mp3, aac-lc, flac, vorbis, opus
- **Video:** h264, vp9, av1
- **Subtitles:** srt, ssa, ass
- **Playlists:** m3u-m3u8, pls, xspf
- **Protocols:** file, http-https, directory

(27 of 82 families; everything else is P1×23 / P2×19 / P3×13 in `formats.json`
with one-line rationales anchored to doc 04 §1.3 where doc 04 has an opinion.)

## 2. Probe method (what the prior 0.B4 session did, and what the logs prove)

1. **Fixtures — our own synthetic files, clean-room:**
   - Pure-Python struct-writers (`probe-work/gen_py.py`, preserved, 16 numbered
     fixtures): canonical WAV, 6 playlists, 8 text-subtitle files, one
     directory fixture. Byte-exact recipes are in the committed script.
   - ffmpeg synthesis (lavfi sine/testsrc at 64×48 / 8–48 kHz mono, ~1 s
     clips): 30 media fixtures covering containers and codec payloads.
     **The ffmpeg command lines were not preserved** — content re-derived
     2026-10-19 with `ffprobe` on the still-present (gitignored)
     `tmp/probe/media/` files, plus x265 encoder remnants in `gen_hevc.err` /
     `gen_mp4hevc.err` [V1]. Media fixtures are regenerable only
     approximately [VERIFY].
2. **Invocation:** headless dummy in/out, 25 s timeout, `-vv` debug logs,
   exit codes captured in `status.txt` (57 rows).
3. **What a "probed" verdict requires:** a log line of the form
   `using demux module "X"` / `using (audio|video|spu) decoder module "Y"` /
   `using access module "Z"` / `using access_demux module "rtp"` /
   `using stream_filter module "playlist"` for the fixture under test.
   Module-selection lines are quoted verbatim per probe in
   `17-format-probes.txt`.
4. **Known blind spots of the method (honesty):**
   - `--aout dummy` makes raw AC-3/E-AC-3/DTS select the **spdif passthrough
     "decoder"** — framing is proven, PCM decode is not [V3].
   - Decoding is proven by *module selection + stream-out events*, not by
     bit-exact output comparison (no rendering occurred); that deeper oracle
     is Phase 5 differential-testing scope.
   - The pinned extract excludes 10 Debian plugin packages (qt, samba, …,
     **fluidsynth**) — MIDI decode is therefore absent from this oracle [V10].
   - The dummy-vout log line `original format sz 128x128 ... GBR8` is the
     vout's internal buffer, not the video size (resize event says 64×48).

## 3. Per-kind result tables (family → chosen modules → verdict)

### 3.1 Containers

| family | chosen demux (→ packetizer) | chosen decoders | verdict |
|---|---|---|---|
| matroska-webm | `mkv` | avcodec(h264), flac, dav1d(av1), opus, avcodec(vp9) | probed ×3 (MKV×2, WebM) |
| mp4-mov-3gp | `mp4` | avcodec(h264/hevc), faad(aac), avcodec(alac) | probed ×3 (incl. M4A) |
| ogg | `ogg` | vorbis, opus, speex, theora | probed ×4 |
| mpegts | `ts` (file); `rtp` access_demux → `ts` (net) | avcodec(h264), mpg123 | probed ×3 (file/udp/rtp) |
| mpegps | `ps` (→ mpegvideo) | avcodec(mpgv) | probed |
| avi | `avi` | avcodec(MJPG/mp4v) | probed ×2 |
| asf-wmv | `asf` | avcodec(WMV2/WMA2) | probed |
| wav | `wav` | araw, avcodec(ADPCM) | probed in 6 distinct sessions |
| flac-native | `flacsys` (→ flac) | flac | probed |
| aiff | `aiff` | araw(s16b) | probed |
| au | `au` | araw(s16b) | probed |
| adts-aac | `es` (→ mpeg4audio) | faad | probed |
| mp3-raw | `es` (→ mpegaudio) | mpg123 | probed |
| mpeg-es-video | `mpgv` (→ mpegvideo) | avcodec | probed |
| h264-annexb | `h26x` (→ h264) | avcodec | probed |
| hevc-annexb | `h26x` (→ hevc) | avcodec | probed |
| midi-smf | `smf` | — | demux positive, decode negative (fluidsynth excluded) |
| archive-zip | `archive` stream_directory → `wav` | araw | probed |
| realmedia | `real` (census) | — | not-probe-verified |
| adaptive-hls-dash | `adaptive` (census) | — | not-probe-verified |
| census long tail | caf/voc/xa/tta/mpc/mod/sid/nsv/pva/ty/nuv/rawdv/rawvid/rawaud/image/diracsys/vc1/nsc/cdg/stl/vobsub/sap/stats/demuxdump/linsys_sdi/libbluray/dvdnav | — | not-probe-verified |

### 3.2 Audio codecs

| family | chosen decoders | verdict |
|---|---|---|
| pcm-family | `araw` (s16l + s16b across wav/aiff/au) | probed |
| adpcm | `avcodec` (fourcc `  ms`) | probed (IMA variant) |
| mp3 | `mpg123` (+`mpegaudio` packetizer) | probed (bare + in-TS) |
| mp2 | (mpg123/mad census) | not-probe-verified [V7] |
| aac-lc | `faad` (ADTS + MP4 framings) | probed |
| alac | `avcodec` | probed |
| flac | `flac` (native, in FLAC + in MKV) | probed |
| vorbis | `vorbis` | probed |
| opus | `opus` (Ogg + WebM + MKV) | probed |
| speex | `speex` | probed |
| ac3-eac3 | `spdif` (passthrough) + `a52` packetizer | framing probed; decode not exercised [V3] |
| dts | `spdif` + `dts` packetizer | same caveat [V3] |
| wma-1-2-pro | `avcodec` (WMA2) | probed (WMAv2 only) [V7] |
| truehd-mlp / wavpack / musepack / pcm-siblings / midi-synth | census (`mlp` packetizer; avcodec; lpcm/aes3/g711; fluidsynth absent) | not-probe-verified (midi: decode-negative probed) |

### 3.3 Video codecs

| family | chosen decoders | verdict |
|---|---|---|
| h264 | `avcodec` (MP4/MKV/TS/Annex-B; + all 8 subsl runs) | probed ×4 contexts |
| hevc | `avcodec` (MP4 + Annex-B) | probed |
| vp9 | `avcodec` (WebM) | probed |
| vp8 | `avcodec` (census; no libvpx in build) | not-probe-verified |
| av1 | **`dav1d`** (MKV) | probed |
| theora | `theora` | probed |
| mpeg1-2 | `avcodec` (PS + bare ES) | probed |
| mpeg4-asp | `avcodec` | probed |
| mjpeg | `avcodec` | probed |
| wmv2 | `avcodec` | probed |
| vc1-wmv9 / video census tail (rawvideo, png, jpeg, svgdec, xwd, cdg, oggspots, dirac) | census | not-probe-verified |

### 3.4 Subtitles

Two probe passes: **standalone open** (negative — a demux claims the file but
0 ES/decoder events, ends "nothing to play") and **sidecar load**
(`subsl_*`: mp4 + `--sub-file`, positive — this is VLC's real subtitle path).

| family | standalone demux | sidecar demux (autodetect) | spu decoder | detected-as line |
|---|---|---|---|---|
| srt | `ps` (probe-race quirk) | `subtitle` | `subsdec` | "detected SubRIP format" |
| ssa | `ps` | `subtitle` | `libass` | — |
| ass | `ps` | `subtitle` | `libass` | — |
| vtt | `webvtt` (0 ES) | `subtitle` | `webvtt` | — |
| microdvd | `ps` | `subtitle` | `subsdec` | "detected MicroDVD format" |
| subviewer | `avformat` | `subtitle` | `subsdec` | "detected SubViewer format" |
| sami | `avformat` | `subtitle` | `subsdec` | "detected SAMI format" |
| ttml | `ttml` (0 ES) | `subtitle` | `ttml` | — |
| pgs / dvb-subs-teletext / cc-608-708 / census tail (spudec, svcdsub, cvdsub, kate, subsusf, substx3g, stl, aribsub, scte18/27, textst, vobsub) | census only (`avcodec` spu, `dvbsub`, `zvbi`, `cc`, …) | — | — | not-probe-verified |

Quirks worth copying into our engine spec: (a) `webvtt demux debug: subtitle
demux discarded` then `ps ... continuing anyway` — the standalone srt/ssa/ass/
microdvd probe race is won by the **ps demuxer**, with "garbage at input"
resync warnings, and still yields no track; (b) sidecar loads log
`forced subtitle: … (forced: 1)` and `loading spu-es slave`; (c) before
subsdec wins, avcodec logs `refusing to decode non validated subtitle codec`;
(d) MicroDVD fps is inferred from the media file ("Movie fps: 10.000000").

### 3.5 Playlists (stream_filters, not demuxers)

| family | detection evidence | item added? | verdict |
|---|---|---|---|
| m3u-m3u8 | "found valid M3U playlist" (shortcut m3u/m3u8) | yes → wav+araw | probed (file + http) |
| pls | "found valid PLS playlist file", "pls file version: 2" | yes | probed |
| xspf | "using XSPF playlist reader" | yes | probed (reader + writer via ml.xspf) |
| asx | "found valid ASX playlist" | yes | probed |
| b4s | shortcut `shout-b4s`; xml reader engaged; no "found valid" line | yes (item replaced by entry name) | probed |
| cue | none — no cue module in census | **no** — negative probe, "nothing to play", 0 ES | probed (negative) |
| census tail (itml, wpl, qtl, sgimb, podcast, shout-winamp, dvb, lua) | census | — | not-probe-verified |

### 3.6 Protocols / access paths

| family | chosen access module | verdict |
|---|---|---|
| file | `filesystem` (file/fd/stream) | probed (every local session) |
| http-https | `access` (the 3.0 http(s) plugin; shortcuts http/https) + `prefetch`/`cache_read` filters; GET/response lines in log | probed (plain http; https not [V4]) |
| directory | `filesystem` (directory/dir) + demux `directory` | probed |
| udp | `udp` ("opening server=:0 local=:5004") | probed |
| rtp | `rtp` **access_demux** ("detected MPEG2 TS", payload 33) | probed (exit 124 = live-stream timeout, decode observed) |
| rtsp / smb / nfs / ftp-sftp / mmsh-mms / disc-family / census tail (tcp, sdp, rist, satip, concat, attachment, avio=rtmp, upnp, vdr, imem, capture…) | census (smb: **absent** — vlc-plugin-samba is an excluded deb) | not-probe-verified |

## 4. Comparison vs doc 04 (`04-codec-format-roadmap.md` §2–5)

doc 04 column = pure-Rust status per doc 04 tables; phase = doc 04 phase
target (playlists/protocols have no doc 04 row — 0.C3-I `subsystems.md` is
the anchor there). "our probe" column cites the oracle module that our
implementation must eventually match behaviorally.

| family | doc04 status (§) | our phase target | oracle chose (probe) |
|---|---|---|---|
| WAV | pure-rust-exists (§2 P0) | 2 | wav + araw |
| FLAC native | pure-rust-exists (§2 P0) | 2 | flacsys + flac |
| MP4/MOV | partial (§2 P0) | 2 | mp4 + avcodec/faad |
| Matroska/WebM | partial (§2 P0) | 2 | mkv + avcodec/flac/dav1d/opus |
| Ogg | pure-rust-exists (§2 P0) | 2 | ogg + 4 native decoders |
| MPEG-TS | pure-rust-exists (§2 P0) | 5 | ts (+ rtp access_demux) |
| MP3 raw | pure-rust-exists (§2 P0) | 2 | es + mpegaudio + mpg123 |
| ADTS | partial (§2 P0) | 5 | es + mpeg4audio + faad |
| MPEG-PS | pure-rust-exists (§2 P1) | 5 | ps + mpegvideo + avcodec |
| AVI | missing (§2 P1) | 5 | avi + avcodec |
| AIFF | pure-rust-exists (§2 P1) | 5 | aiff + araw |
| ASF/WMV | missing (§2 P2) | 6 | asf + avcodec×2 |
| RealMedia | missing (§2 P3) | later | (census: real) |
| MPEG-2 video | missing (§4 P1) | 6 | mpegvideo + avcodec |
| MPEG-4 ASP | missing (§4 P1) | later | avcodec |
| H.264 | hw-only (§4 P0) | hw 5–7, parser 2 | avcodec (all framings) |
| HEVC | hw-only (§4 P1) | hw 7 | avcodec |
| VP9 | missing (§4 P0) | later | avcodec |
| AV1 | partial/rav1d (§4 P0) | 5 | **dav1d** |
| Theora | missing (§4 P1) | later | theora |
| MJPEG | pure-rust-exists (§4 P1) | 2 | avcodec |
| WMV2/VC-1 | missing (§4 P2) | later | avcodec |
| PCM family | pure-rust-exists (§3 P0) | 2 | araw |
| ADPCM | pure-rust-exists (§3 P1) | 5 | avcodec (not native adpcm!) |
| MP3 | pure-rust-exists (§3 P0) | 2 | mpg123 |
| MP2 | pure-rust-exists (§3 P1) | 5 | (not probed) |
| AAC-LC | partial (§3 P0) | 2 | faad |
| ALAC | pure-rust-exists (§3 P1) | 5 | avcodec |
| FLAC | pure-rust-exists (§3 P0) | 2 | flac |
| Vorbis | pure-rust-exists (§3 P0) | 2 | vorbis |
| Opus | partial, no crate (§3 P0) | 5–6 | opus |
| AC-3/E-AC-3 | missing (§3 P1/P1) | 6 | a52 pkt + spdif (decode unproven) |
| DTS | missing (§3 P2) | later | dts pkt + spdif |
| WMA | missing (§3 P2) | later | avcodec |
| Speex | missing (§3 P3) | later | speex |
| MIDI | later/rustysynth (§3 P3) | later | smf demux; decode absent |
| SRT | partial (§5 P0) | 2 | subtitle demux + subsdec |
| ASS/SSA | partial, libass forbidden (§5 P0) | 2 text → 5 styling | subtitle demux + libass |
| WebVTT | partial (§5 P1) | 5 | webvtt demux+spu |
| SubViewer/MicroDVD/SAMI | missing (§5 P2) | 5 | subtitle demux + subsdec |
| TTML | missing (§5 P2) | 6 | ttml demux + spu |
| PGS/DVB/VobSub/CC | missing (§5 P2/P1) | 6 / later | (census: avcodec-spu, dvbsub, zvbi, cc) |
| playlists (m3u/pls/xspf/asx/b4s) | no doc04 row — 0.C3-I §5: self-build S×4 | 2–5 | playlist stream_filter |
| protocols (file/http/dir/udp/rtp) | no doc04 row — 0.C3-I §2 | 2 (http/file), later (udp/rtp) | filesystem / access / udp / rtp |

Reading: the probes confirm doc 04's implicit "what VLC actually uses"
pipeline (e.g. FLAC/Opus/Theora/Speex are native modules, ALAC/ADPCM/VP9 go
through ffmpeg), and add the architectural fact that playlists are
stream_filters and rtp is an access_demux — both feed doc 03 §2 engine-shape
decisions.

## 5. G5 evidence statement

Every one of the 82 families in `formats.json` carries
`probe_status: probed` (with `probe_evidence` pointing at specific preserved
log files, chosen modules, and exit codes — verbatim lines consolidated in
`17-format-probes.txt`) or `probe_status: not-probe-verified` (census
registration in `modules-linux.json` is the sole evidence). No family is
silently asserted. Negative probes (cue: unsupported; midi: decode absent)
are recorded as probed-with-verdict, not as gaps.

## 6. [VERIFY] ledger

| id | item | owner/vehicle |
|---|---|---|
| V1 | ffmpeg fixture generation commands not preserved; regeneration recipe approximate (content re-derived via ffprobe 2026-10-19) | 5.A1 corpus task (write committed generators this time) |
| V2 | udp/rtp sender commands not preserved; stream content derived from VLC-side log only (MP3 program, PAT/PMT/SDT walk) | same as V1 |
| V3 | AC-3/E-AC-3/DTS decode unproven (spdif passthrough under `--aout dummy`); re-probe with real aout or forced decoder | Phase 5 differential harness |
| V4 | https not probed (gnutls `tls client` census module untested) | 0.C3-I network follow-up |
| V5 | Usage-share surveys unreachable from sandbox (Bitmovin 403; no verified Canonical survey URL; archive.org down). P0/P1 rationales lean on doc 04 + Wikimedia Commons (free-format skew) | Lead/0.D3 if share numbers become decision-relevant |
| V6 | WMA1/WMA-Pro, MP2-in-TS, bare .m4v, VP8 fixtures absent (adjacent families unprobed) | 5.A1 corpus |
| V7 | adpcm: avcodec chosen over native `adpcm` module — selection reason (codec match/score) not visible in log | note only; no action |
| V8 | PGS module chain (avcodec spu vs libbluray path) unverified on a BD fixture | Phase 6 bitmap-sub work |
| V9 | MIDI decode oracle needs a fluidsynth-equipped build (excluded deb here) — differential testing must not use this extract | Phase 5+ |
| V10 | CUE: our (non-)support is a scope decision the oracle cannot arbitrate (VLC 3.0.23 has none) — [PROPOSAL] flag to 0.D3 | 0.D3 |

## 7. Notes: subsl_* second pass, empty probes, http server

- **`subsl_*` (8 logs)** = subtitle-**slave** second pass, not "subtitle
  language": each ran `mp4_h264_aac.mp4 --sub-file subs/<fmt>` (log: `forced
  subtitle: … (forced: 1)`, `loading spu-es slave`). This is the pass that
  proves actual subtitle parsing/decoding (demux `subtitle` with format
  autodetection + spu decoder selection). VLC also logs `looking for a
  subtitle file in <media dir>` — the same-basename autoscan path was not
  separately exercised [note, not a gap in evidence].
- **`rtp_ffmpeg.log`, `udp_ffmpeg.log` (0 bytes):** failed/aborted sender-side
  capture attempts; no status.txt rows; contain no evidence either way. The
  working network probes (`udp_ts`, `rtp_ts`) have full VLC-side logs.
- **`http_server.log` (4 lines):** the python `http.server` daemon's own log
  ("Serving HTTP on 127.0.0.1 port 8123" + 3 GETs) — server-side half of the
  http_wav/http_m3u probes, kept with the evidence set.
- **`smoke_wav.log`:** pre-status harness smoke run on wav_pcm_py.wav
  (wav+araw confirmed); no status.txt row.
- **`rtp_ts` exit 124:** expected — a live RTP stream never EOFs under
  `--play-and-exit`; the timeout kill happened after full PAT/PMT/SDT/EIT
  processing and decoded audio (mpg123) appeared in the log.
