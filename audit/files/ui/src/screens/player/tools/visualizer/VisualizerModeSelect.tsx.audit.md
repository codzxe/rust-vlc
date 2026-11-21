# AUDIT: ui/src/screens/player/tools/visualizer/VisualizerModeSelect.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 84 / 1 (VisualizerModeSelect) |
| File verdict | REAL |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (VisualizerTool.tsx imports + renders it; ToolsDialog/music-mode mounts per VisualizerTool header). ENGINE: NO — no visualizer engine stage or FFT source exists (VisualizerTool header: "no FFT source — absent levels render the mode's RESTING frame depiction"); the mode selection drives local state only. SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg VisualizerModeSelect consumers (VisualizerTool, harness, index); sed VisualizerTool.tsx:1-30 + VisualizerDisplay.tsx:1-50 (levels seam, resting state); VisualizerTool.test.tsx survey 35-126 |

## 1. One-paragraph verdict
An exemplary thin composition: the lane's former 230-line private combobox
was deleted in favor of the shared Dropdown primitive, with only the
frame's icon previews (iconNode) and CSS scope added. The component does
exactly its contract — five options in frame order, two-tone icons, commit
seam, structural data-mode markers — and its tests execute it through the
parent tool. The single finding is inherited staging: the mode the user
picks drives a display that has no data source, so every mode renders its
resting depiction forever.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): VisualizerTool.test
74-126 exercises THIS component — five frame options in order with two-tone
icons, whole-list reveal, select-on-click updating the trigger label and
the onChange seam.
— verified by read: options built from VISUALIZER_MODES (types.ts) with
iconNode wired (64), structural `data: { mode }` for test identity (67);
the `next as VisualizerMode` cast (77) is safe because options are built
from the same VISUALIZER_MODES ids.
— verified by read: no duplication of the Dropdown grammar remains (the
header's claim matches the 84-line reality vs. the retired 230-line copy).

## 3. Findings
### F-001 [S4] The selected mode has no data lane — every mode renders its resting depiction in all shipped mounts
Where: ui/src/screens/player/tools/visualizer/VisualizerModeSelect.tsx:76-78
Code:
```
      onChange={(next) => {
        onChange(next as VisualizerMode);
      }}
```
What is wrong: The commit seam fires correctly, but its only consumer
(VisualizerTool) holds local state, and the display's `levels` seam has no
host pushing data (no FFT source exists). Selecting "Spectrum" vs "Scope"
changes only which RESTING depiction is shown — honest (never simulated
data, VisualizerDisplay.tsx:5-6) but inert.
Why it matters: One more interactive selector in the tools dialog whose
effect is cosmetic; the honesty markers live on the parent card (staged
note + data-pending-command, VisualizerTool.tsx:61-62,105-106), so this
child needs no own marker — but the family verdict (my DynamicsCard/
VideoCard S3s) applies.
What correct looks like: An analysis source (engine levels/FFT event or
WebAudio tap on the url-source <video>) feeding the levels seam.
Confidence: HIGH — Verification: read (no levels provider exists — rg
`levels=` finds only the prop declaration and tests).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VisualizerModeSelect | 52-84 | REAL | S4 (F-001 inherited) | thin composition verified; cast safe; props frozen API documented |

## 5. Test honesty (via VisualizerTool.test.tsx:74-126)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| five frame options in order + two-tone icons (83-99) | YES | option order + iconNode render |
| whole-list reveal to scrolling hosts (100-124) | YES | the w26.i07 fold fix through the shared primitive |
| select on click: trigger label + display + onChange (126+) | YES | the commit seam pinned |
Mutation check: drop `data: { mode }` → row-identity assertions fail; swap
option order → test 83 fails. Honest suite.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the lane-scoped re-implementation … is GONE — this is now a thin composition over the shared Dropdown" | header 2-4 | REAL | 84 lines, no combobox logic |
| "the primitive carries the full combobox interaction model" | header 6-10 | REAL | Dropdown.tsx (other agent) + tests through it |
| "absent levels render the mode's RESTING frame depiction, never simulated data" | VisualizerTool header 8-9 | REAL | VisualizerDisplay.tsx:5-6 read |

## 7. Dead / duplicated / unreachable code
None — the duplication was already deleted (header claim verified by the
file's size and shape).

## 8. Dependencies & flags
components/menu Dropdown, ./types. No feature flags, no client import.

## 9. Open questions for the next agent
- The `levels` seam accepts "one array per animation frame" — when an
  analysis source lands, who owns the frame loop (host hook vs. the
  display)? Perf-relevant (60fps allocations).
