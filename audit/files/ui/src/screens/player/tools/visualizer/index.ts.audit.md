# AUDIT: ui/src/screens/player/tools/visualizer/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 30 / 0 (pure barrel: tool + display + mode select + icons + types) |
| File verdict | REAL-UNPROVEN (barrel live; exported tool is staged-static in product) |
| Reachable from shipped entry point? | YES — ToolsDialog.tsx:38 imports VisualizerTool from this barrel (player Tools dialog, audio tab) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; grep consumer; read VisualizerTool.tsx header + levels wiring |

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/visualizer/index.ts:5-6 + VisualizerTool.tsx:8-9,20 + ToolsDialog.tsx:38 (no levels prop)
Code (barrel header):
```
// may host the display standalone). All state is local to the card; the
// mode selector is honestly staged (see VisualizerTool); `levels` is the
// host-owned data seam.
```
What is wrong: the barrel's own header concedes the staging: no FFT/analysis source exists, `levels` is a host seam no host feeds (ToolsDialog mounts the tool bare), so the shipped visualizer is a mode dropdown that swaps between STATIC frame depictions — an "audio visualizer" that never visualizes audio. Interactive local state, visibly marked staged (role="note" + data-pending-command), never simulated — honest, but census: another dead-end tool surface. Confidence: HIGH — Verification: read + grep (no levels= at any mount).

## 1. One-paragraph verdict
Clean re-export barrel (tool, display, mode select, icons, resting-level data, types). The exported product surface is honest-static: without a `levels` source the display renders the frame's resting depiction per selected mode, and the only shipped mount (ToolsDialog audio tab) supplies none.

## 2. What actually works (verified)
— verified by grep: ToolsDialog.tsx:38 consumer; VisualizerTool passes `levels={levels}` through to VisualizerDisplay (L93); mode select interactive locally.

## 4. Per-function table
n.a. — barrel (<100 LOC, compact).

## 5. Test honesty
VisualizerTool.test.tsx + harness exist (visualizer lane).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no FFT source — absent levels render the mode's RESTING frame depiction, never simulated data" | VisualizerTool.tsx:8-9 | REAL | no levels source at any mount |

## 7. Dead / duplicated / unreachable code
None in the barrel; the dead-end is product-level (F-001).

## 8. Dependencies & flags
None (re-exports).

## 9. Open questions for the next agent
None.
