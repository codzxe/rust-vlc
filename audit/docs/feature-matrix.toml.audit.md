# CLAIM AUDIT: docs/feature-matrix.toml

| Field | Value |
|---|---|
| Auditor | W18-A01 (orchestrator) |
| Date | 2026-11-16 |
| Claims extracted (rows) | 252 |
| REAL (executed end to end by this audit) | 4 |
| REAL-UNPROVEN (suite-passed, not individually executed) | 11 |
| SHALLOW (path-existence standard only) | 52 |
| FALSE (implemented cells with dangling evidence) | 1 |
| PARTIAL (as claimed) | 45 |
| STUB/PAPER (blocked/unsupported/not_started as claimed) | 139 |
| Internal contradictions found | 2 dangling-evidence rows + the stale-notes row (see below) |

## 1. Blunt verdict

The matrix is structurally honest (statuses/vocabulary/conditionals all real, enforced by
feature-audit) but its `implemented` tier is certified to a path-existence standard that this
audit PROVED certifies a 0-byte file as full 4-platform evidence. Of 252 rows:
**4 REAL by execution** (I ran the feature end to end on this host),
**11 REAL-UNPROVEN** (the workspace suite I executed passed real
conformance/differential lanes over them, but I did not run each individually),
**52 SHALLOW** (implemented cells with path-existence evidence only),
**1 FALSE** (implemented cells citing evidence paths that do not exist)
and 184 rows claiming absence/partial
(default-true at sweep level; not re-derived file-by-file — coverage-limited run).

## 2. Row-level verdicts (all 252)

Full table: `audit/waves/wave-18/rows-verdicts.tsv`. Categories below list every row that
claims work (implemented/verified on any platform) with its verdict; absence-claim rows are
aggregated per category.

### audio-codec (19 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 38 | codec-audio-pcm | implemented | partial | implemented | not_started | REAL-UNPROVEN | conformance + rc-smoke fixture lane (avi_mjpeg_pcm) passed in the executed suite |
| 39 | codec-audio-mp3 | implemented | implemented | implemented | not_started | REAL-UNPROVEN | same — suite-passed, not individually executed |
| 40 | codec-audio-mp2 | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 41 | codec-audio-aac-lc | implemented | partial | implemented | not_started | REAL | same run — AAC audio limb decoded to end-of-media (null-device sink) — executed on linux; windows/macos cells = REAL-UNPROVEN (same code, not run here) |
| 43 | codec-audio-flac | implemented | implemented | implemented | not_started | REAL-UNPROVEN | codecs crate differential/suite tests passed in the executed workspace run; not individually executed by the orchestrator |
| 44 | codec-audio-alac | implemented | partial | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 45 | codec-audio-vorbis | implemented | implemented | implemented | not_started | REAL-UNPROVEN | same — suite-passed, not individually executed |
| 46 | codec-audio-opus | implemented | implemented | implemented | not_started | REAL-UNPROVEN | same (ruopus) — suite-passed, not individually executed |
| 50 | codec-audio-adpcm | implemented | implemented | implemented | not_started | REAL-UNPROVEN | same — suite-passed, not individually executed |

- Absence/partial claims (10): codec-audio-ac3-eac3, codec-audio-census-tail, codec-audio-dts, codec-audio-he-aac, codec-audio-midi-synth, codec-audio-musepack, codec-audio-speex, codec-audio-truehd-mlp, codec-audio-wavpack, codec-audio-wma — taken as claimed (default-true), not re-derived this run.

### audio-feature (11 rows)

- Absence/partial claims (11): afeat-audio-device-switch, afeat-compressor, afeat-equalizer, afeat-passthrough, afeat-replaygain, afeat-resampler, afeat-spatializer, afeat-stereo-mode, afeat-stereowiden, afeat-visualizations, afeat-volume — taken as claimed (default-true), not re-derived this run.

