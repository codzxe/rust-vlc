# AUDIT: ui/src/screens/settings/areas/controls.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 63 / 5 rule blocks (scoped skin) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (controls.tsx:45 imports it; route /settings/controls via areas registry -> ControlsArea) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg selector targets (all 4 data-section values exist); rg RemotesSection staged state; sed quote verification |

## 1. One-paragraph verdict
Compact, frame-cited geometry skin for the Controls area: per-section row
pitches, title margins, and trailing gaps, every value carrying a `frame:`
comment with SVG-measured y-coordinates. All selectors resolve to live DOM
(data-testid="controls-area" at controls.tsx:183; sections keyboard-shortcuts
:184, remotes :120, skip-amounts + mouse in generated/Screens.tsx:187,190).
Hostile note: the sheet spends 10 lines pixel-perfecting the REMOTES section
whose rows are permanently inert — staged disabled controls awaiting a
"Phase 5+" control server (product.toml [features] control-server = false,
controls.tsx:92-98). Dead surface, dressed to the frame.

## 2. What actually works (verified)
— rg: all four [data-section=...] targets present in the rendered tree.
— Specificity strategy (14-16): .settings-sections + .settings-section chains
  outrank the base sheet's :last-child zeroing — the stated fix for the
  −8/−42px drift is coherent.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/controls.css:18-24 (Remotes rows pitch)
Code:
```
/* Remotes rows — frame: 793→837 = 44 pitch (the frame's own ±2; the sheet
 * switch pitch is 42). +2 on the web-remote row lands Port on y837. */
```
What is wrong: Pixel-perfect rhythm for rows that can never be interacted
with (RemotesSection renders disabled staged controls, controls.tsx:100-121 —
"no controls.web-remote / controls.port [[setting]] keys exist"). The CSS is
honest skin work; the surface it perfects is dead until a control server
exists. Cosmetic-investment note, not a CSS defect.
Confidence: HIGH — Verification: read + rg staged markers.

## 4. Per-function table
Not applicable (<100 LOC; 5 selector blocks, all REAL and live).

## 5. Test honesty
No tests in file; controls area tests owned elsewhere.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the shipped sheet landed the top two blocks pixel-exact but drifted −8 (Mouse) and −42 (Remotes)" | :6-10 | REAL-UNPROVEN | arithmetic coherent with the fixes below; frame SVG not re-measured (Figma file not available) |

## 7. Dead / duplicated / unreachable code
No dead selectors. The Remotes skin dresses a permanently staged section
(F-001).

## 8. Dependencies & flags
settings.css base sheet (pitch/last-child rules it overrides). No flags.

## 9. Open questions for the next agent
controls.tsx owner (not my file): confirm the staged rows' tooltips render
(they claim keyboard-reachable reasons via aria-describedby + sr-only twin).
