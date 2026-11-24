# AUDIT: ui/src/screens/player/tools/visualizer/visualizer.css
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 319 / n.a. (pure stylesheet, 15 class selectors) |
| File verdict | REAL-UNPROVEN (presentational; wired; no dead selectors) |
| Reachable from shipped entry point? | YES — imported by VisualizerTool.tsx:30 `import "./visualizer.css"`; the tool mounts in the player Tools dialog (reachable via /player canvas context menu) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 dead-selector sweep (15/15 used); grep import chain; read of file |

## 1. One-paragraph verdict
Pure presentation for the audio-visualizer tool card, token-driven with frame-measured px provenance comments. All 15 `.viz-*` selectors are referenced by the visualizer components. Two honest `[VERIFY]` markers live in comments (display well height 64px is "a house choice"; toggle font weight) — open design questions, not defects. Context for the master question (component lane owns the finding): the visualizer display itself is honest-static — "no FFT / audio-analysis source exists yet" (VisualizerDisplay.tsx:3) — the bars are the frame's resting depiction, never animated or simulated.

## 2. What actually works (verified)
— verified by python3 sweep (dead: []) and grep import chain (VisualizerTool.tsx:30).

## 3. Findings
None. Checked: dead selectors (none), unimported sheet (imported), raw hex escapes (none — hexes live in comments only), `!important` (none).

## 4. Per-function table
n.a. — stylesheet. Selector families: `.viz-tool*` (card), `.viz-tool__display*` (levels well), plus ModeIcons two-tone classes.

## 5. Test honesty
harness.test.tsx reads the CSS file (lines 28, 92) — a visual-harness existence check, not a behavior test. Fine for CSS.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only ... Raw px below are frame-measured ... each cited" | header L21-24 | REAL | full read: every raw px carries a `frame:` or `[VERIFY]` citation |
| "the well height carries no frame measure [VERIFY]" | header L24 | HONEST-UNKNOWN (disclosed) | comment itself |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Semantic tokens; shared Dropdown/tool-card grammar. No feature flags.

## 9. Open questions for the next agent
- VisualizerTool.tsx (not my file): confirm `levels` is never passed by any caller (no spectrum event source), making the tool a static depiction in product.