### config-feature (20 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 145 | config-area-global | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 146 | config-area-interface | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 147 | config-area-privacy | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 148 | config-area-playback | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 153 | config-area-library | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 154 | config-area-network | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 155 | config-area-performance | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 158 | config-rebrand | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 160 | config-hotkey-editor | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 162 | config-localization | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 163 | config-advanced-folding | implemented | implemented | implemented | not_started | FALSE | status claims implemented while evidence path(s) ['ui/src/screens/settings/areas/advanced.tsx'] do not exist (feature-audit R4 violation, executed exit 1) |

- Absence/partial claims (9): config-area-audio, config-area-controls, config-area-extensions, config-area-recording, config-area-subtitles, config-area-video, config-defaults-source, config-keymap-layers, config-profiles — taken as claimed (default-true), not re-derived this run.

### container (21 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 1 | container-matroska-webm | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 2 | container-mp4-mov-3gp | implemented | implemented | implemented | not_started | REAL | lumen open+advance on a real H.264+AAC MP4: tracks 2 ready, Playing→Ended, 100.0% time, exit 0 (mp4-smoke-open-advance.txt) — executed on linux; windows/macos cells = REAL-UNPROVEN (same code, not run here) |
| 3 | container-ogg | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 4 | container-mpegts | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 5 | container-mpegps | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 6 | container-avi | implemented | implemented | implemented | not_started | REAL-UNPROVEN | conformance avi lanes passed in the executed suite; rc-smoke default fixture |
| 7 | container-asf-wmv | implemented | partial | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 8 | container-wav | implemented | implemented | implemented | not_started | REAL-UNPROVEN | conformance wav lanes passed in the executed suite |
| 9 | container-flac-native | not_started | not_started | not_started | not_started | REAL-UNPROVEN | suite-passed; not individually executed |
| 10 | container-aiff | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 12 | container-adts-aac | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 13 | container-mp3-raw | implemented | implemented | implemented | not_started | REAL-UNPROVEN | suite-passed; not individually executed |
| 15 | container-h264-annexb | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 16 | container-hevc-annexb | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 17 | container-ivf | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |

- Absence/partial claims (6): container-archive-zip, container-au, container-census-long-tail, container-midi-smf, container-mpeg-es-video, container-realmedia — taken as claimed (default-true), not re-derived this run.

### extension (1 rows)

- Absence/partial claims (1): ext-lua-extensions — taken as claimed (default-true), not re-derived this run.

### hotkey (17 rows)

- Absence/partial claims (17): hotkey-bookmarks, hotkey-frame-step, hotkey-global, hotkey-interface, hotkey-nav-disc, hotkey-navigation, hotkey-playlist, hotkey-rate, hotkey-seek, hotkey-snapshot-record, hotkey-subtitle-scale, hotkey-sync, hotkey-track, hotkey-transport, hotkey-video-display, hotkey-volume, hotkey-wallpaper-360 — taken as claimed (default-true), not re-derived this run.

### menu-item (16 rows)

- Absence/partial claims (16): menu-audio, menu-context-playlist, menu-context-surface, menu-help, menu-media-convert-quit, menu-media-open, menu-playback-bookmarks, menu-playback-navigation, menu-playback-transport, menu-subtitle, menu-tools, menu-tray, menu-video-adjust, menu-video-display, menu-video-snapshot, menu-view — taken as claimed (default-true), not re-derived this run.

### network-feature (7 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 96 | network-tls-policy | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |

- Absence/partial claims (6): network-auth, network-dash, network-hls, network-http-client, network-reconnect-buffering, network-remote-playlists — taken as claimed (default-true), not re-derived this run.

### platform-feature (18 rows)

- Absence/partial claims (18): platform-aout-coreaudio, platform-aout-cpal, platform-aout-pipewire, platform-aout-wasapi, platform-desktop-integration, platform-device-capture, platform-disc-bluray, platform-disc-cdda, platform-disc-dvd, platform-hwdecode-av1, platform-hwdecode-h264, platform-hwdecode-hevc, platform-hwdecode-mpeg-family, platform-hwdecode-vp9, platform-vout-wgpu, platform-web-target, platform-web-webcodecs, platform-zero-copy-interop — taken as claimed (default-true), not re-derived this run.

