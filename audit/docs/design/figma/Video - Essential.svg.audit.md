# CLAIM AUDIT: design/figma/Video - Essential.svg
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R3) |
| Claims extracted | 1 (asset integrity as design-of-record) |
| REAL 1 / others 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file` -> "SVG Scalable Vector Graphics image, ASCII text, with very long lines (16570)",
120,648 bytes, root 400x821 viewBox, `<g id="Video - Essential">` with collapsed/expanded
state groups — genuine Figma export; not code, not a build input. Design-of-record for the
Essential video tool: cited by shipped code in ui/src/screens/player/tools/video/
(video.css:14 "8 label-only slider rows @ 76px pitch" provenance, :35, frameData.ts,
EssentialTool.tsx + its tests, ToolsDialog.tsx) and by the verification chain
(w26.g17.md:6 parsed it as SVG ground truth; w26.g04/w26.j11/w26.h15 also cite it;
design/README.md:22 video-tools row).
The 8-rows-at-76px-pitch claim this asset anchors is the same grammar w26.e11.md:66
measured live ("8 rows, all 7 deltas = 76px, ticks [1,1,1,2,9,3,6,9]").
No fabrication indicator: all citations are reference/measurement uses; raw hex inside a
design export is correct (token-lane handoffs live in the shipped CSS comments).
## 2. Reachability
N/A (static design asset; no matrix row evidence field names it).
Commands: file, stat, head -c 240, rg citation sweep (12+ citing files).
