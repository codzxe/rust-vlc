# Module/plugin census summary — VLC 3.0.23 (Task 0.B2, G3)

- **Card**: 0.B2 (doc 13 §3, wave 0.B); methodology: doc 02 §3.2.
- **Date**: 2026-10-19 (all web sources accessed this date).
- **Deliverables**: `modules-linux.json` (real census), `modules-windows.json` /
  `modules-macos.json` (blocked-on-hardware skeletons), this summary,
  `raw/modules/parse-vlc-list.py` (regenerable parser), `raw/modules/manifest.json`.
- **Clean-room status**: 100% of measured data comes from observed runtime output of
  the pinned binary (`vlc --list`, `vlc --list-verbose`). Public wiki/release-note
  citations (S2, doc 02 §3.1) are used only for the Windows/macOS *expected-module
  skeletons* and platform-diff annotations, never for counts.

## 1. Headline numbers

| Metric | Value | Source |
|---|---|---|
| Total module registrations (Linux, measured) | **502** | dumps 05/06 |
| Unique module names among them | **356** | computed |
| Distinct capabilities printed | **56** (+ 4 base entries with no capability line) | computed |
| Sum of per-capability counts | **502** = total (see counting rule, §2) | computed |
| Windows / macOS | **no counts — blocked-on-hardware** | skeletons only |

## 2. Method and counting rule

1. Parsed `docs/baselines/raw/linux/06-vlc-list-verbose.txt` (the `--list-verbose`
   dump) line-by-line: each module header (`name`, description), its `s` shortcut
   lines and its `c <capability> (<score>)` line. Parser:
   `raw/modules/parse-vlc-list.py`; every JSON module object carries `dump_line`,
   the header's line number in dump 06, for traceability.
2. **Counting rule (the sanity check required by the card)**: one entry per
   module-capability *registration* printed by `--list-verbose` — i.e. the 502
   header lines. A single plugin registers multiple entries across capabilities
   (`avcodec`: 5, `lua`: 11, `stats`: 6, `ddummy`: 6, `playlist`: 15,
   `vaapi_filters`: 6). Four entries print **without** a capability line — the
   base/parent plugin modules `export`, `bridge`, `playlist`, `core` — recorded as
   capability `(none)`. Hence sum(by_capability) = total = 502 with **no
   double-counting**: each of the 502 header lines lands in exactly one capability
   group and exactly one seven-category rollup bucket.
3. Cross-checked 1:1, in order, against the 502 module lines of dump 05
   (`--list`): name + description match for all 502 (assertions in the parser; a
   mismatch aborts).