### playback-op (13 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 102 | playback-play-pause-stop | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 103 | playback-seek | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 106 | playback-track-switch | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 114 | playback-format-detection | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |

- Absence/partial claims (9): playback-av-sync, playback-bookmarks, playback-chapter-title, playback-frame-step, playback-fullscreen, playback-rate, playback-record, playback-snapshot, playback-start-options — taken as claimed (default-true), not re-derived this run.

### playlist-feature (9 rows)

- Absence/partial claims (9): plfeat-autostart-behaviors, plfeat-media-library, plfeat-metadata, plfeat-queue-model, plfeat-recent-items, plfeat-repeat-loop-shuffle, plfeat-save-load, plfeat-service-discovery-panel, plfeat-sort-search — taken as claimed (default-true), not re-derived this run.

### playlist-format (7 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 78 | plfmt-pls | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 79 | plfmt-xspf | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 82 | plfmt-cue | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |

- Absence/partial claims (4): plfmt-asx, plfmt-b4s, plfmt-census-tail, plfmt-m3u-m3u8 — taken as claimed (default-true), not re-derived this run.

### plugin-feature (4 rows)

- Absence/partial claims (4): plugin-addon-store, plugin-builtin-registry, plugin-codec-api, plugin-wasm-extensions — taken as claimed (default-true), not re-derived this run.

### protocol-access (11 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 86 | protocol-http-https | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |

- Absence/partial claims (10): protocol-census-tail, protocol-directory, protocol-file, protocol-ftp-sftp, protocol-mmsh-mms, protocol-nfs, protocol-rtp, protocol-rtsp, protocol-smb, protocol-udp — taken as claimed (default-true), not re-derived this run.

### services-discovery (6 rows)

- Absence/partial claims (6): sd-census-tail, sd-directory-watch, sd-mdns-renderers, sd-podcast, sd-sap, sd-upnp — taken as claimed (default-true), not re-derived this run.

### subtitle-format (17 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 57 | subtitle-srt | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 59 | subtitle-vtt | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 64 | subtitle-pgs | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 65 | subtitle-dvb-subs | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 67 | subtitle-cc-608-708 | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 68 | subtitle-vobsub | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |

- Absence/partial claims (11): subtitle-ass-ssa, subtitle-autoload, subtitle-census-tail, subtitle-encoding-detection, subtitle-forced, subtitle-kate, subtitle-microdvd, subtitle-sami, subtitle-subviewer, subtitle-teletext, subtitle-ttml — taken as claimed (default-true), not re-derived this run.

### text-render (3 rows)

- Absence/partial claims (3): textrender-osd, textrender-subsource-overlays, textrender-subtitle-rasterizer — taken as claimed (default-true), not re-derived this run.

### ui-dialog (15 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 204 | ui-dialog-open-file | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 205 | ui-dialog-open-network | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 209 | ui-dialog-goto-time | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 210 | ui-dialog-bookmarks | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |

- Absence/partial claims (11): ui-dialog-about-update, ui-dialog-convert, ui-dialog-messages, ui-dialog-open-capture, ui-dialog-open-disc, ui-dialog-open-multiple, ui-dialog-plugins, ui-dialog-program-guide, ui-dialog-stream-output, ui-dialog-subtitle-add, ui-dialog-vlm — taken as claimed (default-true), not re-derived this run.

### ui-screen (11 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 193 | ui-screen-player | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 194 | ui-screen-playlist-library | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 195 | ui-screen-settings | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 196 | ui-screen-media-info | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 197 | ui-screen-onboarding | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 198 | ui-screen-miniplayer-music | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 199 | ui-screen-keyboard-shortcuts | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 200 | ui-screen-customize | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 202 | ui-screen-theme | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |

- Absence/partial claims (2): ui-screen-av-tools, ui-screen-skins2 — taken as claimed (default-true), not re-derived this run.

