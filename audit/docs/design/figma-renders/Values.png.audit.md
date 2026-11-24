# CLAIM AUDIT: design/figma-renders/Values.png
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 1 (asset is a real design-source artifact, cited as frame evidence) |
| REAL / SHALLOW / STUB / PAPER-ONLY / FALSE | 1 REAL (asset) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file "design/figma-renders/Values.png"` -> PNG 900x1437 RGBA. Design-source artifact, values segmented-control frame; not code, not a build input.
Cited as frame evidence by ControlBar.tsx:525 + ControlBar.test.tsx:1006 (Repeat as segmented control).
No fabrication indicator: the citations use the asset as a measurement reference
(dimension/glyph/row comparisons), not as proof of shipped behavior. Nothing in any
cited file claims this asset executes.
## 2. Reachability
N/A (static design asset; consumed by docs/UI comments as the design-of-record, e.g. by
the citations above; no matrix row's `evidence` field names it — matrix evidence is
code-path-shaped).
