# Feature-matrix population log — Task 0.D2 (Phase 0 synthesis)

- **Card**: 0.D2 (doc 13 §3, wave 0.D) — freeze `docs/feature-matrix.toml` schema + initial
  population from the completed inventories.
- **Date**: 2026-10-19. **Deliverables**: `docs/feature-matrix.toml` (252 rows), this log.
- **Sources (all read, none invented)**: `docs/inventory/formats.json` (82 families),
  `modules-linux.json` (502 registrations) + `modules-summary.md`, `options-linux.json`
  (1,476 options) + `options-summary.md`, `ui-inventory.md` (8 menus / 58 leaf items /
  23 dialogs / 111 hotkey actions), `keymaps/default.toml` (141 bindings),
  `platform-{windows,macos,linux}.md` + `platform-synthesis.md`, and the feasibility
  verdict tables in `docs/feasibility/sections/{video-codecs,audio-codecs,subtitles,subsystems}.md`.
- **Statuses**: nothing is `verified` or `implemented` — no CI exists (R4 honesty).
  211 rows start `not_started` on all four platforms; 33 rows carry `blocked` on
  at least one platform; 8 further rows are `unsupported`-only (by-design or
  platform-specific: 3 per-OS aout rows, 2 web-only rows, skins2, VLM, Lua).

## 1. Counts

**Total rows: 252.** By priority: **P0 61 (24.2%) · P1 92 · P2 60 · P3 39**.
Phase-target distribution: ph2 11 · ph3 5 · ph4 93 · ph5 48 · ph6 18 · ph7 25 · ph8 52.

| Category | rows | P0 | P1 | P2 | P3 | any-blocked |
|---|---|---|---|---|---|---|
| container | 21 | 9 | 6 | 3 | 3 | 0 |
| video-codec | 16 | 4 | 8 | 1 | 3 | 5 |
| audio-codec | 19 | 6 | 5 | 3 | 5 | 9 |
| subtitle-format | 17 | 2 | 4 | 8 | 3 | 2 |
| text-render | 3 | 1 | 0 | 1 | 1 | 0 |
| playlist-format | 7 | 3 | 1 | 2 | 1 | 0 |
| protocol-access | 11 | 3 | 2 | 3 | 3 | 8 |
| network-feature | 7 | 2 | 4 | 1 | 0 | 0 |
| playback-op | 13 | 7 | 5 | 1 | 0 | 0 |
| audio-feature | 11 | 1 | 4 | 6 | 0 | 0 |
| video-feature | 10 | 1 | 4 | 4 | 1 | 1 |
| playlist-feature | 9 | 2 | 3 | 3 | 1 | 0 |
| config-feature | 20 | 2 | 15 | 3 | 0 | 0 |
| plugin-feature | 4 | 0 | 2 | 1 | 1 | 0 |
| platform-feature | 18 | 2 | 9 | 5 | 2 | 3 |
| services-discovery | 6 | 0 | 0 | 2 | 4 | 3 |
| ui-screen | 11 | 3 | 6 | 1 | 1 | 0 |
| ui-dialog | 15 | 2 | 3 | 4 | 6 | 1 |
| menu-item | 16 | 5 | 6 | 4 | 1 | 0 |
| hotkey | 17 | 6 | 5 | 4 | 2 | 1 |
| extension | 1 | 0 | 0 | 0 | 1 | 0 |
| **total** | **252** | **61** | **92** | **60** | **39** | **33** |

P0/P1 for format rows come **verbatim from formats.json `usage_share`** (P0×27 /
P1×23 / P2×19 / P3×13 across its 82 families). Non-format priorities follow the
doc 02 §4.4 rubric (P0 = "breaks the usable-player claim": transport, seek, volume,
open dialogs, settings, rebrand, aout/vout, aspect) and are flagged
**[VERIFY] reasoning** in row notes where judgment was applied
(e.g. `subtitle-forced`, `codec-audio-census-tail`, `playback-start-options`).

## 2. Schema freeze + category reconciliation

**Schema frozen 2026-10-19** per doc 02 §4.7: fields, status vocabulary, platform
columns, evidence rules and validation rules R1–R10 are now Lead-change-control
only (schema_version bump + migration note here). Rows remain append-mostly.

