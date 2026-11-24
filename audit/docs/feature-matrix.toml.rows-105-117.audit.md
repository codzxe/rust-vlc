# CLAIM AUDIT: docs/feature-matrix.toml#rows-105-117
| Field | Value |
|---|---|
| Auditor | W18-A06 |
| Date | 2026-02-11 |
| Matrix rows in chunk | 13 (rows 105-117: playback-op 105-114, audio-feature 115-117) |
| REAL | 3 (113, 114, 117 — incl. 1 executed) |
| REAL-UNPROVEN | 2 (111, 112) |
| SHALLOW | 2 (106, 107) |
| STUB | 0 |
| PAPER-ONLY | 0 |
| FALSE | 6 (105, 108, 109, 110, 115, 116 — status claims contradict code) |
| UNREACHABLE | 0 (row-level; 6 rows cite ≥1 unreachable evidence file) |
| Internal contradictions found | 2 (web=partial vs web=not_started split; vocabulary "partial means a documented subset works" vs unreachable code) |

## 1. Blunt verdict
6 of 13 rows claim "partial" or "implemented" for features that NO shipped
entry point serves. The single host that matters — both the CLI and the
desktop shell, which routes verbatim into the CLI's dispatcher
(apps/desktop/src/dispatch.rs:218-219) — answers `playback.step-frame`,
`tracks.set-*`, `audio.set-delay`, `media.get-chapters`, `media.*-bookmark`,
`video.take-screenshot`, `video.set-fullscreen`, `audio.set-volume` with the
typed refusal `UnsupportedForHost` (apps/cli/src/dispatch.rs:81-155 route
table + `other => Err(unserved(other))`). The evidence strings cite real,
tested, sometimes excellent code — but it is engine/UI-layer code that no
shipped host can trigger; the only "working" volume/bookmark/chapter behavior
lives in `DemoTransport` (ui/src/client/demo-transport.ts:1 "the FRIENDLY
simulated engine for the BROWSER DEV SHELL"). Six "partial" statuses say "a
documented subset works"; the code says nothing works. That is status fraud
on the parity ledger. The two honest rows (113 record, 117 compressor: all
platforms not_started, evidence empty) and one genuinely-real row (114
format-detection, executed in the baseline smoke) prove the file CAN be
honest — the rest of this chunk is not.

## 2. Claim ledger (chunk-level)
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| M-01 | Header: "partial a documented subset works" (status vocabulary) | feature-matrix.toml:51-52 (vocabulary block) | baseline rule | Used below to grade each row's statuses. |
| M-02 | Desktop serves the same narrow command set as the CLI | verified fact | — | apps/desktop/src/dispatch.rs:218-219 `lumen_cli::dispatch::lumen_dispatch(&mut shell.host, request)`; `rg "set-volume|step-frame|set-chapter|add-bookmark|take-screenshot|set-fullscreen|set-delay" apps/` -> zero matches. |

## 5. Matrix rows (per template §11.5)
| Row | id | Statuses claimed | Evidence strings (paths) | Code found? | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 105 | playback-frame-step | partial x4 (windows/linux/macos/**web**) | frame_step.rs; catalog playback.step-frame; keymaps frame-next '.'/frame-prev ','; ui demo-transport step-frame handler | YES — all exist (frame_step.rs 169 lines, tests; keymaps/default.toml:94-95,68; ids.rs:108). BUT unreachable: W03-A10 proved `plan_frame_step` called only by its own tests; CLI route table unserves `playback.step-frame` ("rate and frame stepping are card 3.A1", dispatch.rs:199-204); I re-verified: no handler in any apps/ dir. | NO — no host serves it; notes admit "decode-exact landing … is the honest v0 limit" | FALSE — "partial" on 4 platforms incl. web (web has zero work; other rows in this very chunk use web=not_started). Nothing works anywhere. |
| 106 | playback-track-switch | implemented x3 (w/l/m), web not_started | tracks.rs; keymap cycles b/v/Alt+V → tracks.set-* | YES — tracks.rs (593 lines, REAL per prior audit: default_track reachable at open; switch flow engine-reachable via SessionHandle); keymaps/default.toml:147; UI context.tsx selectTrack dispatches tracks.set-*. | NO — tracks.set-* → `unserved` ("track switching is card 3.A3", dispatch.rs:199-201); UI→tauri→CLI dispatch refuses. `tracks.list` IS served (TracksList, dispatch.rs:137). | SHALLOW — engine+UI code real; "implemented on windows/macos" has no platform code/CI (notes admit "CI 3-OS pending"); end-to-end switch refused in every shipped host. |
| 107 | playback-av-sync | partial x3, web not_started | catalog ids.rs audio.set-delay/video.set-delay/subtitle.set-sync; keymap J/K/G/H + Shift+H/J/K + Ctrl+Shift+K | YES — ids.rs:123/129/133; keymaps/default.toml:149-150,188-189. Notes admit "engine A/V sync + queues (3.B) deliberately NOT in crates/playback-engine/src/lib.rs yet". | NO — audio.* → `unserved` ("audio device control joins with the real backends (ADR-005)", dispatch.rs:192-194). | SHALLOW — catalog-and-keymap paper surface; no host, no engine; "partial" overstates (nothing works). Notes are honest. |
| 108 | playback-chapter-title | partial x4 (incl. web) | chapters.rs; catalog media.get-chapters/media.set-chapter; demux/src/mkv/chapters.rs; demo-transport handlers | YES — chapters.rs (236 lines) REAL functions; mkv/chapters.rs (875 lines) real parser; ids exist. BUT chapters.rs UNREACHABLE: zero production callers (prior audit); CLI refuses `seek chapter N` typed ("no chapter {index}: AVI v0 carries no chapter markers", apps/cli/src/host.rs:1080-1086) and reports `chapters: []`. | NO — baseline mp4-smoke MediaInfoReady shows `chapters: []`. | FALSE — partial x4 incl. web; the UI handlers are DemoTransport mocks only. |
| 109 | playback-bookmarks | partial x4 (incl. web) | bookmarks.rs; catalog media.get-bookmarks/add/remove; ui BookmarksDialog.tsx + tests; demo-bookmarks.test.ts; keymap Ctrl+B | YES — all files exist; BookmarksDialog REAL-UNPROVEN per W15-A04 ("feature dead in every real host"); bookmarks.rs UNREACHABLE (zero callers; prior audit: the live feature is a TS reimplementation in demo-transport.ts). | NO (UI logic only via vitest against the mock; no host serves media.*-bookmark — rg apps/ zero). | FALSE — partial x4 incl. web; the only "live" bookmarks are the browser-dev-shell simulation. |
| 110 | playback-snapshot | partial x4 (incl. web) | snapshot.rs (SnapshotFormat/plan + tests); catalog video.take-screenshot | YES — snapshot.rs (253 lines) REAL planning half; W03-A04: UNREACHABLE, execution-less, "the pixel-readback half the plan exists for does not exist"; zero consumers. | NO — video.take-screenshot unserved in every host. | FALSE — partial x4 incl. web; notes themselves admit "no pixel-readback seam — capture wiring is renderer/video-engine-owned work" — i.e., the user-visible half does not exist, so no subset "works". |
| 111 | playback-fullscreen | partial x3, web not_started | ui useFullscreen.ts; catalog video.set-fullscreen; F key seed | YES — useFullscreen.ts (79 lines) REAL, reachable: PlayerScreen.tsx:635-636/825 (rg-verified by me; W15-A06 "web-API path only, shell-command path not implemented"). Command id exists (ids.rs:125). | NO (desktop-tauri build fails at gdk-sys in sandbox — environment limit, UNVERIFIED); command unserved but the hook uses the Web Fullscreen API directly. | REAL-UNPROVEN — a real, UI-reachable subset exists (browser Fullscreen API); desktop-side execution blocked by sandbox. |
| 112 | playback-start-options | partial x3, web not_started | typed OpenOptions { start_time, start_paused, rate } (catalog types.rs) | YES — types.rs:242-252 verified. MORE than claimed: CLI parses `--start-time/--start-paused` (apps/cli/src/parse.rs:227-288) and 11+ container decoders consume `opts.start_time_us` (mp4/es/ts/wav/aiff/avi/ps/mkv/ivf/ogg/adts .rs). Notes claim "crates/playlists/src/mpd.rs is the only start_time consumer today" — FALSE: mpd.rs:169 consumes `availability_start_time` (a DASH manifest field), NOT OpenOptions.start_time. | NO (not executed by me; no built binary; baseline command-layer-play covers open→playing only). | REAL-UNPROVEN — a genuine subset works in the shipped CLI per code read; w/m statuses unsupported; notes contain a false consumer claim (understates the real wiring). |
| 113 | playback-record | not_started x4 | (empty) | — (by design) | — | REAL — honest row: evidence empty, statuses not_started, notes scope the 7.E2 work accurately. |
| 114 | playback-format-detection | implemented x3, web not_started | media-probe crate (lib.rs + report.rs + contract.rs + config.rs/error.rs; probe fixtures incl. garbage + truncated) | YES — all files exist; W04-A01: REAL, reachable from CLI open path (apps/cli/src/demux_host.rs:446 `media_probe::probe_stream`); garbage/truncation tests verified (crates/media-probe/tests/probe_io.rs:233-235, 263-266). | YES — orchestrator baseline: `lumen open sample.mp4` walks Idle→…→Playing→Stopped exit 0 (mp4-smoke.md); rc-smoke command-layer-play PASS. | REAL — the one row in this chunk whose status matches executed reality. |
| 115 | afeat-volume | partial x3, web not_started | audio-engine effects.rs (EffectChain Gain/Mute, "commit 8bd13ed"); "ui settings area volume slider + mute switch live via audio.set-volume/audio.set-mute + audio.get-volume hydrate, commit 0a8e70d"; "effects tests 86/0" | YES — effects.rs (415 lines) REAL DSP; W07-A08: UNREACHABLE — nothing outside tests constructs EffectChain; shipped path is AudioPipeline mixer per-track Volume (no command). UI dispatch exists (ui/src/screens/player/context.tsx:93 setVolume → audio.set-volume; ControlBar.tsx) — but location misstated (player ControlBar, not "settings area") and NO host serves the command (my rg over apps/: zero set-volume handlers; only DemoTransport responds). | NO — typed refusal in every real host; volume slider "works" only in the browser dev shell. | FALSE — partial claims a working subset; the runtime volume of a real session is not controllable by any shipped host. |
| 116 | afeat-equalizer | partial x3, web not_started | audio-engine effects/eq.rs (Eq3Band, "bit-exact 0 dB identity", analytic |H| tests, "commit 70b4276"); "ui pending sliders awaiting audio.set-eq" | YES — eq.rs (392 lines) REAL DSP per W7 prior audit (UNREACHABLE: no pipeline construction, no command, UI staged disabled); UI sliders verified inert-with-reason (ui/src/screens/settings/areas/effects.tsx:262-264 `data-pending-command="audio.set-eq"`). | NO — no path to execute: catalog has no audio.set-eq command id wired to a host; Eq3Band has no caller. | FALSE — partial x3; notes themselves admit 3-band not 10-band and inert UI; nothing works end-to-end. |
| 117 | afeat-compressor | not_started x4 | (empty) | — (by design) | — | REAL — honest row. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Rows 105/108/109/110: web = "partial" | Rows 106/107/111/112/115/116 in the same chunk: web = "not_started" | No web work exists for any of these rows (web evidence strings are UI/desktop files); web=partial is internally inconsistent inflation. |
| "partial a documented subset works" (vocabulary, matrix:51-52) | apps/cli/src/dispatch.rs:81-155 route table + `unserved()` | The vocabulary is the doc's own rule; rows 105/108/109/110/115/116 claim partial while every cited command is refused typed by the only shipped hosts (CLI; desktop delegates to it at apps/desktop/src/dispatch.rs:218-219). The statuses violate the file's own definition. |
| Row 112 notes: "crates/playlists/src/mpd.rs is the only start_time consumer today" | apps/cli/src/parse.rs:227-288 + apps/cli/src/containers/{mp4,es,ts,wav,aiff,avi,ps,mkv,ivf,ogg,adts}.rs | The CLI itself consumes --start-time in 11+ container decoders; mpd.rs:169 consumes `availability_start_time` (DASH field), not OpenOptions.start_time. The note is wrong in both directions. |

## 4. Fabrication / inflation indicators
- **Statuses without reachability**: 12 of 13 rows carry non-not_started
  statuses on windows/macos with zero Windows/macOS-specific code and an
  admitted "CI 3-OS pending — house convention" backfill. The xtask
  parity-report baseline computes 0/252 verified on every platform.
- **Demo-shell features counted as shipped**: rows 108/109's only live
  handlers are in `DemoTransport` — a browser dev-shell simulation
  (demo-transport.ts:1 "the FRIENDLY simulated engine for the BROWSER DEV
  SHELL"). Volume (row 115) likewise "works" only there.
- **Evidence strings are real paths** (all 20+ cited files exist — checked
  one by one), which makes the false statuses harder to catch: the evidence
  proves the code exists, not that it runs. Rows 105/108/109/110 cite prior
  audits' UNREACHABLE verdicts against the very same files.
- Commit-hash evidence ("commit 8bd13ed", "commit 0a8e70d", "wave-5 merge
  2d0941e") — UNVERIFIED: git is forbidden to agents; no way to confirm
  these hashes reference the cited changes.
- What is honest: rows 113/117 (not_started, empty evidence), row 114
  (implemented + executed in baseline), and most rows' `notes` fields — the
  notes routinely confess the gaps the `status` lines hide (e.g. 105
  "decode-exact landing … is the honest v0 limit"; 116 "UI sliders render
  inert-with-reason"). The fraud is concentrated in the status columns.

## 5.1 Severity roll-up for this chunk
- S0: 6 rows (105, 108, 109, 110, 115, 116) — status "partial" for
  features unreachable from every shipped entry point (doc asserts a
  working subset; code refuses typed).
- S2: 2 rows (106, 107) — "implemented/partial" surfaces that no host
  serves; engine/catalog code real.
- S3: 1 row (112) — false consumer note (mpd.rs), understating real wiring.
- Verified working: 1 row (114, executed via baseline mp4-smoke/rc-smoke).

## 6. Commands run
- sed -n on feature-matrix.toml rows 1533-1700; python3 row-indexer
  ([[feature]] blocks: 252 total; rows 105-117 at lines 1534-1696)
- ls / wc -l on all 12 cited evidence files (all exist; sizes recorded)
- rg over apps/{cli,control-server,desktop,desktop-tauri,web} for the 8
  cited command ids -> zero host handlers
- rg keymaps/default.toml (frame-next/frame-prev :68,94-95; audio-track-cycle
  :147; audio-delay J/K :149-150; subtitle-delay H/G :188-189; fullscreen F
  :154; bookmarks Ctrl+B :224, set/play-bookmark-1..10 :235-248)
- rg crates/commands/src/catalog/ids.rs (:108-158 all ids exist)
- rg ui/src (context.tsx:93 setVolume; effects.tsx:262-264 inert EQ;
  PlayerScreen.tsx:635-636/825 useFullscreen)
- prior-wave audits cited: W03-A10 (frame_step), W03-A04 (snapshot),
  W07-A08 (effects), W07-A06 (eq), W15-A06 (useFullscreen), W15-A04
  (BookmarksDialog), W04-A01 (media-probe lib.rs), tracks/chapters/bookmarks
  audits (reachability verdicts quoted above)
- baseline cited: mp4-smoke.md (open exit 0), rc-smoke.txt:133
  (command-layer-play PASS)
