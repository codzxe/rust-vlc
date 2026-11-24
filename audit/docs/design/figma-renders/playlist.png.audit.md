# DATA AUDIT: design/figma-renders/playlist.png (credit: W19-A08, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Type / size | `file`: PNG image data, 8-bit/color RGBA, non-interlaced; 18 632 bytes |
| Dimensions | 900 x 544 (verified twice: `file` + IHDR parse) |
| Cited as evidence? | YES — w26 wave docs cite `playlist.png` ~15 times |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## Verdict
Design-reference render of the playlist/library screen (Figma export). It is working
evidence in the w26 fidelity lane: docs/waves/w26/logs/w26.a09.md:48-50 uses it across three
iterations (GAPS -> GAPS -> MATCH: "icon slot+glyph, title typography/gray tone, centering,
hover brighten + kebab reveal all conform"). docs/waves/w26/findings/w26.g18.md:10 pairs
"`playlist.png` / `playlist.svg`" with "(377×228: 2 row states ...)" — 377x228 is the SVG
frame geometry, not this PNG's; the PNG is 900x544 (2.39x scale), and a09.md:49 itself
records that "scale artifact (render = 900px for a 377-unit frame, 2.39x)" and claims the
harness was fixed — yet this checked-in render is still the 900px one (S3: dimension
mismatch between the cited claim's implied geometry and the shipped asset; no 377px render
present in design/figma-renders/). No code path loads this PNG (design asset, not runtime).
Verified: `file design/figma-renders/playlist.png` -> "PNG image data, 900 x 544, 8-bit/color
RGBA, non-interlaced"; `rg 'playlist\.png' docs` -> 15+ hits listed above.
