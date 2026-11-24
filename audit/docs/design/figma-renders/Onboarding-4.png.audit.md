# DATA AUDIT: design/figma-renders/Onboarding-4.png
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A08) |
| Type / dims | PNG RGBA 900×627 (file(1) + IHDR parse) |
| SVG counterpart | design/figma/Onboarding-4.svg (1090×759) — 0.826 scale family |
| Citations | ZERO — byte-safe python sweep of docs/ + ui/src finds no reference to "Onboarding-4" |

## 1. Blunt verdict
Real binary render, part of the 10-variant onboarding appearance set
(e01.md's mapping lists "Onboarding-3..10.png (appearance ×8 accent
variants ... 4=yellow)"). This particular variant is cited by NO file —
the accent variants are interchangeable frame references, only some of
which individual lanes measured. No matrix evidence row cites any render.
No fabrication exposure.

## 2. Checks run
- `file` -> PNG image data, 900 x 627, 8-bit/color RGBA, non-interlaced (exit 0)
- IHDR -> (900, 627); counterpart SVG 1090×759 (same scale family as
  Onboarding-1/3 renders — 0.826).
- python byte-safe grep for "Onboarding-4" over docs/** and ui/src/** ->
  0 hits (the rg 0-hit result was re-verified without display-pipeline risk).

Findings: none. Short data audit per template 12.2.
