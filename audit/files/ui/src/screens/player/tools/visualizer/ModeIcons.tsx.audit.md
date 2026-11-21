# AUDIT: ui/src/screens/player/tools/visualizer/ModeIcons.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 5 (1 shared shell + 4 icon components) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — the four icons are the per-mode icons in types.ts VISUALIZER_MODES (types.ts:58-73), rendered by the mode selector inside the Tools dialog's visualizer card (ToolsDialog.tsx:130). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Grep 'SpectrometerIcon|ScopeIcon|SpectrumIcon|WaveformIcon' src (consumers: types.ts:23,58-73 → selector; re-exported index.ts:11-14); geometry cross-check of every path against its viewBox (hand-computed, below). |

## 1. One-paragraph verdict
Verbatim frame-geometry icon ports, internally consistent: every path coordinate was checked against its viewBox bounds (all bars/dots/arcs inside bbox + 0.75 stroke halo), the zero-length round-cap paths correctly render as dots, and the tone classes map to the semantic tokens consumed by visualizer.css. One comment lies about its own geometry (the Spectrum heights list counts a dot position as a bar height). Reachable, consumed, correct.

## 2. What actually works (verified)
— Geometry audit (hand-computed): Scope bars 5/14/12/16/13/8 px within viewBox y 198.25..215.75 ✓; Spectrum dots at 253/244/246/242/245/250, bars 5/14/12/16/13/8, all inside 241.25..261.75 ✓; Waveform symmetric bars (half-heights 2/7/5/9/6/4) inside 288.25..307.75 ✓; Spectrometer circles r4/r7 + arc path within 149.25..170.75 ✓. Baseline: visualizer suites pass (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/tools/visualizer/ModeIcons.tsx:97-98
Code:
```
 * Spectrum — frame Icon_5/Frame 1_4: six gray bars from the baseline (y261,
 * heights 5/14/12/19/13/8) with an accent peak dot floating above each top.
```
What is wrong: the actual bar heights are 5/14/12/16/13/8 (261−245=16 for the tallest bar, line 122); 19 is the distance from the baseline to the fourth PEAK DOT (261−242), not the bar height. The bar geometry itself matches ScopeIcon's bars exactly (as the frame ships them).
Why it matters: a comment that misstates its own file's geometry undermines the "ported VERBATIM from the frame" provenance claim — verifiability is the whole point of these headers.
What correct looks like: "bars 5/14/12/16/13/8, dots 3px above each top".
Confidence: HIGH — every coordinate read and recomputed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `iconShell` | 18-40 | REAL | — | shared svg shell; aria-hidden + focusable=false correct for decorative icons |
| `SpectrometerIcon` | 46-68 | REAL | — | circles r4/r7 gray + arc-with-heads accent; bbox verified |
| `ScopeIcon` | 74-94 | REAL | — | bars alternate gray/accent; pitch 4px verified |
| `SpectrumIcon` | 100-127 | REAL | S4 (F-001) | geometry correct; comment misstates one height |
| `WaveformIcon` | 133-153 | REAL | — | midline-centered bars; symmetry verified |

## 5. Test honesty
No tests in this file. harness.test.tsx:97 references the visualizer icon calibration (other agent). Mutation: any coordinate change would only be caught by pixel-level visual tests, which the harness runs in the visual lane (not in the vitest baseline) — coordinates are effectively untested in CI. Noted, not a finding against this file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every path below is the frame's own geometry, cropped per icon into its measured viewBox" | ModeIcons.tsx:3-4 | REAL-UNPROVEN | internally consistent + halo margins correct; the Figma source itself not accessible for byte-comparison |
| "heights 5/14/12/19/13/8" (Spectrum) | ModeIcons.tsx:98 | FALSE (comment vs own geometry) | F-001 |

## 7. Dead / duplicated / unreachable code
None — all four icons consumed via types.ts VISUALIZER_MODES and re-exported.

## 8. Dependencies & flags
visualizer.css tone classes only. No imports beyond React types.

## 9. Open questions for the next agent
- types.ts owner: VISUALIZER_MODES order (disabled/spectrometer/scope/spectrum/waveform) — does the dropdown's frame order match ("the frame's five modes")?