### video-codec (16 rows)

| # | id | win | linux | macos | web | verdict | reason |
|---|---|---|---|---|---|---|---|
| 22 | codec-video-h264 | partial | partial | partial | not_started | REAL | same run — H.264 software decode (rust_h264) to end-of-media + conformance mp4_h264_family_lane passed in the executed suite — executed on linux; windows/macos cells = REAL-UNPROVEN (same code, not run here) |
| 23 | codec-video-h264-software | implemented | implemented | implemented | not_started | REAL | same run — the avcC-seeded software decoder (pipeline_host.rs:328) decoded it; NOTE: this row's evidence string cites the missing path crates/codecs/src/video/h264/sw (feature-audit R4 violation, executed exit 1) — REAL feature, FALSE evidence string — executed on linux; windows/macos cells = REAL-UNPROVEN (same code, not run here) |
| 26 | codec-video-av1 | implemented | implemented | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 27 | codec-video-vp9 | implemented | partial | implemented | not_started | SHALLOW | implemented cells certified by path-existence evidence only (the empty-file experiment standard); no execution by this audit |
| 32 | codec-video-mjpeg | implemented | implemented | implemented | not_started | REAL-UNPROVEN | container_conformance avi_mjpeg_pcm lanes passed in the executed workspace suite; bench decodes mjpeg (bench allowlist's only video member) |

- Absence/partial claims (11): codec-video-ffv1, codec-video-hevc, codec-video-hevc-software, codec-video-legacy-blocked, codec-video-mpeg1-2, codec-video-mpeg4-asp, codec-video-prores-dnxhd, codec-video-raw-fourcc, codec-video-theora, codec-video-vp8, codec-video-wmv-vc1 — taken as claimed (default-true), not re-derived this run.

### video-feature (10 rows)

- Absence/partial claims (10): vfeat-aspect-ratio, vfeat-color-fx, vfeat-colors-adjust, vfeat-crop, vfeat-deinterlace, vfeat-hdr, vfeat-novelty-filters, vfeat-rotate-transform, vfeat-sharpen-gradfun, vfeat-zoom-scale — taken as claimed (default-true), not re-derived this run.

## 3. Contradictions

| This matrix says | Other source says | Which is true (proof) |
|---|---|---|
| row 23 (implemented) evidence `crates/codecs/src/video/h264/sw` | the path does not exist in the tree | feature-audit executed: R4 violation (exit 1); the h264 code actually lives at crates/codecs/src/video/h264/ — the evidence string is stale |
| row 163 evidence `ui/src/screens/settings/areas/advanced.tsx` | the path does not exist | feature-audit executed: R4 violation |
| notes field of container-mp4-mov-3gp row: 'No MP4 demuxer code exists' style stale text (per W05-A02) | mp4 demuxer is REAL and executed | stale notes field on an implemented row |

## 4. Fabrication / inflation indicators

- The evidence standard: empty-file experiment (PASS on a 0-byte 'decoder') — implemented is
  compatible with nothing existing behind the path.
- verified cells: ZERO in the whole matrix (verified-only parity 0/252, executed) while
  v0.1.0-v0.1.2 shipped publicly — releases carried implemented-tier, not verified-tier, claims.
- last_updated = 2026-11-09 (9 days before this audit's HEAD); PARITY-REPORT quotes older numbers still (64/252 vs live 66/252, W19-A01).

## 5. Method note

Verdicts assign per the DEVIL-AUDIT verdict scale with executed evidence only: E0 tier = the
orchestrator ran the feature end to end on this host (binary + real media); E1 tier = the
orchestrator executed `cargo test --workspace` (1001 passed) which includes real conformance
lanes over the row's feature. Platform cells: executions happened on linux; windows/macos
cells of otherwise-verified rows are REAL-UNPROVEN (same code path, not run on those OSes here).
Web column: 0/252 implemented (executed parity output) — the wasm plane is unbuilt here
