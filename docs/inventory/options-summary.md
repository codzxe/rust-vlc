# Options & preferences inventory summary — VLC 3.0.23 (Task 0.B1, G4)

- **Card**: 0.B1 (doc 13 §3, wave 0.B); methodology: doc 02 §3.3.
- **Date**: 2026-10-19 (web sources accessed this date).
- **Deliverables**: `options-linux.json` (real parse of real dumps), `options-windows.json` /
  `options-macos.json` (blocked-on-hardware skeletons, no fabricated data), this summary,
  and the regenerable derivation `raw/options/parse-options.py` + `raw/options/manifest.json`.
- **Raw evidence**: `docs/baselines/raw/linux/12-options-longhelp.txt`,
  `13-options-advanced-longhelp.txt`, `14-options-full-help.txt`,
  `15-options-help-and-probes.txt`, `16-options-vlcrc.txt`.
  (Card suggested filename `08-options-longhelp.txt`; numbers 08–11 were already taken by
  0.A2's dumps, so 12–16 were used — recorded here as the honest adaptation.)
- **Clean-room status**: 100% of Linux data comes from observed runtime help/config output of
  the pinned binary (`-I dummy`, no display). Public wiki citations are used only for the
  Windows/macOS *expected-difference* fields, never for counts.

## 1. Headline numbers (Linux, measured)

| Metric | Value | Source |
|---|---|---|
| Option records parsed (`--full-help`, canonical) | **1476** | dump 14 |
| — visible in plain `--longhelp` (non-advanced) | 615 (41.7%) | dump 12 |
| — advanced-only (in 13/14, absent from 12) | 861 records / 855 names (**58.3%**) | dumps 12↔13 |
| Unique option names | 1470 (6 names × 2 modules each) | computed |
| Option records in `core program (core)` | 442 (191 basic + 251 advanced) | dump 14 |
| Option records in module sections (non-core) | 1034 across 187 sections | dump 14 |
| Module sections (unique module ids) | 124 (123) / 187 (179) / 187 (179) in dumps 12/13/14 | computed |
| vlcrc walk: sections / name=value lines / unique names | 191 / 1439 / 1433 | dump 16 |
| Types | string 519 · integer 420 · bool 267 · enum 162 · float 108 | computed |
| With declared range (min/max) / choices / short form | 235 / 162 / 15 | computed |
| Windows / macOS | **no counts — blocked-on-hardware** | skeletons only |

Advanced share is the key finding for our settings design: **58% of VLC 3.0.23's option
surface is hidden behind "advanced"** — our schema's `advanced` folding (doc 07 §5.2)
reproduces a real, dominant VLC UX pattern, not an edge case.

## 2. Which flags actually work on 3.0.23 (doc 02 §3.3 said [VERIFY flags])

| Flag | Verdict | Evidence |
|---|---|---|
| `--help` | works: 407-line usage + core non-advanced options only | dump 15 |
| `--longhelp` | works: basic options of modules + core (1516 output lines) | dump 12 |
| `--advanced --longhelp` | works: adds advanced options (3352 output lines) | dump 13 |
| `--full-help` (`-H`) | works: exhaustive option set + expanded long descriptions (5495 output lines); **byte-identical to `--advanced --full-help`** (diff = 0) | dump 14 + probe |
| `--help-verbose` | **NOT a dump flag on 3.0.23**: accepted, produces no stdout; without a stream it runs until killed (timeout exit 124); with `vlc://quit` exits 0 silently | dump 15 |
| `--reset-config` | forces a full vlcrc rewrite enumerating every option + default (a plain headless run writes NO file) | dump 16 preamble |

Doc-vs-public-wiki drift (observed, minor): the wiki says "`vlc --longhelp --advanced` will
give you all options" and "append `--help-verbose` … for more detailed help"
(wiki.videolan.org/Documentation:Command_line/, accessed 2026-10-19). Observed on 3.0.23:
the *option set* of `--advanced --longhelp` is indeed complete (1476 = `--full-help`), but
`--full-help`/`-H` additionally exists with expanded long descriptions (+2143 lines), and `--help-verbose`
added no output in our test. Dump footers (verbatim in the raw files): `--longhelp` hides
**67** advanced-only modules; `--advanced --longhelp` and `--full-help` still hide **4**.

## 3. Totals per category (Linux, `core program` section of dump 14)

| Core category | basic | advanced | total | Notable subsections (from the dump) |
|---|---|---|---|---|
| Audio | 8 | 10 | **18** | — |
| Video | 21 | 23 | **44** | Snapshot (7), Window properties (20) |
| Subpictures† | 17 | 50 | **67** | On Screen Display (3), Subtitles (6), Overlays (2), Track settings (11), Playback control (9), Default devices (2), Network settings (9), Socks proxy (3), Metadata (8), Advanced (14) |
| Decoders | 0 | 2 | **2** | — |
| Input | 1 | 3 | **4** | — |
| Stream output | 0 | 16 | **16** | VLM (8), Special modules (1), Plugins (3) |
| Miscellaneous | 0 | 6 | **6** | — |
| Playlist | 24 | 11 | **35** | Performance options (2) |
| Hot keys | 120 | 130 | **250** | Jump sizes (74) |
| **core total** | **191** | **251** | **442** | |

† The dump's `Subpictures` header scope runs until the next category header, so its later
subsections hold what the Qt UI groups under Input/Codecs (see `structure_notes` in
options-linux.json). Non-core module sections carry no category; the biggest option
families live there: x264 (76), x26410b (76), v4l2 (45), avcodec (41), dtv (40),
stream_out_transcode (26), mux_ts (24), stream_out_rtp (22), freetype (18), imem (16).

The card's expected headers ("Main", "Audio", "Video", "Subtitles", "Input", "Decoders",
"Stream output", "Playlist", "Interfaces", "Miscellaneous") do not literally match 3.0.23:
there is no "Main" header (the core section is ` core program (core)`), "Subtitles" is
"Subpictures", and there is no "Interfaces" header (interface options live in their own
module sections: rc, lua, oldrc, telnet…). Recorded in `structure_notes.card_assumption_delta`.

