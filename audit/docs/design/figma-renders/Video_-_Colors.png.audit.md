# DATA AUDIT: design/figma-renders/Video_-_Colors.png
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A02) |
| Date | 2026-11-24 |
| Type / dims | `file` -> PNG image data, 900 x 1998, 8-bit/color RGBA, non-interlaced |
| Verdict | REAL (real render asset, byte-verified type; cited by 5+ docs) |

Depicts: the "Video - Colors" video-tools panel Figma export — collapsed 360x48 rx6 #090909
card plus the expanded panel (Color Extraction / Color Threshold hex rows with 22x22
swatches, label-only Saturation/Similarity/Sepia sliders). Source frame `design/figma/
Video - Colors.svg` is 400x888; the render is 900x1998 = 2.25x export scale (same export-
scale class W19-R7 recorded for playlist.png).

Cited by:
- docs/08-ui-figma-fidelity.md:168 "video tools | `Video - Colors/Crop/Essential.svg`, `HDR Options.svg`" (fidelity map row)
- design/README.md:22 (Video tools frame inventory)
- docs/waves/w26/CONTRACT.md (Video tools evidence contract)
- docs/waves/w26/findings/w26.e11.md ("MATCH [VERIFY]" F2/F3/F5/F6 deltas only)
- docs/waves/w26/findings/w26.l02.md row 23 "colors-open-1440 MATCH (measurement)"
- docs/waves/w26/findings/w26.g17.md ("GAPS (rhythm)" — thumb 20.7/20.7/34.3% vs frame 33.5)

Consumer (ui/ component): `ui/src/screens/player/tools/video/ColorsTool.tsx` — "w26.c4 —
ColorsTool (frame: \"Video - Colors.svg\", 400×888)" — plus `tools/video/harness.tsx`, which
renders the built tool against this PNG (golden-compare target). ColorsTool.test.tsx derives
row composition from the same frame.

Findings: S4 only — the render itself carries no embedded provenance (no timestamp/commit),
and its 2.25x scale is undocumented in the citing docs (g17 measures against frame values,
not the PNG). No defect in the asset.
