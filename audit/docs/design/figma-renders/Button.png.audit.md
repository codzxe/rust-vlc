# DATA AUDIT: design/figma-renders/Button.png
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A06) |
| Date | 2026-11-24 |
| Type / dims | `file` -> PNG image data, 900 x 315, 8-bit/color RGBA, non-interlaced |
| Verdict | REAL (real render of the Button primitive frame; verified-consumer chain) |

Depicts: the Button control-part grammar — source `design/figma/Button.svg` (400x140):
the button variants sheet (quiet/default/primary pill classes the a03 lane measured).
Render 900x315 (2.25x export scale).

Cited by:
- docs/waves/w26/CONTRACT.md:58 — Control parts row (`Button.png`)
- docs/waves/w26/logs/w26.a03.md:4,56,60 — the vision iteration ladder:
  iteration 2 GAPS (real defect) → iteration 3 **VERDICT: MATCH** vs Button.png
- docs/waves/w26/logs/w26.i03.md:47-48 — "Button.png's quiet button, measured live:
  **360×32 rx4 `#090909`**" (the tools-dialog footer grammar)
- docs/waves/w26/logs/w26.gfix-C.md:113 — "Button.png dialog-row density (P3-4)"

Consumer (ui/ component): `ui/src/components/button/Button.tsx` + button.css — the
`.lumen-button` grammar; the measured consumers are the tools-dialog footers
(`.lumen-button--quiet`, 360x32 rx4 #090909 per i03) and `.lumen-button--primary`
(accent pill, cross-verified in this batch via h05/k01 audits).

Findings: S4 only — 2.25x export scale undocumented; no embedded provenance. Asset
clean; the a03 iteration ladder (GAPS→MATCH) is the canonical fidelity record for the
button family.
