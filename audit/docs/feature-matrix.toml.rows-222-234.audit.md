# CLAIM AUDIT: docs/feature-matrix.toml rows 222-234 (menu-item category)
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted (rows) | 13 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 0 / 0 / 3 / 0 / 0 / 10 / 0 |
| Internal contradictions found | 2 (evidence strings vs AppShell.tsx content) |

## 1. Blunt verdict
Ten of thirteen rows claim `not_started` ("no work yet") while working code for
the row's own described feature exists in the tree (dialogs, canvas context
menus, fullscreen, crop/aspect tools). Three `partial` rows cite evidence
strings naming an "AppShell.tsx … menu" that the file itself says no longer
exists — the menubar was removed in the W25/W26 redesign and the app menu now
lives in ControlBar.tsx. The statuses are systematically UNDER-stated (the
anti-inflation direction), but a parity ledger that is wrong in either
direction is still wrong, and the three evidence strings do not resolve
against the code they cite. This chunk is evidence-rotten, not fraudulent.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "status = { windows = \"not_started\", … }" menu-playback-navigation | feature-matrix.toml:3039-3049 | FALSE (understated) | chapters implemented: crates/demux/src/mkv/editions.rs, crates/demux/src/mp4/mod.rs, ui/src/client/demo-transport.ts:22 (`rg -l 'chapter' ui/src crates` hits 8 files) |
| 2 | all not_started — menu-playback-bookmarks | feature-matrix.toml:3051-3060 | FALSE (understated) | GotoTimeDialog.tsx + BookmarksDialog.tsx exist; keymaps/default.toml:120,224; baseline ui-test.txt:46,91 (11+10 tests ✓) |
| 3 | all not_started — menu-audio | feature-matrix.toml:3063-3072 | FALSE (understated) | ControlBar.tsx track selectors live in canvas context menu (ControlBar.test.tsx:162 "the track selectors' menu home is the canvas context menu") |
| 4 | all not_started — menu-video-display | feature-matrix.toml:3075-3084 | FALSE (understated) | ui/src/screens/player/useFullscreen.test.tsx exists (`rg -l fullscreen ui/src`) |
| 5 | all not_started — menu-video-adjust | feature-matrix.toml:3087-3096 | FALSE (understated) | ui/src/screens/player/tools/video/CropTool.test.tsx, renderer-wgpu/src/params.rs aspect/crop params |
| 6 | all not_started — menu-video-snapshot | feature-matrix.toml:3099-3108 | REAL (status honest) | `rg -i 'snapshot' crates ui/src` → no video-snapshot implementation (only unrelated words) |
| 7 | all not_started — menu-subtitle | feature-matrix.toml:3111-3120 | FALSE (understated) | ui/src/screens/playlist/subtitles/SubtitlePickList.test.tsx + PlaylistPanel.tsx:440-443 add-subtitles entry |
| 8 | partial + evidence "AppShell.tsx Tools menu" — menu-tools | feature-matrix.toml:3122-3132 | SHALLOW | AppShell.tsx:35-37 "The Solstice menubar is GONE"; no menu.tools.* rendered anywhere; items live in ControlBar.tsx:1030-1054 app menu |
| 9 | partial + evidence "AppShell.tsx View menu" — menu-view | feature-matrix.toml:3134-3144 | SHALLOW | AppShell.tsx:166-167 uses menu.view.music-mode/miniplayer as crumb labels; no View menu exists; navigation is the rail (AppShell.tsx:151-157) |
| 10 | partial + evidence "AppShell.tsx Help menu" — menu-help | feature-matrix.toml:3146-3156 | SHALLOW | menu.help.* keys exist only in i18n/messages.ts:560-561,2046-2047; About/shortcuts are ControlBar.tsx:1024-1054 app-menu ids |
| 11 | all not_started — menu-context-surface | feature-matrix.toml:3159-3168 | FALSE (understated) | ControlBar.test.tsx:162 — video-surface canvas context menu exists (track selectors) |
| 12 | all not_started — menu-context-playlist | feature-matrix.toml:3171-3181 | FALSE (understated) | PlaylistPanel.tsx:15 "QUEUE-ROW CONTEXT MENU (playlist context menu.svg, 8 icon rows" — real 8-row menu |
| 13 | all not_started — menu-tray | feature-matrix.toml:3183-3193 | REAL (status honest) | `rg -i 'tray' ui/src apps crates` → only Cargo.lock:4190 dependency, no code |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| rows 229-231 evidence: "ui/src/components/appshell/AppShell.tsx Tools/View/Help menu" | AppShell.tsx:35-37: "The Solstice menubar is GONE (no W25 frame carries one)" | AppShell is true; the matrix evidence is stale — ControlBar.tsx:881 "the app menu lives in the ControlBar FOR NOW" |
| row 223 (bookmarks) notes "Ctrl+T / Ctrl+B" with status not_started | keymaps/default.toml:120/224 bind both, PlayerScreen dialogs exist, baseline tests pass | keymap + dialogs are true; not_started is false |

