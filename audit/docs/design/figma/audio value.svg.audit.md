# DATA AUDIT: design/figma/audio value.svg (credit: W19-A02, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: SVG Scalable Vector Graphics image, ASCII, 22 lines, longest line 2258 chars |
| Dimensions | width=121 height=37 viewBox="0 0 121 37" |
| Cited as evidence? | YES — cited by ui/src code comments (ValueChip.tsx:13, form.css:553, states.md:85, EQSlider.tsx:13) and docs/08-ui-figma-fidelity.md:169 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
Figma export of the dark "value chip" used on EQ/audio-value rows: single group id
`100 %` (text converted to outline paths), 121x37 rounded-6 well filled `#0E0E0E` at
0.95 opacity, plus a Figma backdrop-blur foreignObject/filter apparatus. The geometry and
color claims that the UI code makes against this file check out byte-exactly:
ui/src/components/form/form.css:553 — "frame: audio value.svg (121×37 rx6 #0e0e0e well)" —
matches SVG line 3 `<rect width="121" height="37" rx="6" fill="#0E0E0E" fill-opacity="0.95"
shape-rendering="crispEdges"/>`. ValueChip.tsx:13 "r6 recessed well, bright tabular value
text" is consistent (rx=6 ✓). docs/08-ui-figma-fidelity.md:169 lists it among component-
level exports — correct category. S4 only: filename contains a space ("audio value.svg"),
which forces shell quoting everywhere and invites path drift (four different consumers
spell it identically today — verified via `rg 'audio value\.svg'` hits above). Quotes
byte-verbatim from `sed -n '1p;3p'`. Design asset; nothing here executes.
