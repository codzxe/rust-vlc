# AUDIT: ui/src/screens/player/dialogs/ToolsDialog.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 207 / 1 helper + 4 describe blocks (14 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:94 `✓ src/screens/player/dialogs/ToolsDialog.test.tsx (14 tests) 1271ms`; subject ToolsDialog.tsx (169 L) ships via ToolsDialogHost on the player shell |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `grep -n ToolsDialog audit/runs/baseline/ui-test.txt` -> :94; `rg -n "data-staged-tool|data-pending-command" ToolsDialog.tsx` -> :18,:119-120; read of ui/src/generated/commands.ts:24 CommandRequest union (audio commands enumerated) |

## 1. One-paragraph verdict
Honest integration suite for the tools dialog shell: real tab-pair a11y state, real focus-trap wrap behavior (Tab/Shift+Tab cycle :57-67), real event-seam lifecycle for ToolsDialogHost (mount on CustomEvent, unmount on close, re-open fresh :181-194), and — importantly — it does NOT fake tool wiring: it pins the `data-staged-tool`/`data-pending-command` markers that declare each audio tool awaits a command the engine does not expose. The tests would fail if the staged markers were silently dropped, which is exactly the anti-fabrication property wanted.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:94 (14/14 pass; lane 1705 passed :178-179). Mutation reasoning: tab-state regression fails :41-42; trap-wrap regression fails :62-66; seam lifecycle regression fails :165/:188-193; staged-marker removal fails :113-115/:126-129.

## 3. Findings
None in this file. Checked: tautologies (none — role queries + state attributes), subject mocks (none — real ToolsDialog/Host), skipped/ignored tests (none — 14/14 run), copy-paste (the two tab suites differ meaningfully).

CROSS-REF (not a finding against this test, for the matrix/feature agents): the staged markers name `audio.set-eq`, `audio.set-compressor`, `audio.set-spatializer`, `audio.set-stereo-widener`, `audio.set-visualizer` — none of these commands exist in the generated CommandRequest union (ui/src/generated/commands.ts:24 lists only audio.set-volume/get-volume/set-mute/list-devices/set-device/set-delay). The tools dialog is UI-forward, engine-unwired by design and honestly labeled. Any feature-matrix row claiming equalizer/compressor/spatializer/widener/visualizer as WORKING on any platform is FALSE — verify before crediting such a row.

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| renderDialog helper | 28-32 | REAL | — | onClose spy, defaults |
| shell chrome suite | 34-74 | REAL | — | dialog/tabs/aria-selected, initial focus, trap wrap, Escape |
| AUDIO tab suite | 76-131 | REAL | — | 5 tools mounted collapsed; EQ expands to 11 sliders + preset button; staged markers pinned |
| VIDEO tab suite | 133-160 | REAL | — | tab swap swaps tool sets; defaultTab preselect |
| Host seam suite | 162-207 | REAL | — | event detail -> tab; per-open fresh mount; openToolsDialog dispatch |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| trap wrap :57-67 | YES | real keyDown focus movement asserted |
| EQ expansion :96-106 | YES | 11 sliders after real click |
| staged markers :108-130 | YES | attribute values exact |
| host lifecycle :181-194 | YES | unmount + remount after real close click |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 14 tests run | baseline ui-test.txt:94 | REAL | `✓ ... (14 tests) 1271ms` |

## 7. Dead / duplicated / unreachable code
None. VIDEO_TOOL_NAMES includes "Essential" and "HDR options" — both asserted in the video-tab suites.

## 8. Dependencies & flags
Imports resolve (w26.a18 keys, ToolsDialog exports incl. TOOLS_DIALOG_EVENT/openToolsDialog — used :167/:199).

## 9. Open questions for the next agent
Feature-matrix agents: see CROSS-REF above — audio tools staged, no commands exist.
