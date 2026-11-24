# DATA AUDIT: design/figma/option.svg (credit: W19-A02, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: SVG Scalable Vector Graphics image, ASCII, 21 lines, longest 5158 chars |
| Dimensions | width=301 height=292; row plates 261x48 rx6 |
| Cited as evidence? | YES — w26 findings/log citations + design/README.md:24 + figma-fidelity-spec.md:96 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
Figma export of the menu "option" row in its three states. Checked, with method:
1. State grammar claim (docs/waves/w26/findings/w26.g19.md:54 "Default transparent / hover
   `#1F1F1F` / selected `#1F1F1F` + check") vs the SVG: `Property 1=Default` group carries
   no rect (transparent) ✓; `Property 1=hover` rect `fill="#1F1F1F"` ✓; `Property 1=selected`
   rect `fill="#1F1F1F"` + check glyph ✓ (`sed -n '8p;12p;16p'`).
2. Check glyph: `Vector 44` = `M248 179.5L252.667 185L262 174` stroke 2 `#D37918`; g19.md:49
   "14×11 stroke2 #D37918" — 262-248=14, 185-174=11, stroke-width 2, stroke #D37918 ✓
   (x228–242 is row-relative: 248-262 minus row x=20 ✓).
3. Check color traced into shipped CSS: ui/src/theme/lumen.css:197 mentions `#d37918`
   contrast measurement — the accent is real in code.
4. Category claims: design/README.md:24 and design/figma-fidelity-spec.md:96 list it under
   menu/component exports — matches content (menu-row states).
No defects found; every claim made against this file reconciles to its geometry. Design
asset; nothing here executes.
