# DATA AUDIT: design/figma/Values-1.svg (credit: W19-A02, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Type / size | `file`: SVG Scalable Vector Graphics image, ASCII, 35 lines, longest line 15901 chars |
| Dimensions | width=360 height=216 viewBox="0 0 360 216" |
| Reachable from shipped entry point? | Not executed code; design asset consumed as reference only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## Verdict
Figma export of a settings "Values" component: two variant groups `Property 1=Default` /
`Property 1=Variant2`, each with option rows (`1`, `2`), texts `Always starts at:`,
`Remember last volume`, and value label `200%` (text converted to outline paths, ids carry
the original strings). One S3: docs/08-ui-figma-fidelity.md:139 labels this file a "Value
steppers" with variants "`option=1/2, number=2/3`" — the SVG contains no stepper geometry
and no such variant ids; variant ids are `Property 1=Default`/`Property 1=Variant2`
(verified `rg -o 'id="[^"]*"'` output above). It is a radio-row pair, not a stepper.
Consumed as design reference by ui/src/screens/settings/areas/audio.tsx:11
("Startup volume = the Values-1 radio rows (a19 B5: \"Remember last ..."). Internal
consistency of labels vs audio.tsx claim: plausible, not machine-verified (design asset).
Verified quotes: line 1 `<svg width="360" height="216" viewBox="0 0 360 216" fill="none" ...>`;
line 12 path id `Always starts at:` (sed -n '12p' | cut -c1-120). Nothing here executes.
