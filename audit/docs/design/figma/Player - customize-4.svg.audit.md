# DATA AUDIT: design/figma/Player - customize-4.svg
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 646 lines, ASCII SVG; 1920x1080 canvas; card radii rx6 (x4), rx4 (x3), rx14 (x2) |
| File verdict | REAL (Figma export) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (bloat class of the family) |
| Commands run | head/rg censuses; consumer rg |

## 1. What it depicts
Frame "Player - customize" variant 4 of 9: the customization panel OPEN state — the panel
carries the verbatim instruction text "Drag items around and out of the box below to
re-order them. Drag items into the box to remove them from the player." plus the panel
title "Customize Player". Native 1920x1080 export, one pattern-fill media backdrop,
card-radius grammar rx6 consistent with the tools-settings analysis (r6 card rule).

## 2. Consumer / fidelity
- Consumer: ui/src/screens/player/customize/types.ts:1-2 — the customize surface data
  model is explicitly "rebuilt to the 9-state frame set `design/figma/Player -
  customize*.svg`", with the derived constraint at types.ts:14 ("NO adjustment sections
  exist in any customize frame"). The variant feeds the shipped customize surface's
  state model.
- No PNG twin exists for variant 4 (figma-renders carries -3 and -6 only) — the w26
  visual-differential lanes did not screenshot this state; SVG is its sole evidence layer.
- Fidelity: clean vector export, ASCII text, no embedded raster beyond the media pattern.

## 3. Verdict
REAL asset with a live code consumer; S4 only for the family-wide export bloat class.
