# CLAIM AUDIT: docs/feature-matrix.toml rows 131-143 (video-feature tail + playlist-feature head)
| Field | Value |
|---|---|
| Auditor | W19-A01 |
| Claims extracted | 31 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 0 / 0 / 2 (rows 133, 139) / 0 / 0 / 1 (row 142, understated) / 4 (rows 136, 137, 138, 141-demo-dead) — plus 6 ACCURATE-no-code rows (131, 132, 134, 135, 140, 143) |
| Internal contradictions found | 4 |

## 1. Blunt verdict
13 rows: 5 video-feature tail (131-135) + 8 playlist-feature (136-143). Nine rows are honest
absences — `not_started`/`blocked` with empty evidence, and the rg census confirms the engine
code does not exist. The rot in this chunk is UNDERSTATEMENT, the same disease W18 found in
rows 084/095/099-101: rows 136-138 sit on top of a fully-built playlist surface (UI provider
+ panel + catalog commands + keymap bindings + a 10-format parser crate including an M3U
writer) that is 100% dead because BOTH shipped hosts refuse every `playlist.*` command with
"the playlist model is a Phase-4 card" (apps/cli/src/dispatch.rs:191-192,
apps/web/src/host.rs:1842-1843) and the engine admits "no playlist model in the engine"
(events.rs:99). Row 142 is flat-out FALSE for its play-and-pause sub-behavior: `--start-paused`
is shipped AND tested green in the orchestrator baseline. Row 133's blocked_reason carries a
sub-claim ("Settings UI must render the control disabled-with-reason") that the shipped
settings UI contradicts — it renders deinterlace as enabled enums with a neutral VLC tooltip.
Row 139 understates a shipped, user-visible client-local sort/view. One wrong §-pointer (135).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | status all "not_started", evidence "" — color effects | :1874-1875 | ACCURATE (no code) | `rg -il 'colorthres|posterize|sepia' crates/ apps/` -> 0 engine hits; only ui/.../ColorsTool.tsx = SHALLOW per W15-A10 audit :7 ("no ... engine stage or catalog command exists — local state") |
| 2 | notes: "Video - Colors.svg panel; same shader path." | :1876 | TRUE | `ls design/figma/` -> "Video - Colors.svg" + figma-renders/Video_-_Colors.png exist |
| 3 | status all "not_started" — HDR options | :1886 | ACCURATE (no code) | `rg -il 'tonemap|tone-map' crates/ apps/` -> 0; HdrTool.tsx staged, video.set-hdr absent from protocol (W15-A05 audit: "rg video.set-hdr ... ABSENT — exit 1") |
| 4 | notes: "per design HDR Options panel" + "[VERIFY]" scope | :1888 | TRUE / disclosed-open | design/figma/"HDR Options.svg" + renders/HDR_Options.png exist (ls); [VERIFY] is an honest open marker |
| 5 | status all "blocked" — deinterlacing | :1899 | ACCURATE for the engine feature | `rg -il 'deinterlace|yadif' crates/ apps/` -> 0 engine hits (settings UI + i18n only); "blocked" is a legal status (matrix :55-56) with required blocked_reason present |
| 6 | blocked_reason: "'Later' per feasibility subsystems §4.1" | :1901 | TRUE | docs/feasibility/sections/subsystems.md:420-424: "**Deinterlace**: 50-registration video-filter family ... heavy DSP and hw-coupled. **Later** [PROPOSAL: Phase 6+ decision card, no v1 row beyond `blocked/later` with reason]"; :639 "blocked/later | M (accepted gap) | 6+" |
| 7 | blocked_reason: "Settings UI must render the control disabled-with-reason meanwhile (config-never-invents, F1/F2)" | :1901 | FALSE as shipped | ui/src/screens/settings/generated/schema.ts:1427-1466: video.deinterlace/-method entries have `featureGate: ""`, `featureExposed: true`; schemas/settings.toml:935-960 `ui = true`, `feature_gate = ""`; tooltip messages.ts:2873 "When interlaced video is deinterlaced." — an ENABLED enum, neutral tooltip, no reason, no disabled state |
| 8 | notes: "d/Shift+d hotkeys seeded but engine-blocked" | :1902 | TRUE | keymaps/default.toml:159 `deinterlace-toggle = { key = "D", ... }`, :160 `deinterlace-mode-cycle = { key = "Shift+D", ... }` |
| 9 | notes: "deinterlace/-mode options in inventory" | :1902 | TRUE | docs/inventory/options-linux.json:20067 "option_name": "deinterlace", :20089 "deinterlace-mode" |
| 10 | status all "not_started" — rotate/transform/mirror | :1913 | ACCURATE (no code) | `rg -n 'rotate|mirror' apps/cli/src/host.rs ...` -> all hits are comment false-positives (ts.rs:633 "mirrored", host.rs:969 "mirror constants"); no transform code |
| 11 | status all "not_started" — novelty/analysis filters | :1926 | ACCURATE (no code) | `rg -il 'psychedelic|mosaic|motiondetect|gaussianblur' crates/ apps/` -> only theme-tokens/src/color.rs (color-name false positive) |
| 12 | notes: "v1 covers ~6-8/50 video-filter registrations" | :1928 | TRUE (verbatim) | subsystems.md:436 "v1 covers 4/16 audio-filter registrations + ~6–8/50 video-filter registrations" |
| 13 | notes: "full wont-do list in matrix-seed-notes §5" | :1928 | WRONG POINTER | docs/inventory/matrix-seed-notes.md:148 "**novelty video filters ~90** (`vfeat-novelty-filters` P3)" sits in §4 "Not-adopted v1" (:127); §5 (:155) is the "Blocked list" |
| 14 | status all "not_started" — queue semantics | :1943 | UNREACHABLE (accurate-for-shipped, dead code mass) | UI+catalog+keymap all built; every host refuses: apps/cli/src/dispatch.rs:191-192 `if id.starts_with("playlist.") { "the playlist model is a Phase-4 card" }`; apps/web/src/host.rs:1842-1843 same; engine: events.rs:99 "`playlist.changed` | wired-by-later (no playlist model in the engine; Phase 4)"; W15-A03: panel buttons are "silent no-op" in the shipped shell |
| 15 | notes: "playlist.* command catalog (doc 05 §2.9)" | :1945 | TRUE | docs/05-command-layer.md:285 "### 2.9 Playlist commands"; crates/commands/src/catalog/ids.rs:137-147 PlaylistEnqueue...PlaylistSetRepeat |
| 16 | status all "not_started" — repeat/loop/shuffle | :1955 | UNREACHABLE (accurate-for-shipped) | keymaps/default.toml:202 shuffle R -> command "playlist.set-shuffle" (refused by hosts per #14); queue.ts cycleRepeat is real UI math (W15-A06 "REAL") feeding refused commands |
| 17 | notes: "--loop/--repeat/--random options; r/l hotkeys; C-04 ... unbound by design" | :1957 | TRUE | options-linux.json:21852 "loop", :21869 "repeat", :21835 "random"; keymaps:202 R shuffle; :203-204/:209 repeat-cycle unbound (C-04, L = file-browser) |
| 18 | status all "not_started" — playlist save/load | :1968 | UNREACHABLE | crates/playlists/src/lib.rs:325 `pub fn write(playlist: &Playlist) -> String` (M3U writer, roundtrip law :25) + readers (m3u/pls/xspf/hls/cue/mpd.rs); `rg '::write\(' crates/ apps/` -> zero non-test callers; open-path-wiring.md: "playlists-local ... NOT a 6.OPW unit ... Record as its own future card". UI save = localStorage display names only (W15-A09: "persisting display NAMES only") |
| 19 | notes: "4 playlist-export registrations in census" | :1970 | TRUE | modules-summary.md:191 "playlist-exporter ← `playlist export` (4: html, xspf, m3u8, m3u)" |
| 20 | notes: "M3U v0 write in 4.D1" | :1970 | TRUE-as-dead-code | lib.rs:24-25 "[`write`] — canonical extended M3U output ... Roundtrip law: `parse(write(parse(x))) == parse(x)`" — exists, zero shipped callers |
| 21 | status all "not_started" — sort + search | :1980 | SHALLOW / UNDERSTATED | Sort+view ARE shipped user-visible client-local: PlaylistScreen.tsx:294 `naturalSort`, :474 "the REAL sort + view triggers (PlaylistOptionsToolbar)"; options/SortingOptionsMenu.tsx + ViewOptionsMenu.tsx exist. Search filter: ABSENT (`rg -i 'search' ui/src/screens/playlist/` -> 0). Data = localStorage seeded fixtures (W15-A09) |
| 22 | notes: "Sorting Options/View Options design surfaces" | :1982 | TRUE | design/figma/"Sorting Options.svg" + "View Options.svg" + renders/Sorting_Options.png, View_Options.png exist (ls) |
| 23 | notes: "Sort by Title in context menu" | :1982 | TRUE (VLC census) | ui-inventory.md:172 "Sort (by Title)" in §3.1 VLC playlist context menu |
| 24 | status all "not_started" — media library / watch folders | :1992 | ACCURATE (staged UI only) | ui/src/screens/settings/areas/library.tsx: "no schemas/settings.toml [[setting]] keys exist behind them yet ... staged"; `rg 'media.?library|recursive' crates/ apps/` -> no engine scan code |
| 25 | status all "not_started" — recent media | :2004 | ACCURATE-for-shipped | recentOpenings demo-gated: W15-A02 "isDemoTransport(client) excludes recentOpenings — in the shipped desktop [shell]"; ControlBar has an "Open Recent" submenu rendering "the honest empty reason — never a fabricated history" (ControlBar.test.tsx:738) — surface shipped, feature absent |
| 26 | status all "not_started" — queue behaviors | :2016 | FALSE (understated subset) | `--start-paused` shipped+tested: apps/cli/src/dispatch.rs:88-99 composes open -> settle -> pause; apps/cli/tests/session_script.rs:173-178 asserts "state-changed Playing → Paused"; baseline cargo-test-workspace.txt:3218 `test start_paused_opens_then_pauses ... ok`. play-and-exit: only VLC-side differential-test args (crates/codecs/tests/vorbis_vlc_differential.rs:70). cork/one-instance: absent (`rg` -> 0; no tauri single-instance plugin) |
| 27 | notes: "one-instance = desktop-shell concern (Tauri single-instance)" | :2018 | TRUE as concern; not implemented | `rg -in 'single|instance' apps/desktop-tauri/Cargo.toml` -> only a code comment; no plugin |
| 28 | status all "not_started" — network SD sources panel | :2028 | ACCURATE (no code) | playlist-services.tsx: "renders nothing today (no playlist-services [[setting]] keys exist)"; SD framework rows deferred (sd-* rows per MP §14) |
| 29 | row 131 inventory_ref "inventory/options-summary.md#s5" | :1873 | RESOLVES | options-summary.md:105 "## 5. Mapping to our settings-schema seed categories" |
| 30 | rows 132/136/139/141/143 inventory_ref ui-inventory #s6/#s2/#s1.1 | :1885/:1942/:1979/:2003/:2027 | RESOLVE | ui-inventory.md:346 "## 6. Crosswalk", :136 "## 2. Dialog catalog", :30 "### 1.1 Media (Alt+M)" |
| 31 | rows 133/137/138 inventory_ref options-linux #deinterlace/#random, formats #xspf | :1898/:1954/:1967 | RESOLVE | options-linux.json:20067 deinterlace, :21835 random; formats.json:1679 family "xspf" |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 133 blocked_reason: "Settings UI must render the control disabled-with-reason meanwhile (config-never-invents, F1/F2)" | Shipped schema: featureGate "" + featureExposed true, enabled enums, neutral tooltip | The source is a directive ("must") the shipped UI does not implement: schema.ts:1427-1443 + messages.ts:2873-2874. The control looks alive and writes config nothing consumes — the exact "dead control that looks alive" their F1/F2 rule forbids |
| Row 142: all platforms "not_started" | CLI: --start-paused shipped, composed open→pause, tested green | Baseline cargo-test-workspace.txt:3218 `ok`; session_script.rs:173-178; web host refuses it honestly (host.rs:1467-1471 UnsupportedForHost) |
| Row 139: "not_started" sort+search | PlaylistScreen.tsx:474 "the REAL sort + view triggers (PlaylistOptionsToolbar)" | UI ships working client-local sort/view (search absent); matrix hides a user-visible surface |
| Row 135: "full wont-do list in matrix-seed-notes §5" | matrix-seed-notes §4 (:148) holds the novelty wont-do entry; §5 (:155) is the Blocked list | §4 is the true location — anchor drift |
| (cross-doc) subsystems.md:438 "Feature-matrix rows must say `partial` with the named set" (extract/invert/posterize/gradient) vs row 131 "not_started" | rg census: zero engine code for those filters | The MATRIX is the honest one here (feasibility overpromises); noted as feasibility-vs-matrix drift, not a matrix lie |

## 4. Fabrication / inflation indicators
ZERO inflation direction. 12/13 rows carry empty `evidence` and honest absence statuses. The
chunk's defects are (a) understatement of shipped/tested code (rows 142, 139, and the dead
playlist surface under 136-138), (b) one normative sub-claim stated as fact that the shipped UI
contradicts (row 133), (c) one stale §-pointer (row 135). Same understatement disease W18
proved on rows 084/095/099-101 — the parity-report tooling (0/252) counts none of it.

