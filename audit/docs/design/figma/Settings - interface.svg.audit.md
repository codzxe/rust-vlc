# CLAIM AUDIT: design/figma/Settings - interface.svg
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 1 (asset is a real design-source artifact, cited as frame evidence) |
| REAL / SHALLOW / STUB / PAPER-ONLY / FALSE | 1 REAL (asset) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file "design/figma/Settings - interface.svg"` -> SVG, longest line 15873 chars. Design-source artifact, settings interface frame; not code, not a build input.
Cited as frame evidence by ui/src/screens/settings/areas/index.tsx:240 + interface.tsx:13 (frame composes Skin/Window sections).
No fabrication indicator: the citations use the asset as a measurement reference
(dimension/glyph/row comparisons), not as proof of shipped behavior. Nothing in any
cited file claims this asset executes.
## 2. Reachability
N/A (static design asset; consumed by docs/UI comments as the design-of-record, e.g. by
the citations above; no matrix row's `evidence` field names it — matrix evidence is
code-path-shaped).
