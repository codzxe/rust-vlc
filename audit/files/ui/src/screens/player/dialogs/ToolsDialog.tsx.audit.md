# AUDIT: ui/src/screens/player/dialogs/ToolsDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 169 / 5 (asTab, openToolsDialog, useToolsMsg, ToolsDialog, ToolsDialogHost) |
| File verdict | SHALLOW — real dialog plumbing hosting fully-interactive controls that command NOTHING |
| Reachable from shipped entry point? | YES — ToolsDialogHost mounted in PlayerScreen; opened from ControlBar's "Show Advanced Settings" footers (ControlBar.tsx:697-716) and Ctrl+E (PlayerScreen DIALOG_KEYMAP -> openToolsDialog("audio")) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "send\(|useClient|dispatch" tools/**` (ZERO dispatches in any tool card); `rg -n "data-staged|data-pending-command" CompressorTool/ColorsTool`; `rg -n "it\(" ToolsDialog.test.tsx`; read of CompressorTool.tsx/ColorsTool.tsx/EqualizerTool.tsx headers |

## 1. One-paragraph verdict
The dialog chrome and trigger seam are real (CustomEvent `lumen:open-tools`, per-open remount keyed by tab, TestsDialog.test.tsx asserts the seam dispatch), but the payload is the largest interactive dead surface in the player: eight tool cards (Equalizer+presets, Compressor, Spatializer, Stereo Widener, Visualizer, Colors, Crop, Essential, HDR) whose sliders/toggles/dropdowns are all local useState with no `send`, no `useClient`, no dispatch — `rg` across src/screens/player/tools/ finds zero command sends. Every control resets when the dialog closes ("fresh tool state each open"). The staging is disclosed per-card (ColorsTool.tsx:29-31: "no color-threshold/... engine stage or catalog command exists — local state + the visible staged markers, controls fully interactive") and the equalizer wrapper carries `data-pending-command="audio.set-eq"` — a command that does not exist in the generated catalog.

## 2. What actually works (verified)
— verified by ToolsDialog.test.tsx:196-204 (openToolsDialog dispatches the seam event, default audio / video) and :4 (trigger seam wiring). Tab primitive semantics real (roving tabindex). Baseline vitest 1705 passed.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/dialogs/ToolsDialog.tsx:116-138
Code:
```
        <TabsPanel value="audio" className="tools-dialog__panel">
          <div
            className="tools-dialog__staged-tool"
            data-staged-tool="equalizer"
            data-pending-command="audio.set-eq"
          >
```
What is wrong: the whole dialog is interactive theater — ~50 controls across 8 cards, zero engine commands. `rg "send\(|useClient" src/screens/player/tools/` returns nothing; the equalizer's awaited `audio.set-eq` is absent from the generated CommandRequest union (src/generated/commands.ts). Users who open "Advanced Settings" from the control bar can adjust an equalizer, compressor, spatializer, color correction, crop, HDR — and nothing anywhere changes, and the values are discarded on close. Why it matters: the ControlBar comments call this "the real A/V tools surface" — the surface is real, the tools are inert; the disclosure is a small role="note" line per card that a user adjusting 10 sliders will not have read. What correct looks like: engine stages + catalog commands (the cards' own pending markers name them), or collapse the cards to non-interactive staged rows (the settings/staged.tsx precedent) until the commands exist. Confidence: HIGH — Verification: read + rg dispatch sweep + generated catalog inspection.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| asTab | 57 | REAL | — | unknown→audio normalization |
| openToolsDialog | 63-65 | REAL | — | CustomEvent seam, tested |
| useToolsMsg | 78-81 | REAL | — | catalog-first lane seam |
| ToolsDialog | 99-142 | SHALLOW | S3 | F-001: shells 8 inert cards |
| ToolsDialogHost | 151-169 | REAL | — | listener half, per-open remount, escape/scrim close |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :196 seam event dispatch (default/explicit tab) | YES | event assertion |
| harness tests per tool | YES (for rendering) | but no test can fail on "slider does not command the engine" — nothing asserts sends because there are none |
Tool tests assert rendering + local state only — honest for what exists, but the gap between "fully interactive" and "controls nothing" is untested by design.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Both open the real A/V tools surface (w26.c5)" (ControlBar.tsx:695-696) | comment | MISLEADING | the dialog is real; its controls dispatch nothing (this finding) |
| "data-pending-command naming the awaited `audio.set-eq` command" (L17-18) | header | REAL (honest marker) | audio.set-eq not in generated catalog — the wait is open-ended |

## 7. Dead / duplicated / unreachable code
All 8 cards are reachable, interactive, and inert — the dead surface is the point (staged). EqualizerTool's `onChange` seam exists for future wiring (frozen w26.c1 contract per EqualizerTool.tsx:15).

## 8. Dependencies & flags
Tabs primitive, ToolsDialogShell, i18n lane keys. No transport imports anywhere in this subtree — the entire tools tree is backend-free by staging.

## 9. Open questions for the next agent
- Does any roadmap doc promise working EQ/compressor in v1? If a feature-matrix row claims audio-effects = shipped, this file is its S0 contradiction — matrix agents should check the audio-effects rows.
- The equalizer presets (presets.ts in my assignment) feed this dead control — audited next.
