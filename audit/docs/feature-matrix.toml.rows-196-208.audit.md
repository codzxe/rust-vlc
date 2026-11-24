# CLAIM AUDIT: docs/feature-matrix.toml rows 196-208 (ui-screen tail + ui-dialog head)
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 13 rows (≈52 platform cells + 13 evidence strings) |
| REAL 12 / SHALLOW 1 / FALSE 0 / others 0 | |
| Internal contradictions found | 1 (row 204 status vs its own description/notes) |

## 1. Blunt verdict
This chunk is the honest end of the matrix: 8 "implemented" rows whose cited evidence files ALL exist with substantive code and matching tests; 1 honestly-declared "partial" row (av-tools) whose inert-with-reason discipline is verified in code; 1 wont-do row (skins2) recorded instead of hidden; 3 absence rows (not_started/blocked) with no evidence fabrications. The single defect: row 204 (ui-dialog-open-file) claims status implemented while its own description enumerates native file picker / folder picker / clipboard MRL / drag-and-drop — of which the notes admit the native pickers and clipboard flow are still open, with a path input as substitute. Suite-level proof: baseline ui-test.txt = 136 files / 1705 passed (covers every .test.tsx cited below). No fabricated evidence paths in this chunk (the two repo-wide missing paths flagged by xtask feature-audit belong to rows 23 and 163, not this chunk).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 196 | "ui-screen-media-info ... implemented w/l/m; evidence MediaInfoScreen.tsx + Panel + Provider (+ .test.tsx each, fixtures.ts, format.ts)" | rows-196 | REAL | all 5 files exist (29/423/98/159/57 LOC) + MediaInfoPanel.test.tsx + MediaInfoScreen.test.tsx exist (ls run); suite green baseline |
| 197 | "ui-screen-onboarding ... implemented; evidence OnboardingScreen.tsx + onboardingState.ts (+ test)" | :197 | REAL | 964/156 LOC + OnboardingScreen.test.tsx exist |
| 198 | "ui-screen-miniplayer-music ... implemented; evidence MiniPlayer.tsx + MusicMode.tsx + variantKeymap.ts + variantLayout.ts (+ variants.test.tsx)" | :198 | REAL | 595/154/47/115 LOC + variants.test.tsx exist |
| 199 | "ui-screen-keyboard-shortcuts ... implemented; evidence ShortcutsScreen.tsx (+ test)" | :199 | REAL | 511 LOC + ShortcutsScreen.test.tsx |
| 200 | "ui-screen-customize ... implemented; evidence CustomizePanels.tsx + panels.ts + capabilities.ts (+ test)" | :200 | REAL | 842/227/61 LOC + CustomizePanels.test.tsx |
| 201 | "ui-screen-av-tools ... partial; ... balance + 3-band EQ + video-adjust sliders inert-with-reason pending audio.set-eq/video.set-adjust" | :201 | REAL | effects.tsx 397 LOC with `data-pending-command="audio.set-eq"` markers at :262-264,:340-351 — inert-with-reason is real, not a claim; volume/mute live via audio.set-volume/set-mute verified in ui/src/screens/player/context.tsx:390-393 (loosely attributed to the effects area by the evidence string — the commands are wired in the player provider, not effects.tsx) |
| 202 | "ui-screen-theme ... implemented; evidence ThemeModePicker.tsx + settingsThemeSync.ts + tokens.json + tokens.rs" | :202 | REAL | 91/120/1457/80 LOC all exist |
| 203 | "ui-screen-skins2 ... unsupported all platforms; evidence = empty; blocked_reason 'By design: not reimplemented'" | :203 | REAL | honest wont-do row; no code promises skins2 (rg: no skins2 impl in ui/) |
| 204 | "ui-dialog-open-file ... implemented; description 'file picker, folder picker, clipboard MRL, drag-and-drop'" | :204 | SHALLOW | OpenMediaDialog.tsx 326 LOC + test exist; App.tsx:199 lumen:open-media listener + session.open/playlist.load wiring verified — the DIALOG is live. But the description's native pickers + clipboard MRL are admitted-open in the row's own notes ("path input is the current substitute"); drag-and-drop exists only in playlist/variant panels (PlaylistPanel.tsx onDrop), not an open flow. Status cell overclaims the described surface. |
| 205 | "ui-dialog-open-network ... implemented; evidence NetworkStreamScreen.tsx (+ test)" | :205 | REAL | 312 LOC + NetworkStreamScreen.test.tsx; design extras admitted open in notes |
| 206 | "ui-dialog-open-multiple ... not_started; evidence empty" | :206 | REAL | absence claim; no code contradicts (no multi-open dialog in ui/src/screens) |
| 207 | "ui-dialog-open-disc ... not_started" | :207 | REAL | absence; disc open gated behind platform-disc rows per notes |
| 208 | "ui-dialog-open-capture ... blocked (hw-gated); web unsupported" | :208 | REAL | honest blocked_reason; capture-device lanes are separate platform rows |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 204 status "implemented" for "file picker, folder picker, clipboard MRL, drag-and-drop" | Row 204's own notes: "NATIVE file/folder pickers (4.H1) and clipboard-MRL flow still open — path input is the current substitute" | The notes are the truth; the status+description overclaim. Dialog live, enumerated flows largely substituted. |
| (cross-chunk) row 23 evidence `crates/codecs/src/video/h264/sw` flagged missing by xtask feature-audit baseline | crates/codecs/src/video/h264/sw.rs EXISTS | path-format violation only (missing .rs), not a missing implementation |
| (cross-chunk) row 163 evidence `ui/src/screens/settings/areas/advanced.tsx` | file MISSING; advanced.test.tsx EXISTS | genuine broken evidence path — belongs to another agent's chunk |