Doc 02 §4.2's draft category enum is reconciled to the 0.D2 card's taxonomy as
follows (frozen categories = left column of the matrix):

| doc 02 §4.2 category | frozen category (id prefix) | note |
|---|---|---|
| container | `container` (`container-`) | unchanged |
| video-codec / audio-codec | same (`codec-video-` / `codec-audio-`) | unchanged |
| subtitle-codec | `subtitle-format` (`subtitle-`) | renamed (card) |
| playlist-format | `playlist-format` (`plfmt-`) | unchanged |
| protocol + access | `protocol-access` (`protocol-`) + `network-feature` (`network-`) | scheme handlers vs streaming stacks (HLS/DASH/TLS/reconnect/auth) |
| ui | `ui-screen` / `ui-dialog` / `menu-item` | card's split; prefixes `ui-screen-`, `ui-dialog-`, `menu-` |
| — (new) | `playback-op` (`playback-`) | engine playback operations (doc 05 §2 catalog) |
| audio-filter | `audio-feature` (`afeat-`) | includes non-filter features (volume, device switch) |
| video-filter | `video-feature` (`vfeat-`) | includes crop/aspect/zoom geometry (not VLC "filters") |
| — (new) | `playlist-feature` (`plfeat-`) | queue behaviors vs parsers |
| — (new) | `config-feature` (`config-`) | settings areas/profiles/rebrand/l10n |
| — (new) | `plugin-feature` (`plugin-`) | registry/WASM/kinds |
| device + audio-output + video-output | `platform-feature` (`platform-`) | per-OS statuses do the work doc 02's per-OS categories did |
| text-render / services-discovery / hotkey / extension | unchanged (`textrender-`, `sd-`, `hotkey-`, `ext-`) | kept from doc 02 |

Full category→prefix map (R1) is enforced by the generator validation; doc 02's
`aout-`/`vout-`/`vfilter-`/`afilter-`/`access-`/`device-` prefixes are retired in
favor of the platform/feature prefixes above (migration note = this table).

## 3. Mapping decisions (how inventories became rows)

1. **formats.json 82 families → 91 format-side rows.** 1:1 for most families; splits:
   `adaptive-hls-dash` → `network-hls` + `network-dash`; `disc-family` →
   `platform-disc-{dvd,bluray,cdda}`; `ssa`+`ass` → one `subtitle-ass-ssa`;
   `dvb-subs-teletext` → `subtitle-dvb-subs` + `subtitle-teletext` (blocked);
   `h264`/`hevc` → decode row + `-software` row (feasibility's "matrix row says"
   column prescribes `h264.hw` vs `h264.software`); `pcm-siblings-census` → folded
   araw/lpcm/adpcm into their rows, remainder into `codec-audio-census-tail`;
   `subtitle-census-tail` split out vobsub/kate/forced/autoload/encoding rows.
   Additions beyond families: `container-ivf` (doc 13 card 5.I3 names it; census-level).
2. **options-linux.json 1,476 options → 20 config-feature rows + engine/feature rows.**
   Matrix-scoping rule (options-summary §5): individual options are never rows.
   442 core options map via the 13 seed areas (one row each, `_config_areas` loop)
   + rebrand/profiles/hotkey-editor/keymap-layers/l10n/advanced-folding/defaults-source.
   Other option clusters became playback-op / audio-feature / video-feature /
   playlist-feature rows (e.g. Playback-control subsection → `playback-start-options`;
   Snapshot subsection → `playback-snapshot`; replay-gain family → `afeat-replaygain`).
   All `options-linux.json#…` anchors in the matrix were programmatically verified
   to exist in the inventory (0 misses).
3. **modules-linux.json 502 registrations → vlc_modules arrays + rollup rows.**
   Format rows carry their observed modules from formats.json/census. Capability
   families without v1 scope became rollup rows: `container-census-long-tail`,
   `codec-video-legacy-blocked`, `codec-audio-census-tail`, `subtitle-census-tail`,
   `plfmt-census-tail`, `protocol-census-tail`, `sd-census-tail`,
   `vfeat-novelty-filters`, `textrender-subsource-overlays`. The packetizer gap
   (25 registrations, no v1 kind) is recorded on `plugin-codec-api`.
