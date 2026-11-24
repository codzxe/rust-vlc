# AUDIT: ui/src/screens/player/tools/visualizer/ModeIcons.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 153 / 5 (shell + 4 icon components) |
| File verdict | REAL (verbatim frame-ported two-tone SVG icons; consumed by the mode list) |
| Reachable from shipped entry point? | YES — types.ts:58-73 attaches the four icons to VISUALIZER_MODES; VisualizerModeSelect.tsx:64 renders `<mode.icon />`; select mounts in VisualizerTool → ToolsDialog → player. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg tone classes (visualizer.css:215-219 both exist); rg icon consumers (types.ts:23,58-73; VisualizerModeSelect.tsx:64; index.ts:11-15 barrel) |

## 1. One-paragraph verdict
Pure presentational SVG, no logic to break: one shared shell (aria-hidden, focusable=false, frame stroke 1.5/round caps) and four frame-verbatim icons whose colors ride the semantic tone classes `.viz-tone-gray`/`.viz-tone-accent` (both defined in visualizer.css:215-219). Consumers verified end-to-end. Zero findings.

## 2. What actually works (verified)
— verified by rg: tone classes defined; icons attached to mode defs and rendered by the mode select; barrel re-export intact.

## 3. Findings
None. Checked: dead exports (none — all four consumed), token leaks (none — classes only), a11y (decorative, aria-hidden), stroke geometry constants (frame-cited comments).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| iconShell | 18-40 | REAL | shared svg shell |
| SpectrometerIcon | 46-68 | REAL | circles + arcs, frame-cited |
| ScopeIcon | 74-94 | REAL | 6 bars, alternating tones |
| SpectrumIcon | 100-127 | REAL | zero-length round-cap strokes as peak dots (clever, frame-accurate) |
| WaveformIcon | 133-153 | REAL | midline-centered bars |

## 5. Test honesty
VisualizerTool.test.tsx (baseline PASS) covers the tool that renders these; icon geometry is visual-harness territory.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Colors are SEMANTIC TOKENS via the two tone classes in visualizer.css" | ModeIcons.tsx:6-9 | TRUE | visualizer.css:215-219 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None beyond React types. No flags.

## 9. Open questions for the next agent
- None.
