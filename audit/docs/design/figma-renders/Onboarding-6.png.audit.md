# DATA AUDIT: design/figma-renders/Onboarding-6.png (credit: W19-A07, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 900 x 627, 8-bit/color RGBA, non-interlaced (md5 0c2d6ce3...) |
| Dimensions | 900x627 onboarding step 4 (appearance · cyan accent) |
| Cited as evidence? | YES — w26.g02.md:15/:38/:52, w26.i01.md:66 (step3-appearance-cyan MATCH) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The cyan-appearance variant, twin of Onboarding-7/-9. Both of g02.md's pixel-level
claims about this frame re-verify by census (PIL):
- "Accent #18AAD3 exact on dot/Next/ring" — #18AAD3 present, 3735 px (Next CTA body;
  family-symmetric with blue 3734 / pink 3732 — same-size buttons across variants).
- "Reference PNG pixel sample at ring edge: `(16,54,65)` = cyan @30% on #0E0E0E"
  — the ring-edge color is present in the frame (9 px, edge sample as described),
  proving the frame's selected-dot ring exists (it was the APP that lacked it [P1-1]).
- Background census #090909 x370001 / #0e0e0e x121358 / #191919 x15544 — identical
  template to the blue/pink twins.
The g02 heading measurement (frame ≈24px type) is an SVG path-bbox derivation
(Onboarding-6.svg, 427.7x23.2) — consistent, not re-derived here.
No defects. Checked: dims, accent px census + family symmetry, ring-edge color presence,
template identity, 3 consumer citations.
