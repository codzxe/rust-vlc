# DATA AUDIT: design/figma/new playlist.svg
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A06) |
| Date | 2026-11-24 |
| Type / dims | `file` -> SVG, ASCII, very long lines (46,020 chars); svg tag `width="713" height="589"` |
| Verdict | REAL (Figma export; the create-playlist dialog frame, heavily frame-cited in shipped CSS) |

Depicts: the New Playlist dialog — details form + drag-and-drop file box ("Drag files
into the box or click here…" per doc 08:164), 713x589 rx14 dialog with a 671x184 drop
well. S4 note: the filename carries a space (the family's recurring export naming).

Cited by:
- docs/08-ui-figma-fidelity.md:164 — "`playlist` (new) | `new playlist.svg`,
  `playlist.svg` | details form + drag-and-drop file box"
- design/README.md:23 — Playlists row
- docs/waves/w26/CONTRACT.md:55 — `/playlist` surfaces (render twin
  `new_playlist.png` in figma-renders/)
- SHIPPED CODE (densest frame-citation chain of this batch):
  ui/src/screens/playlist/playlist.css:12 (the create-playlist dialog `.lumen-plnew`),
  :274 ("new playlist.svg (713×589 rx14) at the app's dialog column scale"),
  :340-341 (input 35 × 0.785 scale, rx5), :425 (671×184 well), :431 (drop zone
  #2D2D2D 2px dashed 4-4, stylelint-disabled with frame cite), :482 (Frame 140
  Cancel fill #1B1B1B, "g18 P3-4")

Consumer (ui/ component): `ui/src/screens/playlist/NewPlaylistDialog.tsx` (the
`.lumen-plnew` dialog; also surfaced from AppShell's create row since f08) +
`ui/src/screens/playlist/playlistsStore.ts:22` ("A saved local playlist (new
playlist.svg 'Save' output)").

Findings: S4 — space in filename (family-wide export quirk); no other defect. The
dialog's frame→CSS citation chain (6+ sites) is the strongest in this batch — any
geometry claim about .lumen-plnew resolves to this SVG.
