# AUDIT: ui/src/screens/player/tools/visualizer/VisualizerTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 111 / 1 component |
| File verdict | REAL (honestly STAGED tool: local state only, visible "staged" note, forward-declared command absent from the wire — engine-unwired by design) |
| Reachable from shipped entry point? | YES — ToolsDialog.tsx:130 `<VisualizerTool />` (no props) inside the player tools dialog; also composed in the settings effects area / music mode per header. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg mounts (ToolsDialog.tsx:38,130); rg audio.set-visualizer in generated/commands.ts (exit 1 = ABSENT); rg levels= suppliers (only harness.tsx:84 hard-codes demo levels); baseline ui-test.txt:58,120 — VisualizerTool 18 + harness 5 tests PASS |

## 1. One-paragraph verdict
This is the codebase's model citizen for unwired UI: the header states no visualizer engine stage or catalog command exists, the section carries `data-staged-tool` + `data-pending-command="audio.set-visualizer"`, a `role="note"` paragraph tells the user it's staged, the display renders RESTING frame geometry when no levels arrive (never simulated data), and the awaited command is genuinely absent from the wire union (rg exit 1). Production mount supplies neither `levels` nor `onChange` (ToolsDialog.tsx:130), so in the shipped app the mode dropdown is a fully interactive local toy — the honest framing keeps it out of fraud territory, but for the wave's master question this is another surface with zero backend: the audio visualizer does not visualize anything (no FFT source exists anywhere in ui/src) and no mode choice leaves the component.

## 2. What actually works (verified)
— verified by rg: staged markers, resting-decision, and PENDING_COMMAND all real; only the visual harness pushes levels (hard-coded 12-value array, harness.tsx:84).
— verified by baseline: 18 + 5 tests PASS (staged-note presence, mode select behavior, resting display).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/visualizer/VisualizerTool.tsx:33
Code:
```
/** Forward-declared command the mode selection awaits (staged marker). */
const PENDING_COMMAND = "audio.set-visualizer";
```
What is wrong: Not a bug — a recorded gap. "audio.set-visualizer" does not exist in the generated CommandRequest union (rg exit 1), no `levels` producer exists anywhere in ui/src, and the production mount passes no onChange. The tool cannot ever be wired without new engine surface; the marker correctly says so. Confidence: HIGH — Verification: ran rg (union absent, no levels source, bare mount).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VisualizerTool | 35-111 | REAL | S4 | staged by design (F-001) |
| ↳ commitMode | 50-53 | REAL | — | local set + optional seam call (never supplied in production) |
| ↳ render toggle/body | 55-108 | REAL | — | aria-expanded/controls; hidden body; staged note |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| VisualizerTool.test.tsx (18, baseline PASS) | YES | pins staged markers, mode cycling, resting display |
| harness.test.tsx (5) | YES | pins the levels-driven rendering path (demo data, labeled as such) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no visualizer engine stage or catalog command exists" | VisualizerTool.tsx:13 | TRUE | audio.* union: volume/mute/devices/delay only (commands.ts:24) |
| "absent levels render the mode's RESTING frame depiction, never simulated data" | VisualizerTool.tsx:8-10 | TRUE | VisualizerDisplay.tsx:86 `live ? levels : undefined` — resting path |

## 7. Dead / duplicated / unreachable code
None — the levels-driven path is harness-only but honestly framed as the host seam.

## 8. Dependencies & flags
VisualizerDisplay, VisualizerModeSelect, useVisualizerMsg. No flags.

## 9. Open questions for the next agent
- Does any engine-side plan (doc 05) list audio analysis / FFT? If not, this tool awaits a command nobody has specced.