## 4. Fabrication / inflation indicators
None in the inflation direction: no row claims verified/partial without a
subset actually working. The rot is the opposite direction — 8 rows say
"no work yet" for shipped code. Matrix header (lines 74-75) admits "nothing
in this file is verified — no CI exists yet"; orchestrator baseline
(xtask-parity-report) shows 0/252 verified, consistent.

## 5. Matrix rows (per-row verdicts)
| Row id | Statuses claimed | Evidence string | Code actually found | Executed? output | Verdict |
|---|---|---|---|---|---|
| menu-playback-navigation | all not_started | "" | mkv/mp4 chapters (demux), demo-transport chapters, PlayerScreen | no direct run; baseline cargo-test covers demux | FALSE (understated) |
| menu-playback-bookmarks | all not_started | "" | GotoTimeDialog.tsx, BookmarksDialog.tsx, keymap Ctrl+T/Ctrl+B, engine media.add/get/remove-bookmark | baseline ui-test.txt:46,91 ✓ 21 tests | FALSE (understated) |
| menu-playback-bookmarks-engine | — | — | crates/commands/src/catalog/command.rs:386-402 bookmark commands | cargo-test-workspace.txt (4744 pass) | REAL-UNPROVEN (engine layer) |
| menu-audio | all not_started | "" | canvas context-menu track selectors (ControlBar) | baseline ControlBar tests | FALSE (understated) |
| menu-video-display | all not_started | "" | useFullscreen.tsx + test | baseline ui-test | FALSE (understated) |
| menu-video-adjust | all not_started | "" | CropTool/VideoCard + renderer-wgpu aspect/crop | baseline ui-test | FALSE (understated) |
| menu-video-snapshot | all not_started | "" | none found | rg: no hits | REAL (honest) |
| menu-subtitle | all not_started | "" | SubtitlePickList + Add Subtitle File entry | baseline ui-test | FALSE (understated) |
| menu-tools | partial x3 | "AppShell.tsx Tools menu (menu.tools.customize → …)" | menu.tools.* keys never rendered; customize/settings/about live in ControlBar app menu | rg: keys only in messages.ts:2042-2044 | SHALLOW (subset real, evidence pointer false) |
| menu-view | partial x3 | "AppShell.tsx View menu (menu.view.player/…)" | no View menu; rail+crumb navigation real (AppShell.tsx:151-175) | read AppShell.tsx | SHALLOW |
| menu-help | partial x3 | "AppShell.tsx Help menu (menu.help.about, …)" | no Help menu; about/keyboard-shortcuts in ControlBar app menu (ids "about","keyboard-shortcuts") | read ControlBar.tsx:1024-1054 | SHALLOW |
| menu-context-surface | all not_started | "" | canvas context menu w/ track selectors | ControlBar.test.tsx:440 | FALSE (understated) |
| menu-context-playlist | all not_started | "" | PlaylistPanel queue-row context menu, 8 rows | PlaylistPanel tests in baseline | FALSE (understated) |
| menu-tray | all not_started | "" | no tray code (Cargo.lock dep only) | rg: no code hits | REAL (honest) |

Findings: F-001 [S2] evidence strings of rows 229-231 cite a surface
("AppShell.tsx … menu") that does not exist in that file
(feature-matrix.toml:3131,3143,3155 vs AppShell.tsx:35-37) — evidence does
not resolve; the ledger's own R4-style resolution fails. F-002 [S3] 8 rows
say not_started while the feature subset is implemented elsewhere (see
table). F-003 [S4] web column: all 13 rows not_started but the UI is one
React app for both hosts; statuses were copied per-platform without
platform-specific checking (UNVERIFIED whether web build strips dialogs).

Commands run: rg (menus/bookmark/snapshot/tray/chapter/fullscreen/aspect/
subtitle searches), sed -n verification of every quoted line, awk row
indexing, wc. Baseline citations: audit/runs/baseline/ui-test.txt:46,91,29,38;
xtask-parity-report (0/252 verified).