## 4. Fabrication / inflation indicators
One overclaim (row 204, status vs description). Notably ABSENT here: no [MOCK], no invented digests, no "verified" claims — statuses stay at "implemented" (never "verified"), consistent with PARITY-REPORT's 0/252 verified-parity honesty. The 2026-11-10 self-retraction in row 23's notes (h264 hw "previously claimed ... Media Foundation claim was false ... corrected") shows the matrix carries its own correction history — the opposite of fabrication culture.

## 5. Matrix rows (per-row detail)
| row id | statuses claimed | evidence strings | code actually found | executed? | verdict |
|---|---|---|---|---|---|
| ui-screen-media-info | w/l/m=implemented, web=not_started | 5 named files + tests | all exist (29-423 LOC) | suite-level: baseline ui-test 1705 green; not run individually | REAL |
| ui-screen-onboarding | implemented x3 | 2 files + test | 964+156 LOC + test | same | REAL |
| ui-screen-miniplayer-music | implemented x3 | 4 files + variants.test.tsx | all exist | same | REAL |
| ui-screen-keyboard-shortcuts | implemented x3 | 1 file + test | 511 LOC | same | REAL |
| ui-screen-customize | implemented x3 | 3 files + test | 842+227+61 LOC | same | REAL |
| ui-screen-av-tools | partial x3 | effects.tsx + commit ids + pending markers | 397 LOC; pending-command markers verified at :262-264; volume/mute in context.tsx:390-393 | same | REAL (honest partial) |
| ui-screen-theme | implemented x3 | 4 files incl. tokens.rs | 91/120/1457/80 LOC | same | REAL |
| ui-screen-skins2 | unsupported x4 | "" + blocked_reason | no skins2 code (correct) | n/a | REAL (wont-do) |
| ui-dialog-open-file | implemented x3 | dialog + test + AppShell listener | 326 LOC; listener at App.tsx:199; session.open wiring at OpenMediaDialog.tsx:136 | same | SHALLOW (description overclaim; notes honest) |
| ui-dialog-open-network | implemented x3 | 1 file + test | 312 LOC | same | REAL |
| ui-dialog-open-multiple | not_started x4 | "" | none (correct) | n/a | REAL |
| ui-dialog-open-disc | not_started x4 | "" | none (correct) | n/a | REAL |
| ui-dialog-open-capture | blocked x3, web=unsupported | "" + reason | none; capture lanes elsewhere | n/a | REAL |

Row-count for the headline metric (this chunk only): REAL 12, SHALLOW 1, others 0 — of 13.
