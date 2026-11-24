# DATA AUDIT: design/figma/Open network stream.svg
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A07) |
| Type | SVG ASCII, file(1) verified, 70 element lines |
| ViewBox / window | 1048×631, rx14, fill `#191919` @0.95, bg-blur scrim filter |
| Matrix evidence citations | NONE directly (no row cites the .svg; rows cite the *implementation*) |

## Blunt verdict
Real design frame for the W26 network-stream dialog (w26.b15). The
implementation header comments it 1:1:
ui/src/screens/network-stream/NetworkStreamScreen.tsx:1-3 —
"// design/figma/Open network stream.svg (1048×631 rx14 #191919 window on the
// shared scrim), ported 1:1 at the frame's own scale" — frame geometry
(1048×631, rx14, #191919) byte-matches the SVG's own rect attrs (sed-verified
above). feature-matrix rows for open-network-stream cite the .tsx, not this
asset; no fabricated-evidence exposure.

## Checks run
- `file` -> SVG Scalable Vector Graphics, ASCII (exit 0)
- `head -3` -> width=1048 height=631 viewBox "0 0 1048 631" rx="14" fill="#191919" fill-opacity="0.95"
- `rg` citations: NetworkStreamScreen.tsx (+css/+test), dialogs, convert,
  open-media screens; `rg 'network-stream' docs/feature-matrix.toml` -> 2
  evidence rows pointing at code files, none at this SVG.

Findings: none. Short data audit per template 12.2.
