# CLAIM AUDIT: docs/feature-matrix.toml rows 235-247 (hotkey category, lines 3200-3355)
| Field | Value |
|---|---|
| Auditor | W19-A08 |
| Claims extracted | 13 rows (status claims + evidence-path claims + notes claims) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 1 / 0 / 11 / 0 / 0 / 1 / 0 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
The hotkey rows are honestly labeled: none claims `verified`; every platform column is
`partial` (desktop) / `not_started` (web) / `blocked` (disc nav), and the notes repeat
"Backfill (issue #20) partial". The evidence paths exist (13/13 files checked) and the
seed bindings exist verbatim in keymaps/default.toml. What "partial" means is thin: the
seed data file + a shortcuts-browser screen + an editor are live; the shipped web player
dispatches only ~8 keys (F, Esc, I, B, G, Ctrl+T, Ctrl+B, Ctrl+E, Ctrl+A) of the 141
bindings seeded — Space, S, P, N, arrows, B-track, ., etc. do nothing. Row 235's evidence
sub-claim "control-bar subset wired: playback.play/pause/toggle-play-pause" is FALSE in
the shipped build: the dispatch branch exists but no binding in the live keymap can
reach it (dormant code). Row 244 cites the WRONG FILE (AppShell.tsx) for the shuffle/
repeat consumers (real consumer: PlaylistPanel.tsx).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "control-bar subset wired: playback.play/pause/toggle-play-pause" (row 235 evidence) | matrix L3207 | FALSE | ui/src/screens/player/controls/state-mapping.ts:155-158: "The seed keymap has no play/pause binding yet — this stays data-driven so the loaded keymap lights it up"; PLAYER_SHELL_KEYMAP = 5 bindings only (variants/variantKeymap.ts:44-47 + keymap.ts:68-73). rg: no Space/S/N/P keydown dispatch in PlayerScreen.tsx |
| 2 | "dispatch ui/src/screens/player/keymap.ts → PlayerScreen.tsx onBinding seam" (rows 235-243) | matrix L3207 etc. | SHALLOW | PlayerScreen.tsx:821 matchBindings(PLAYER_SHELL_KEYMAP) → 5 plain keys; App.tsx:203-207 renders <PlayerScreen> WITHOUT onBinding — forwarded bindings go nowhere (optional chain) |
| 3 | "keymaps/default.toml (playback.play/pause/stop/toggle-play-pause + playlist.prev/next + media keys)" | matrix L3207 | REAL | keymaps/default.toml:61-69, 83-84 (sed-verified bindings exist) |
| 4 | "jump-magnitude args 3/10/60/300 s" (row 236) | matrix L3219 | REAL | keymaps/default.toml:78-81 jump-size-*-ms = 3000/10000/60000/300000 |
| 5 | "engine crates/playback-engine/src/seek.rs + session seek()" (row 236) | matrix L3219 | REAL | session.rs:368 `pub fn seek(&self, request: RequestId, target: SeekTarget) -> Result<...>`; playback_engine used by apps/cli/src/host.rs (rg) |
| 6 | "playback.set-rate (crates/commands/src/catalog/ids.rs)" (row 237) | matrix L3231 | REAL | ids.rs:107 `PlaybackSetRate => "playback.set-rate"` |
| 7 | "engine clock-rate interplay (3.B) not implemented (crates/playback-engine/src/lib.rs 'deliberately NOT here')" (row 237) | matrix L3231 | REAL | lib.rs:49 `//! # What is deliberately NOT here (Phase 3+)` |
| 8 | "volume/mute live in the effects area via audio.set-volume/set-mute" (row 238) | matrix L3243 | REAL-UNPROVEN→SHALLOW row | ids.rs:118 `AudioSetVolume => "audio.set-volume"`; effects-area wiring not re-executed (ui-screen-av-tools is another agent's row) |
| 9 | "ui/src/screens/player/useFullscreen.ts (4.C1)" (row 242) | matrix L3279 | REAL | file exists, 79 lines; PlayerScreen.tsx:822-827 calls toggleFullscreen/exitFullscreen |
| 10 | "engine-reported shuffle/repeat checkboxes consumed by ui/src/components/appshell/AppShell.tsx" (row 244) | matrix L3307 | FALSE (wrong file) | `rg -i 'shuffle|repeat' ui/src/components/appshell/AppShell.tsx` → 0 hits; real consumer: PlaylistPanel.tsx:11 "shuffle and repeat reflect ENGINE-reported state", :145-149 |
| 11 | "set-bookmark-1..10 Ctrl+F1–F10 + play-bookmark-1..10 F1–F10" (row 245) | matrix L3319 | REAL | `rg -c 'set-bookmark-|play-bookmark-' keymaps/default.toml` → 10 + 10; L238-241 verified |
| 12 | "engine crates/playback-engine/src/{frame_step,tracks,bookmarks,chapters}.rs" (rows 239/240/245/246) | matrix | REAL | all exist (169/593/204/236 lines); pub mod at lib.rs:93-107 |
| 13 | "web = not_started" (all 12 partial rows) | matrix L3206 etc. | SHALLOW | every row's notes cite live web-UI code ("shortcuts browser + editor live" — ShortcutsScreen routed at App.tsx:306 /keyboard-shortcuts; HotkeysEditor.tsx exists) → web column understates its own evidence |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 235: "control-bar subset wired: playback.play/pause/toggle-play-pause" | state-mapping.ts:156-157: "seed keymap has no play/pause binding yet" | Code is true — dispatch branch dormant; no Space/S binding in PLAYER_SHELL_KEYMAP |
| Row 244 evidence: AppShell.tsx consumes shuffle/repeat | rg: zero shuffle/repeat hits in AppShell.tsx | PlaylistPanel.tsx:11,145 is the consumer — matrix evidence path is wrong |

## 4. Fabrication / inflation indicators
No fabricated status (no `verified` jumps — the anti-fabrication rule is respected).
Inflation is marginal but real: "partial" for 12 rows rests on (a) a data seed, (b) an
editor screen, (c) engine modules not reachable by any key press in the shipped UI;
a reader could infer working hotkeys. The notes themselves disclaim this ("dispatch is
seam-only"), so this is over-optimistic phrasing, not fraud.

## 5. Matrix rows
| Row id | Claimed statuses | Evidence strings | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|
| 235 hotkey-transport | win/linux/mac partial, web not_started | keymaps seed, ShortcutsScreen, HotkeysEditor, keymap.ts seam | seed ✓ (L61-69,83-84); screens ✓; live dispatch = 5 plain keys + 4 combos; play/pause branch dormant | read + rg (no test run) | SHALLOW |
| 236 hotkey-seek | partial x3, web ns | seed, seek.rs, session seek() | all ✓ (session.rs:368) | read | SHALLOW |
| 237 hotkey-rate | partial x3, web ns | seed set-rate, catalog, editor; engine rate absent | ✓ ids.rs:107; lib.rs:49 quote ✓ | read | SHALLOW |
| 238 hotkey-volume | partial x3, web ns | seed, effects area, editor | seed ✓ (L65-67,144); catalog ✓ | read | SHALLOW |
| 239 hotkey-frame-step | partial x3, web ns | seed .,/,, frame_step.rs, demo handler | seed ✓ (L94-95); engine ✓ | read | SHALLOW |
| 240 hotkey-track | partial x3, web ns | seed B/V/Alt+V, tracks.rs | seed ✓ (L147,185-187); tracks.rs ✓ | read | SHALLOW |
| 241 hotkey-sync | partial x3, web ns | seed J/K/G/H, subsync, macOS overrides, catalog | seed ✓ (L149-150,188-193,278-281); ids.rs:123,133 ✓ | read | SHALLOW |
| 242 hotkey-video-display | partial x3, web ns | seed F/A/C/Z/D, useFullscreen.ts | seed ✓ (L154-156,159-164); useFullscreen ✓ | read | SHALLOW |
| 243 hotkey-subtitle-scale | partial x3, web ns | seed Ctrl+wheel/0, editor | seed ✓ (L194-196) | read | SHALLOW |
| 244 hotkey-playlist | partial x3, web ns | seed, AppShell.tsx claim, editor | seed ✓ (L202,206); AppShell claim FALSE (PlaylistPanel is consumer) | read + rg | SHALLOW (1 FALSE evidence path) |
| 245 hotkey-bookmarks | partial x3, web ns | seed 20 rows, bookmarks.rs, editor | 10+10 rows ✓; bookmarks.rs ✓ | rg count | SHALLOW |
| 246 hotkey-navigation | partial x3, web ns | seed Shift+P/N, chapters.rs, Q6 flags | seed ✓ (L122-126); chapters.rs ✓ | read | SHALLOW |
| 247 hotkey-nav-disc | blocked x3, web unsupported | (empty; blocked_reason) | no disc-nav dispatch in UI; consistent | read | REAL (honest blocked declaration, empty evidence per schema) |

Commands run: sed -n '3195,3360p' docs/feature-matrix.toml (rows read); rg over
keymaps/default.toml, ui/src/**, crates/{playback-engine,commands}; existence checks
(13/13 evidence paths exist). No cargo/npm commands (baseline cited where needed).
