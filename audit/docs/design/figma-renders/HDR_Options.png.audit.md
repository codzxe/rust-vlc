# CLAIM AUDIT: design/figma-renders/HDR_Options.png
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 1 (asset is a real design-source artifact, cited as frame evidence) |
| REAL / SHALLOW / STUB / PAPER-ONLY / FALSE | 1 REAL (asset) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file "design/figma-renders/HDR_Options.png"` -> PNG 900x935 RGBA. Design-source artifact, HDR options tool frame; not code, not a build input.
Cited as frame evidence by w26.i08.md:79 (Auto at 16px inset) + w26.CONTRACT.md:54.
No fabrication indicator: the citations use the asset as a measurement reference
(dimension/glyph/row comparisons), not as proof of shipped behavior. Nothing in any
cited file claims this asset executes.
## 2. Reachability
N/A (static design asset; consumed by docs/UI comments as the design-of-record, e.g. by
the citations above; no matrix row's `evidence` field names it — matrix evidence is
code-path-shaped).