4. **ui-inventory.md: 8 menus / 58 leaf items → 16 menu-item rows** (grouped per
   card: e.g. one "Media menu: open family" row for 7 leaf items). **23 dialogs →
   15 ui-dialog rows** (merged: Media+Codec info; Preferences→ui-screen-settings;
   Adjustments→ui-screen-av-tools; Customize→ui-screen-customize; snapshot has no
   dialog). **111 hotkey actions → 17 hotkey group rows** partitioning the actions
   exactly: transport 6 + seek 9 (8 jumps + position) + rate 5 + volume 3 +
   frame-step 1 + track 4 + sync 8 + video-display 23 + subtitle-scale 5 +
   playlist 3 + bookmarks 20 + navigation 7 + nav-disc 5 + snapshot-record 2 +
   interface 4 + wallpaper-360 5 = 110, plus audio-device-cycle (1) sitting on
   `afeat-audio-device-switch` = 111 total; the 111 `global-key-*` actions are one
   separate scope row (`hotkey-global`). VLC-side [VERIFY] markers from ui-inventory are
   carried in row notes.
5. **Feasibility verdict tables → phase targets + blocked statuses** (see §5).
   Doc 13's wave program supplies phase numbers (2.A–2.E → ph2; 3.A/3.B → ph3;
   4.* → ph4; 5.* → ph5; 6.* → ph6; 7.* → ph7; later/post-v1 → ph8 parking).

## 4. Not-adopted v1 (explicit wont-do; recorded, not hidden — MP §14)

Rows exist for most of these (statuses honest); items with **no row at all** are
listed here with reasons:

- **Stream-output family beyond the 7.E subset** (46 registrations: sout stream 23,
  sout mux 11, sout access 10, vod server 2): we are a player, not a streaming
  server (MP scope). v1 = PCM encode + lossless MP4 remux + file/pipe sink (3/46,
  ~7% — feasibility subsystems §6). Covered by `playback-record` + `ui-dialog-convert`
  + `ui-dialog-stream-output` rows; network sout (rtp/chromecast/shout) not-adopted.
- **Transcode beyond WAV/MP4-remux** (x264 76 options, x26410b 76, mux_ts 24,
  stream_out_rtp 22, transcode 26…): encoder-side mirror of the decoder gap; x264-class
  is forbidden-territory C. Phase 7 encoders = WAV PCM + stream-copy only.
- **VLM/VOD** (`ui-dialog-vlm`, unsupported-by-design), **skins2** (`ui-screen-skins2`,
  unsupported-by-design), **Lua extensions** (`ext-lua-extensions`, WASM replaces).
- **Most services-discovery** (UPnP, mDNS renderers, SAP, MTP, xcb_apps, pulselist):
  blocked/later rows exist (`sd-*`); minimal-SD recommendation per feasibility §5.2.
- **Capture** (dshow/v4l2/DV/DVB/decklink): hardware-gated, no pure path, untestable
  in CI (`platform-device-capture` blocked).
- **Acoustid fingerprinting, addons store** (`plugin-addon-store` P3 later),
  **mosaic/remoteosd/dynamicoverlay/audiobargraph** overlays (P3),
  **novelty video filters ~90** (`vfeat-novelty-filters` P3),
  **DirectX wallpaper** (Windows-only novelty, not-adopted; noted on
  `playback-fullscreen` + `hotkey-wallpaper-360`), **skins-era theming** (replaced by
  `ui-screen-theme`).
- **Windows/macOS option+census skeletons** remain blocked-on-hardware (0.B1/0.B2
  ledgers) — expected diffs only, no fabricated rows.

## 5. Blocked list (33 rows with ≥1 `blocked`; reasons summarized)