## 4. Three-way reconciliation (doc 02 §3.3)

Extraction 1 = help dumps (12/13/14); extraction 2 = vlcrc walk (16); extraction 3 =
Qt preferences UI tree — **blocked on hardware** (no display; transcription deferred to the
desktop-stack runbook, hardware-template.md §5; 0.B3's `ui-inventory.md` covers the
headless-observable hotkey subset). Every difference between 1 and 2:

| # | Difference | Count | Annotation |
|---|---|---|---|
| D1 | In vlcrc but not in `--full-help` | **0** | perfect name-set containment |
| D2 | In `--full-help` but not vlcrc | **37** | (a) CLI/meta-only flags not persisted (16): help, longhelp, full-help, help-verbose, version, list, list-verbose, module, config, quiet, verbose, ignore-config, reset-config, reset-plugins-cache, started-from-file, color; (b) pointer/callback/handle options not persistable (13): imem-get/release/data/cookie, sout-smem-* (6), drawable-xid, shm-file, shm-id; (c) remainder for [VERIFY] (8): cdda-track, cdda-first-sector, cdda-last-sector, http-continuous, http-referrer, sub-text-scale, lua-sd, extractor-flatten — likely runtime/context-only or non-persisted module options |
| D3 | Option names colliding across modules (flat CLI namespace) | **6** | scaletempo-stride/-overlap/-search (scaletempo_pitch ∥ scaletempo), rc-host (lua ∥ oldrc), glconv (gles2 ∥ gl), wl-display (wl_shell ∥ xdg_shell) — **the same 6 names also appear twice in the vlcrc walk**: cross-extraction agreement |
| D4 | Help `(default X)` vs vlcrc `name=X` disagreements | **0 / 252** | all 252 options declared in both agree after enabled/disabled→1/0 normalization |
| D5 | Defaults only in vlcrc (no help marker) | 1187 | help marks defaults mostly for bools (267 markers); vlcrc is the authoritative default source — our schema seeds defaults from vlcrc-style values |
| D6 | `--longhelp` vs `--advanced --longhelp` | +861 records | the advanced folding (see §1) |
| D7 | `--advanced --longhelp` vs `--full-help` option set | 0 difference | identical 1476; full-help adds long descriptions only (+2135 lines) |

**Verdict: no unexplained differences** between extractions 1 and 2 (D2's 9-item remainder
is the only open review item, marked [VERIFY] in options-linux.json). Doc 02 §3.3's
"unexplained differences are Phase 0 defects" test therefore passes on the two runnable
extractions; extraction 3 remains hardware-blocked.

## 5. Mapping to our settings-schema seed categories (doc 07 §5.4's 13 areas)

Matrix-scoping rule respected: individual options are NOT matrix rows; this table maps
VLC categories/option-clusters to our seed areas to feed the Phase 1 schema (doc 07 §5).

