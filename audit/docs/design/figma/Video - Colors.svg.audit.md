# CLAIM AUDIT: design/figma/Video - Colors.svg
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R3) |
| Claims extracted | 1 (asset integrity as design-of-record) |
| REAL 1 / others 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file` -> "SVG Scalable Vector Graphics image, ASCII text, with very long lines (16570)",
153,370 bytes, root 400x888 viewBox, `<g id="Video - Colors">` with collapsed/expanded state
groups (Property 1=collapsed rect 360x48 rx6 #090909 — matches the shipped video.css:5
provenance comment verbatim: "collapsed 360×48 rx6 #090909 · expanded 360×755").
Genuine Figma export; not code, not a build input. Heavily used as design-of-record:
design/README.md:22 (video tools row), ui/src/screens/player/tools/video/frameData.ts:9,13,
video.css:5,34,85,117,139,371 (six `frame: "Video - Colors.svg"` provenance comments),
w26.g17.md:6 (parsed as SVG ground truth for a DOM-measured geometry audit),
w26.e15.md:13 (frame-provenance census), w26.g08.md:119.
No fabrication indicator: citations are all reference/measurement uses; raw hex inside a
design export is correct (the shipped CSS carries the token-lane handoffs; the repo's
eslint-plugin-local semantic-tokens-only rule polices that boundary and cites this asset).
## 2. Reachability
N/A (static design asset; no matrix row evidence field names it).
Commands: file, stat, head -c 400, rg -n citations (15+ hits pasted).
