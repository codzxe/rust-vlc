# CLAIM AUDIT: design/figma-renders/new_playlist.png
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R3) |
| Claims extracted | 1 (asset integrity as frame evidence) |
| REAL 1 / others 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
`file` -> "PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced", 52,685 bytes.
Genuine design-source render of the create-playlist dialog state; not code, not a build
input. Cited as the create-playlist frame in CONTRACT.md:55 (playlists row) and
design/README.md:23; the SVG twin is the shipped code's geometry authority —
ui/src/screens/playlist/playlist.css:12 (".lumen-plnew — new playlist.svg"), :274
"(713×589 rx14)", :340-341 frame-measured input geometry, :425 well. Also cited by
w26 findings/ratings (g18.md, e09.md, h16.md) and PlaylistScreen.tsx/Surface.tsx.
No fabrication indicator: all citations are reference/measurement uses; nothing claims
the asset executes. No matrix row's evidence field names it.
## 2. Reachability
N/A (static design asset; the create-playlist dialog it depicts is shipped-UI reachable).
Commands: file, stat, rg citation sweep (9 citing files; playlist.css frame lines pasted).
