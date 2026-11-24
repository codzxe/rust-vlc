# DATA AUDIT: design/figma-renders/Player_-_customize-4.png
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A07) |
| Date | 2026-11-24 |
| Type / dims | `file` -> PNG image data, 900 x 506, 8-bit/color RGB, non-interlaced |
| Verdict | REAL (real render of an existing frame; family-cited, not individually cited) |

Depicts: variant 4 of 9 of the "Player - customize" frame set (source `design/figma/
Player - customize-4.svg`, 1920x1080 canvas): the customize panel OPEN state — "Customize
Player" title + the verbatim instruction "Drag items around and out of the box below..."
(see the prior SVG audit audit/docs/design/figma/Player - customize-4.svg.audit.md,
W19-A08/R1). Render is 900x506 = 2.14x downscale export of the 1920x1080 frame.

Cited by:
- docs/08-ui-figma-fidelity.md:158 — `player.customize` row cites the family
  "`Player - customize.svg`, `-1..8`" collectively (this render is -4 of that set)
- design/README.md:18 — "Player — customize | `Player - customize.svg`, `-1..8` |
  Customize panel states"
- No doc names `Player_-_customize-4.png` individually (no exact-string citation).

Consumer (ui/ component): the customize modal — PlayerScreen.tsx hosts the per-open
centered MODAL (w26.gfixA ruling, `lumen:open-customize` listener at :617-625) mounting
`customize/CustomizePanels.tsx`; the data model is `customize/types.ts` ("rebuilt to the
9-state frame set design/figma/Player - customize*.svg") with `toolbarItems.tsx` /
`toolbarLayout.ts` implementing the drag-box grammar the frame pins.

Findings: S4 — individually uncited render (family-level citation only; same class as the
other -N customize renders); 2.14x scale undocumented. No defect in the asset itself.
