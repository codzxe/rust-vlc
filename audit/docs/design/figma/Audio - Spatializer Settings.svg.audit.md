# CLAIM AUDIT: design/figma/Audio - Spatializer Settings.svg
| Field | Value |
|---|---|
| Auditor | W19-A06 (recovered by W19-R3) |
| Claims extracted | 1 (asset integrity as design-of-record) |
| REAL 1 / others 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file` -> "SVG Scalable Vector Graphics image, ASCII text, with very long lines (16570)",
65,836 bytes, root 400x821 viewBox, `<g id="Audio - Spatializer Settings">` with
collapsed/expanded state groups — genuine Figma export; not code, not a build input.
Design-of-record for the Spatializer tool: cited by CONTRACT.md:53 (audio-tools row),
design/README.md, the shipped code (SpatializerTool.tsx + test, ToolSliderRow.tsx;
dynamics.css:6 "expanded 360×H rx6 #1B1B1B, 59px [header]", :199/:219 frame-marked tick
grammar "Vector 48-62"), lane logs c2/c5/i07/k10, review j10, and the verifier chain
(w26.e10.md:42 "5 rows @ pitch 76; degree ticks [10,9,9,9,9] — exactly the frame's
data-driven marks").
No fabrication indicator: all citations are reference/measurement uses; raw hex inside
a design export is correct (shipped CSS carries tokens + frame-provenance comments).
## 2. Reachability
N/A (static design asset; no matrix row evidence field names it).
Commands: file, stat, head -c 200, rg citation sweep (10+ citing files; key lines pasted).