Video codecs: `h264-software` + `hevc-software` (no proven pure-Rust decoder; young
2026 candidates are conformance-gated pending ADR-001; ph5), `theora` (no decoder,
L self-build later), `wmv-vc1` (macos only: no VT exposure, no pure sw), 
`legacy-blocked` (Indeo/RealVideo/Cinepak/Hap/Dirac — patent haze, doc 14 A5).
Audio codecs: `he-aac` (zero upstream SBR/PS — detect+report, never silent),
`opus` (empty placeholder crate — the R-06 headline; decision 5.K2-I), `ac3-eac3`
(self-build L in Phase 6), `dts` (spec availability itself [VERIFY]), `wma`
(absent upstream entirely), `wavpack` (GPL-3.0 wrinkle → doc 11), `speex`,
`musepack`, `midi-synth` (rustysynth = later path; oxisynth LGPL out-of-tree).
Subtitles: `teletext` (zvbi C), `kate` (no crate; μADR drop-v1).
Protocols (6.G1-I registry): `udp`, `rtp`, `rtsp` (pure crates exist — retina
0.4.21 — but hw-only payload + async island + camera-quirk tail), `smb` (C; absent
from pinned census), `nfs`, `ftp-sftp`, `mmsh-mms`, `protocol-census-tail`.
Filters: `deinterlace` (heavy DSP + hw-coupled; Phase 6+ decision card [PROPOSAL]).
Devices/discs: `capture` (hardware-gated), `dvd` (libdvdnav/CSS legal — ADR-008),
`bluray` (libbluray C; m2ts fallback [PROPOSAL]). SD: `upnp`, `mdns-renderers`,
`sap` (minimal-SD recommendation). UI: `open-capture` dialog (blocked with capture).
Hotkeys: `nav-disc` (blocked with disc navigation; arrows repurposed per C-01).

Plus 8 unsupported-only rows: per-OS aout rows (wasapi/coreaudio/pipewire),
web-only rows (webcodecs, web-target), skins2, VLM, lua-extensions — each with a
by-design/platform reason in `blocked_reason`.

## 6. Open questions for the Lead

1. **Q1 — R2's P0 ⇒ phase ≤ 6 vs adapter timing.** Resolved here by splitting:
   codec rows (P0, ph5 = 5.F2/5.G2 integration) vs platform hw-adapter rows
   (P1, ph7 = 7.B1–7.B3). Confirm the split, or bless P0 rows at ph7.
2. **Q2 — category rename fallout.** The frozen taxonomy (card) renames/splits
   doc 02 §4.2's enum (subtitle-codec→subtitle-format; ui→3 categories; new
   playback-op/playlist-feature/config-feature/plugin-feature; device+outputs folded
   into platform-feature). Doc 02 §4 needs a pointer edit at next revision
   (not edited here — scope discipline; same pattern as ui-inventory Q7).
3. **Q3 — `not-selected` status.** Doc 13's card-status is documented in the matrix
   header as NOT a matrix status (wont-do = `unsupported`/`blocked` + reason).
   Confirm, or ratify `not-selected` as an 8th matrix value (schema bump).
4. **Q4 — blocked rows with phase targets.** Convention frozen: phase_target =
   revisit phase (e.g. rtsp ph6 = 6.G1-I registry pass; dts ph8 = post-v1 parking).
   Confirm that blocked+ph8 reads as "later/post-v1", not "planned Phase 8".
5. **Q5 — P0 share 24.2%.** Formats.json contributes 27 P0 families; UX/platform
   rows raise P0 to 61/252. Confirm the rubric application (volume/seek/aspect =
   P0; EQ/compressor = P1) before Phase 5 gating uses P0-only parity.
6. **Q6 — vfeat-deinterlace blocked vs 4.E3 settings AC** ("Video (display,
   deinterlace)" group-2 screen): the disabled-with-reason rendering is the
   designed outcome — confirm no re-prioritization to Phase 5/6 is wanted.
7. **Q7 — mp4/avi decision-pending.** Both rows sit at ph2 with the [DECISION-PENDING]
   note; the unselected variant's row will need a phase move to 5 (5.J1/5.C1) when
   the Lead decides at wave-2.B launch (append-mostly edit, not a schema change).

## 7. Validation performed at population time

Generator (`tmp/d2/gen_matrix.py`, gitignored scratch) + `tomllib` round-trip:
252 rows parse; ids unique + regex + prefix↔category; priorities/phases in range;
status keys exactly {windows,linux,macos,web} with vocabulary values; zero
`verified`/`implemented` (no CI); every blocked/unsupported row has a
`blocked_reason`; **P0 ⇒ ph ≤ 6 holds (0 violations)**; all
`inventory/options-linux.json#*` anchors exist in options-linux.json (0 misses);
all `inventory/formats.json#*` anchors exist in formats.json families (0 misses).
tools/feature-audit (1.E) will own these checks going forward (R1–R10).