| VLC category / cluster (counts) | Our area | Notes |
|---|---|---|
| Hot keys: `--key-*` (250 core records; 222 key-/global-key- records, 106 with non-empty vlcrc defaults) | **controls** | 0.B3 already generated `keymaps/default.toml` from the same dumps; `--hotkeys-{x,y}-wheel-mode` → `controls.mouse-wheel-action` |
| Video: deinterlace/-mode, aspect-ratio, crop, zoom, align (44) | **video** | direct seed for doc 07 §5.4 video entries |
| Video: Snapshot subsection (7) | **recording** (+ ui) | snapshot-path/prefix/format ↔ recording.dir/format analog |
| Audio: volume-step, gain, replay-gain family, stereo-mode, audio-desync (18) | **audio** | aout/audio-filter/audio-visual → engine module selection, see "engine-internal" below |
| Subpictures: Subtitles + OSD subsections (spu, sub-*, osd, text-renderer) | **subtitles** | sub-text-scale ↔ subtitle.font-size percent |
| Subpictures§: Track settings (audio-language, sub-language, sub-track…) | **playback** | doc 07's playback.preferred-{audio,subtitle}-languages |
| Subpictures§: Playback control (start-time, stop-time, run-time, rate, input-fast-seek, input-repeat) | **playback** | seek-step seeds are hotkey-side (Jump sizes) |
| Playlist: random, loop, repeat, play-and-exit/-stop/-pause, start-paused, autostart, cork, one-instance, media-library, recursive, ignore-filetypes, show-hiddenfiles, services-discovery | **playback** + **library** | media-library/recursive/ignore-filetypes/show-hiddenfiles/services-discovery → library (watch-folders/scan); queue behaviors → playback.autoplay-next etc. |
| Subpictures§: Network settings + Socks proxy (http-host/port, socks*, mtu, ipv4-timeout) | **network** | plus http module options (http-proxy…, [VERIFY per-module]) |
| Subpictures§: Advanced + clock/caching cluster (file/live/disc/network-caching, clock-synchro, clock-jitter, cr-average) | **performance** | network-caching ↔ network.timeout-ms; performance.cache-override seed |
| Subpictures§: input-record-path/-native, timeshift | **recording** | recording.dir/format/include-subtitles seeds |
| Decoders + core codec/encoder (2+2) | **performance** (hw-decode-policy gate) / engine-internal | avcodec family (41 opts) is codec-tuning → engine-internal |
| Miscellaneous: plugins-cache/scan, keystore, rt-priority/offset, vod-server | engine-internal (not a user setting) | keystore → privacy-adjacent secret storage [PROPOSAL: revisit] |
| Input: access/demux/stream-filter/demux-filter, Stream output: sout* (16) + all sout-module sections (udp/http/rtp/transcode/…) | **recording** (sout record/display subset) / otherwise engine-internal | transcode/muxer/AV-family (x264 76, mux_ts 24, …) not user settings |
| Playlist: verbose, daemon, pidfile, color, advanced, interact, stats, intf/extraintf/control, dbus, started-from-file | **global** (a few: stats→privacy.usage-stats analog) / otherwise engine-internal or not-adopted (daemon/pidfile: server concerns, not a media-player setting) | |
| Lua `--lua-*`, services-discovery modules | **extensions** (partial) | VLC's extension model ≠ our WASM model — semantics, not options, carry over |
| Hotkeys: `--global-key-*` (111) | controls (global scope variant) | |
| Privacy-relevant VLC counterparts | **privacy** | thin in this dump: `--stats` (usage stats), `--metadata-network-access` (network fetch of metadata), keystore; update-check/crash-report options live in the qt family absent from the headless install [VERIFY desktop-stack] |

§ Recorded under the dump's broad `Subpictures` category (see §3 note); subsection used for mapping.

## 6. G4 evidence statement

**G4 (normalized option/preference records with annotated diffs) is COMPLETE for Linux**:
1476 normalized records in `options-linux.json`, every field either verbatim from the dumps
or computed from dump set-differences (`is_advanced`, `visibility`, `vlcrc_default`); parse
coverage 615/1476/1476 records vs 615/1476/1476 raw option lines; leftovers are footer notes
only (3/2/1, quoted verbatim in the JSON); three-way reconciliation in §4 with zero
unexplained diffs between the two runnable extractions. Windows/macOS are documented
blocked-on-hardware skeletons (`status: "blocked-on-hardware"`, empty `options` arrays,
citable expected-difference fields only) — G4 closes for those platforms when the
hardware-template.md §3/§4 runbooks execute.

## 7. [VERIFY] ledger (open items)

| # | Item | Owner/when |
|---|---|---|
| V1 | D2 remainder: 9 options in full-help but not vlcrc (cdda-track, http-referrer, sub-text-scale, lua-sd, extractor-flatten, http-continuous, cdda-first/last-sector) — confirm non-persistence is intended behavior | Phase 0 defect review or desktop-stack run |
| V2 | The 4 modules hidden even from `--full-help` ("only have advanced options" footer) — identify them (not derivable from help output alone) | desktop-stack run ([VERIFY-blocked-on-hardware-desk]) |
| V3 | Qt/skins2/notify/jack/svg/fluidsynth/visualization/video-splitter/access-extra option families absent from the Linux inventory (excluded packages) — full census needs the 15-package desktop install | hardware-template.md §5 |
| V4 | Windows/macOS full dumps (option sets, defaults, per-platform module families) | hardware-template.md §3/§4; replaces the skeletons |
| V5 | Qt preferences UI tree transcription (doc 02 §3.3 extraction 3) — display required | hardware run; cross-check vs wiki.videolan.org/Preferences/ (live 2026-10-19) |
| V6 | `--help-verbose` semantics (wiki says "more detailed help"; observed no output change) — retest on desktop stack | desktop-stack run |
| V7 | `sub-text-scale` etc. defaults live only in vlcrc (D5: 1187 options) — our schema generator should treat vlcrc-style dumps as the default-value source, not help text | doc 07 §5.3 G2 implementation |