4. G3 rollup rule (capability → one of the seven categories; the seven are the
   card's minimum buckets, everything else is `other` and mapped in §5):

   | Bucket | Capabilities folded in | Module-level exceptions |
   |---|---|---|
   | demuxers (57) | `demux` (55), `demux_filter` (2) | — |
   | decoders (35) | `audio decoder` (18), `video decoder` (14), `hw decoder` (3) | `spu decoder` goes to subtitle |
   | access (57) | `access` (28), `access_demux` (16), `stream_extractor` (1), `stream_directory` (1), `stream_filter` (11 of 26) | 14 `playlist`-named + 1 `lua` `stream_filter` entries counted under playlist; `sout access` counted under other (write side) |
   | filters (110) | `audio filter` (16), `audio converter` (9), `audio resampler` (4), `audio renderer` (1), `audio volume` (2), `video filter` (50), `video converter` (23), `video blending` (1), `glconv` (4) | — |
   | subtitle (32) | `spu decoder` (22), `sub source` (7), `sub filter` (1), `text renderer` (2) | subtitle *demuxers* (`subtitle`, `webvtt`, `demux_stl`, `vobsub`, `demux_cdg`, `ttml`) stay in demuxers; subtitle *packetizers* stay in the packetizer group |
   | playlist (21) | `playlist export` (4), playlist importers = 14 `playlist` `stream_filter` entries + 1 `lua` "Lua Playlist Parser Interface", base modules `playlist` + `export` | VLC implements playlist import as `stream_filter` capability — re-bucketed by module identity, annotated |
   | services-discovery (26) | `services_discovery` (14), `services probe` (10), `renderer probe` (1), `renderer_discovery` (1) | renderer discovery (avahi/Chromecast-style) included |
   | other (164) | everything else (see §4/§5) | includes the `bridge` and `core` base entries |

   Seven-bucket total 338 + other 164 = **502** ✓ (asserted programmatically as
   `seven_category_rollup.sum_check`).

## 3. G3 evidence statement

**G3 is satisfied on Linux with real data** and remains open for Windows/macOS
exactly as the pin record (0.A4) leaves their oracle runs open:

- `modules-linux.json`: every module has name + capability + description (+ score,
  shortcuts, dump line); per-capability counts; per-seven-category rollup counts.
  All 502 traceable to raw dumps 05/06 captured from the pinned
  3.0.23-2-0-g79128878dd binary.
- `modules-windows.json` / `modules-macos.json`: status `blocked-on-hardware`,
  zero fabricated counts; expected platform-specific modules cited to the VideoLAN
  wiki and official 3.0.0 release notes (URLs + access date in the JSONs).
- **Cross-platform diffs**: only Linux is measured, so no measured cross-platform
  diff exists yet; therefore **no unexplained diff = no open defect** at this
  stage. All *anticipated* diffs are annotated (§6): platform module substitution
  (audio/video outputs, capture, hw decode), plus the Linux headless packaging
  scope limitation (below). Windows/macOS hardware runs (hardware-template.md
  §3/§4) will turn anticipated into measured diffs; any residual unexplained diff
  becomes an open Phase 0 defect per doc 02 §3.7.

## 4. Full capability table (Linux, measured)

`our kind` maps to doc 06 §1's 16 plugin kinds (§5 has the full mapping).

| Capability | Count | Bucket | Our kind(s) |
|---|---|---|---|
| demux | 55 | demuxers | demuxer |
| video filter | 50 | filters | video-filter |
| access | 28 | access | access / network-protocol |
| stream_filter | 26 | access 11 / playlist 15 | access / playlist-parser |
| packetizer | 25 | other | **none — engine-design flag** |
| sout stream | 23 | other | none (stream-output pipeline) |
| video converter | 23 | filters | video-filter (renderer-internal) |
| spu decoder | 22 | subtitle | subtitle-parser |
| encoder | 21 | other | encoder (Phase 7) |
| audio decoder | 18 | decoders | decoder |
| access_demux | 16 | access | access + demuxer |
| audio filter | 16 | filters | audio-filter |
| interface | 16 | other | ui-panel / command layer |
| services_discovery | 14 | services-discovery | service-discovery |
| video decoder | 14 | decoders | decoder |
| vout display | 14 | other | video-output |
| sout mux | 11 | other | none (stream-output pipeline) |
| services probe | 10 | services-discovery | service-discovery |
| sout access | 10 | other | none (stream-output sinks) |
| audio converter | 9 | filters | audio-filter |
| sub source | 7 | subtitle | subtitle-renderer / overlay |
| audio output | 5 | other | audio-output |
| (none — base modules) | 4 | playlist 2 / other 2 | host core |
| audio resampler | 4 | filters | audio-filter |
| glconv | 4 | filters | video-filter (hw interop) |
| keystore | 4 | other | none — host secrets service |
| logger | 4 | other | none — host logging |
| playlist export | 4 | playlist | playlist-exporter |
| vout window | 4 | other | video-output (host surface mgmt) |
| addons finder | 3 | other | none — future extension store |
| hw decoder | 3 | decoders | decoder (+ `codec_api::HwAccelerator`) |
| opengl | 3 | other | video-output (host GL provider) |
| art finder | 2 | other | metadata-provider |
| audio volume | 2 | filters | audio-filter (host volume path) |
| demux_filter | 2 | demuxers | demuxer (engine chain) |
| inhibit | 2 | other | none — host app service |
| meta reader | 2 | other | metadata-provider |
| opengl es2 | 2 | other | video-output (host GL provider) |
| text renderer | 2 | subtitle | subtitle-renderer |
| vod server | 2 | other | none (stream-output family) |
| addons storage | 1 | other | none — future extension store |
| audio renderer | 1 | filters | audio-filter |
| extension | 1 | other | ui-panel (WASM) |
| fingerprinter | 1 | other | metadata-provider |
| meta fetcher | 1 | other | metadata-provider |
| meta writer | 1 | other | metadata-provider |
| renderer probe | 1 | services-discovery | service-discovery |
| renderer_discovery | 1 | services-discovery | service-discovery |
| stream_directory | 1 | access | access (archive browsing) |
| stream_extractor | 1 | access | access (archive) |
| sub filter | 1 | subtitle | subtitle-renderer |
| tls client | 1 | other | none — network stack (Rustls) |
| tls server | 1 | other | none — network stack |
| video blending | 1 | filters | video-filter (renderer-internal) |
| xml | 1 | other | none — host library |
| xml reader | 1 | other | none — host library |

## 5. Mapping to our 16 plugin kinds (doc 06 §1) and gaps

Every VLC capability maps to one of our kinds or is explicitly flagged:

1. **access** ← `access` (28), `access_demux` (16, combined source+container for
   capture/disc: v4l2, cdda, dvdnav, libbluray, dvb, rtp…), `stream_extractor` +
   `stream_directory` (libarchive), `stream_filter` non-playlist (11: cache,
   prefetch, inflate/decomp, skiptags, record, hds, adf). In our split, the
   byte-source half of these is the `access` kind; the disc/capture container
   half needs `access`+`demuxer` cooperation (doc 06 §1 rows 1–3).
2. **network-protocol** ← the network scheme registrations inside `access`:
   http/https, ftp/sftp, mms, udp, tcp, rtp, realrtsp/rtsp, satip, rist, nfs
   (and smb — absent here only via the packaging exclusion, §6).
3. **demuxer** ← `demux` + `demux_filter` (57). 51 unique demuxer names.
4. **decoder** ← `audio decoder` + `video decoder` + `hw decoder` (35).
   `hw decoder` (vaapi, vaapi_drm, vdpau_avcodec) + the `avcodec` `ffmpeghw`
   registration (score **10001**, the highest in the whole dump — hardware is
   preferred by default, corroborating MP §2's fixed decision for ADR-001) map to
   `codec_api::HwAccelerator` in `platform-*`, not to a separate kind.
5. **encoder** ← `encoder` (21: x264, x265, x26410b, avcodec, flac, vorbis,
   speex, opus, theora, jpeg, png, lpcm, g711, araw, twolame, shine, t140,
   dvbsub, rtpvideo, edummy, stats).
6. **audio-filter** ← `audio filter` + `audio converter` + `audio resampler` +
   `audio renderer` + `audio volume` (32).
7. **video-filter** ← `video filter` + `video converter` + `video blending` +
   `glconv` (78). The `video converter` family (swscale + a family of chroma
   conversion registrations — i420/i422/yuy2/grey conversions incl. MMX/SSE2
   variants) is renderer-internal for us (wgpu), not a
   user-visible plugin kind — same as VLC treats them (score-ranked internals).
8. **audio-output** ← `audio output` (5: alsa, pulse, adummy, afile, amem).
   Reality check vs platform-linux.md: our Linux target is PipeWire-era; VLC 3.0.23
   Debian ships alsa+pulse only.
9. **video-output** ← `vout display` (14) + `vout window` (4) + `opengl`/`opengl
   es2` (5). We collapse all of this into one kind (wgpu presenter + winit
   surfaces); VLC's 23 display/window/GL-provider registrations are engine
   internals in our architecture.
10. **subtitle-parser** ← `spu decoder` (22): text (subsdec, webvtt, ttml,
    subsusf, substx3g, aribsub, scte27, scte18, stl, textst), bitmap (spudec/DVD,
    dvbsub, svcdsub, cvdsub, kate), teletext/cc (zvbi, cc). Subtitle *demuxers*
    (`subtitle`, `webvtt`-demux, `demux_stl`, `vobsub`, `demux_cdg`, `ttml`-demux)
    stay demuxers, feeding these parsers.
11. **subtitle-renderer** ← `text renderer` (freetype, tdummy) + `sub source`
    (marq, rss, logo, mosaic, remoteosd, dynamicoverlay, audiobargraph_v) +
    `sub filter` (subsdelay). VLC's OSD/sub-source generators have no separate
    kind of ours — they are overlay content producers; flag for doc 08
    (which of marq/rss/logo we surface in v1 UI).
12. **playlist-parser** ← 14 `playlist` stream_filter importers (m3u/m3u8, xspf,
    asx, pls, b4s, ram, wpl, itml, qtl, sgimb, podcast, dvb, shout-winamp, dummy
    IFO) + `lua` playlist interface.
13. **playlist-exporter** ← `playlist export` (4: html, xspf, m3u8, m3u).
14. **metadata-provider** ← `meta reader` (taglib, lua) + `meta fetcher` (lua) +
    `meta writer` (taglib) + `art finder` (folder, lua) + `fingerprinter`
    (Acoustid). VLC's read/fetch/write/art split collapses into one kind with a
    `provides` manifest (doc 06 §3.2) — good confirmation of that design.
15. **ui-panel** ← `extension` (1: lua) only. The 16 `interface` registrations
    (lua interpreter + lua telnet/cli/http, ncurses, oldrc, dbus, hotkeys,
    xcb_hotkeys, gestures, lirc, motion, netsync, logger, audioscrobbler,
    dummy) are **control-plane
    surfaces** — for us that is the command layer (doc 05) and hotkey map, not
    ui-panels; ui-panel stays extension-only per doc 06 §1.4.
16. **service-discovery** ← `services_discovery` (14) + `services probe` (10) +
    `renderer probe` + `renderer_discovery` (26): udev (v4l/discs/alsa),
    pulselist, podcast, sap, upnp, avahi/mdns, mediadirs, mtp, xcb screen,
    lua. `services probe` registrations duplicate the SD families (probe vs
    enumerate phases) — our single kind covers both.

**VLC capabilities with no v1 analog (flagged, not hidden):**

- **`packetizer` (25) — the big one.** VLC packetizers frame elementary streams
  (h264, hevc, mpegvideo, mpeg4video, mpeg4audio, mpegaudio, a52, dts, mlp,
  flac, vorbis, speex, theora, oggspots, lpcm, aes3, av1, vc1, dirac, copy,
  avparser + subtitle packetizers spudec/svcdsub/cvdsub) between demuxer
  and decoder/muxer. Our plugin kinds have no packetizer: doc 06's `demuxer`
  contract already emits framed `Packet`s, and `decoder` consumes them. Gap
  surfaces for (a) raw-ES playback (VLC's `es` demuxer with 11 audio shortcuts
  leans on packetizers) and (b) any future transcode/mux work. → **open item for
  doc 03 (engine plan) and doc 06 Q&A, not a new v1 kind.**
- **Stream-output family (46)**: `sout stream` (23: transcode, duplicate, bridge,
  display, rtp, gather, chromecast…), `sout mux` (11: ts, mp4, ogg, asf, ps,
  avi, wav, mpjpeg, dummy, avformat), `sout access` (10), `vod server` (2). We
  are a player, not a streaming server (MP scope); `encoder` (Phase 7) covers the
  transcode core. Out of v1 scope by design.
- **Host services with no plugin kind** (needed as app services, not plugins):
  `keystore` (4: secret/memory/file/kwallet), `logger` (4), `inhibit` (2:
  xdg/dbus screensaver), `xml`/`xml reader` (libxml2 service), `tls client`/
  `tls server` (gnutls; we use Rustls in the network stack), `addons finder`/
  `addons storage` (4 — our future extension-store), `core` base module.
- **`audio volume`** (float/integer mixers) — host audio path detail, ours lives
  in the audio-output/master-clock subsystem.

## 6. Platform diffs and known census scope limitation

- **Linux scope limitation (inherited from 0.A2, recorded)**: the 502 modules =
  `vlc-plugin-base` + `vlc-plugin-video-output` only. Debian's
  `vlc-plugin-qt`, `-skins2`, `-notify`, `-samba`, `-jack`, `-svg`,
  `-fluidsynth`, `-visualization`, `-video-splitter`, `-access-extra` were never
  downloaded (headless container). Missing families vs a desktop Linux VLC:
  Qt/skins2 interfaces, smb access, MIDI (fluidsynth), visualizations
  (goom/projectM/vsxu family), video splitters (wall/clone/panoramix),
  additional access extras (e.g. sftp variants / screen / time-shift related
  [VERIFY exact contents of -access-extra on trixie]), JACK output, SVG decode,
  notify OSD. A full desktop-stack census runbook exists (hardware-template.md
  §5). **These are packaging-driven absences, not platform facts.**
- **Windows (expected-only)**: audio outputs mmdevice (WASAPI), directsound,
  waveout; vouts direct3d9, **direct3d11** (3.0.0 release notes), glwin32,
  wingdi; capture `dshow` (access_demux); screen access; DirectX-VA hw decode
  [VERIFY module identity]. Absent vs Linux: alsa/pulse, v4l2/dvb family,
  xcb/Wayland/fb vouts, vaapi/vdpau family, udev SD. See
  `modules-windows.json` (wiki + release-note citations inline).
- **macOS (expected-only)**: audio output auhal (CoreAudio AudioUnit); native
  macosx vout/interface; avfoundation capture (avcapture) [+ qtsound, likely
  stale]; screen access; **VideoToolbox HEVC/H.264 hw decode** (3.0.0 release
  notes) [VERIFY module identity]; MIDI decoder shared with Windows. Absent vs
  Linux: same Linux-only families as Windows. See `modules-macos.json`.
- **Unexplained diffs**: none measured (single platform). Criterion for the
  hardware runs: every Win/mac-vs-Linux diff must match an annotated
  platform-specific substitution or packaging exclusion; anything else is an
  open Phase 0 defect (doc 02 §3.7).

## 7. Inventory highlights (what drives later phases)

- **Phase 5 (format breadth, doc 04) is driven by three census buckets**:
  demuxers (57 registrations / 51 unique container parsers — mp4 240, avi 212,
  asf 200, mkv 50, ogg 50, ts 10, adaptive DASH/HLS…), decoders (35: 18 audio +
  14 video + 3 hw) and subtitle (32, of which 22 spu parsers ≈ doc 04 §5's
  format list). Shortcut leaders across the census: the `es` demuxer alone
  claims 11 audio scheme shortcuts (mpga, mp3, m4a, mp4a, aac, ac3, a52, eac3,
  dts, mlp, thd); the `dtv` access claims 15 (dvb-s/s2/t/t2, isdb-*, atsc…).
- **avcodec is the codec keystone**: 5 registrations across 4 capabilities
  (audio decoder 70, video decoder 70, video decoder ffmpeghw **10001**, spu
  decoder 70, encoder 100). The two highest scores in the entire dump are
  ffmpeghw (10001) and dav1d (10000) — both above any other decoder — i.e. the
  oracle strongly prefers hardware decode, then dav1d, then everything else.
  Direct input to ADR-001 and doc 04 §4's "video codec hole".
- **lua is the extensibility keystone**: 11 registrations across 8 capabilities
  (interface ×4, playlist parser, services_discovery + probe, meta reader,
  meta fetcher, art finder, extension). Our equivalent surface is WASM + the
  plugin kinds above — the census shows what a scripting layer reaches in VLC.
- **filters (110) is the largest seven-bucket** but is polish/UI-effects
  surface (50 video effect filters incl. puzzle/ball games), not Phase 5
  format work; the load-bearing subset for parity is small (adjust, deinterlace,
  scale, croppadd, transform, gradfun, hqdn3d, postproc, swscale-family).
- **Multi-registration shape matters for our plugin manifests**: 502
  registrations / 356 unique names; `playlist` (15) and `lua` (11) show one
  binary presenting many capabilities — doc 06's one-package-many-claims
  manifest design mirrors this correctly.

## 8. [VERIFY] ledger (this task)

| # | Item | Unblock path |
|---|---|---|
| V1 | Windows census counts (all capabilities) | hardware run, hardware-template.md §3 |
| V2 | macOS census counts (all capabilities; universal-binary slices) | hardware run, hardware-template.md §4 |
| V3 | Full Linux desktop-stack census (10 excluded Debian plugin packages; Qt etc.) | desktop run, hardware-template.md §5 |
| V4 | Windows hw-decode module identity (separate d3d11va/dxva2 modules vs avcodec-internal) | `--list-verbose` on win64 3.0.23 |
| V5 | macOS VideoToolbox module identity in 3.0.23 (dedicated module vs avcodec) | `--list-verbose` on macOS 3.0.23 |
| V6 | exact 3.0.23 win64 vout/aout set (direct3d11 name/split; waveout/wingdi/directsound still shipped?) | V1 |
| V7 | qtsound (QuickTime capture) still present on 3.0.23 macOS (wiki page is stale-marked, first VLC 2.0) | V2 |
| V8 | exact contents of Debian vlc-plugin-access-extra / -svg / -visualization (names only, for diff annotation) | V3 / Debian package contents listing |
| V9 | wiki module pages are community-maintained, version-agnostic (some describe pre-3.0 versions; e.g. no direct3d11 page exists) — used only for expected-skeletons, never counts | inherent S2 caveat (doc 02 §3.1) |

## 9. Traceability

- Raw dumps: `docs/baselines/raw/linux/05-vlc-list.txt` (502 terse lines),
  `06-vlc-list-verbose.txt` (same 502, + shortcuts/capability/score), captured
  2026-10-19 by 0.A2 from Debian 3.0.23-0+deb13u1 (base+video-output plugin
  set); acquisition + LD_LIBRARY_PATH: `docs/baselines/raw/linux/02-acquisition-and-hashcheck.txt`.
- Derivation: `docs/inventory/raw/modules/parse-vlc-list.py` (parsing + cross-check
  + rollup assertions); manifest: `docs/inventory/raw/modules/manifest.json`.
- S2 citations: wiki.videolan.org `Documentation:Modules/` index (167 pages) and
  per-module pages (dshow, mmdevice, waveout, directx_aout, direct3d, glwin32,
  wingdi, auhal, avcapture, qtsound, screen); videolan.org/vlc/releases/3.0.0.html
  (Direct3D11 vout, WASAPI pass-through, VideoToolbox hw decode, SMB browsing,
  MIDI decoder). All accessed 2026-10-19.
- Pin identity: `docs/baselines/vlc-3.0.23.md` / `.json` (master pin, 0.A4).
