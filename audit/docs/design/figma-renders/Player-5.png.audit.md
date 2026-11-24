# CLAIM AUDIT: design/figma-renders/Player-5.png
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R3) |
| Claims extracted | 1 (asset integrity as frame evidence) |
| REAL 1 / others 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file` -> "PNG image data, 900 x 506, 8-bit/color RGB, non-interlaced", 48,884 bytes.
Genuine design-source render of the Player-5 "home" state (the remembered-openings idle
face); not code, not a build input. Cited as frame evidence across the verification chain:
w26.gfix-C.md:27 (P1-7 home face implemented over "the Player-5 card grammar
(237×133 r9 / 150×150 @×0.75)"), w26.l01.md:41 (home face GAPS row) and :52
([l01-P1-1] "Player-5 home face is unreachable" — an honest negative finding citing
this asset), w26.g04/e02/h04/h-scoreboard. Shipped code cites the SVG twin
(states/states.css, StageOverlays.tsx).
No fabrication indicator: consumers use it as measurement/visual reference; the l01 lane
even used it to report a REACHABILITY defect against, not to claim success. No matrix
row's evidence field names it.
## 2. Reachability
N/A (static design asset; the home-face surface it depicts is shipped-UI reachable per
gfix-C P1-7).
Commands: file, stat, rg citation sweep (7 citing files; gfix-C:27/:52, l01:41/:52
pasted).