## 5. Matrix rows
| Row | id | Statuses claimed | Evidence string | Code actually found (rg) | Executed? | Verdict per §5 |
|---|---|---|---|---|---|---|
| 131 | vfeat-color-fx | not_started x4 | "" | 0 engine hits; staged ColorsTool (local state only, W15-A10 SHALLOW) | no (census rg, exit 1) | ACCURATE-no-code (feature absent; UI honestly staged) |
| 132 | vfeat-hdr | not_started x4 | "" | 0 engine hits; HdrTool staged; video.set-hdr absent from protocol (W15-A05) | no | ACCURATE-no-code |
| 133 | vfeat-deinterlace | blocked x4 + blocked_reason | "" | 0 engine hits; keymap D/Shift+D seeded; settings enums exposed (enabled, not disabled-with-reason) | no | SHALLOW (blocked status honest; blocked_reason's settings-UI sub-claim FALSE as shipped) |
| 134 | vfeat-rotate-transform | not_started x4 | "" | 0 (all rotate/mirror hits = comment false-positives) | no | ACCURATE-no-code |
| 135 | vfeat-novelty-filters | not_started x4 | "" | 0 (theme-tokens false positive only) | no | ACCURATE-no-code (notes §-pointer drift: §5 -> §4) |
| 136 | plfeat-queue-model | not_started x4 | "" | Full dead surface: PlaylistProvider/PlaylistPanel/queue.ts, catalog ids.rs:137-147, keymap; hosts refuse playlist.* on CLI+web; engine has no playlist model (events.rs:99) | no (host refusal read; W15-A03 cited) | UNREACHABLE (real code, no shipped consumer) |
| 137 | plfeat-repeat-loop-shuffle | not_started x4 | "" | keymap shuffle->playlist.set-shuffle (dead-end); cycleRepeat UI math real; engine none | no | UNREACHABLE |
| 138 | plfeat-save-load | not_started x4 | "" | crates/playlists readers + lib.rs:325 M3U write(), zero non-test callers; UI saves names to localStorage | no | UNREACHABLE |
| 139 | plfeat-sort-search | not_started x4 | "" | REAL client-local sort/view (PlaylistScreen:294,474; options/SortingOptionsMenu.tsx, ViewOptionsMenu.tsx); search ABSENT; data = seeded fixtures | no | SHALLOW (UI subset shipped+user-visible; engine absent; search missing) |
| 140 | plfeat-media-library | not_started x4 | "" | library.tsx staged inert rows; no engine scan code | no | ACCURATE-no-code |
| 141 | plfeat-recent-items | not_started x4 | "" | recentOpenings demo-gated (dead in desktop shell); "Open Recent" menu = honest empty state | no | ACCURATE-for-shipped (demo-dead UI code) |
| 142 | plfeat-autostart-behaviors | not_started x4 | "" | --start-paused shipped (CLI compose, web honest-refusal); play-and-exit/cork/one-instance absent | YES (baseline test:3218 ok, orchestrator run; not re-run — cargo-build-budget) | FALSE (understated: tested sub-behavior marked not_started) |
| 143 | plfeat-service-discovery-panel | not_started x4 | "" | playlist-services.tsx renders nothing (honest seam); no SD code | no | ACCURATE-no-code |

## 6. Open questions for the next agent
- Row 133: decide whether the enabled deinterlace enum violates config-never-invents F1/F2
  in the shipped settings screen, or whether "VLC-inherited config key" exempts it.
- Row 142: baseline `start_paused_opens_then_pauses` is the only executed evidence; a
  --play-and-exit / --start-paused parity sweep was never run.
