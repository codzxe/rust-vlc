# DATA AUDIT: design/figma-renders/Player.png
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A04; R3's abandoned tail #47) |
| Date | 2026-11-24 |
| Type / dims | `file` -> PNG image data, 900 x 506, 8-bit/color RGB, non-interlaced |
| Verdict | REAL (real render of the player core frame; family-referenced throughout) |

Depicts: the base "Player" core screen frame — source `design/figma/Player.svg`
(1920x1080): the populated player composition (video still + 00:02:10 timecode, sidebar
with the playlists browser, strip bar with transport) that h03/i02 measured against (dock
283px = Player.svg sidebar 377px @1920 x0.75 — player.css:101 cites it). Render
900x506 (2.13x downscale).

Cited by:
- docs/waves/w26/CONTRACT.md:46 — "Player core | /player | `Player.png`, `Player-1..6.png`"
- docs/waves/w26/logs/w26.a02.md:91 — vision compare target (GAPS from then-stale
  Solstice token values, not the asset)
- docs/waves/w26/logs/w26.i02.md (:82-83 comparison loop) — empty-1440 and populated-1440
  comparisons; the frame "depicts a POPULATED player (video still, 00:02:10)"
- docs/08-ui-figma-fidelity.md §1.6 player rows (family: Player.svg/-1..6)

Consumer (ui/ component): `ui/src/screens/player/PlayerScreen.tsx` (the /player core
screen — dock, MediaSurface, strip bar; states/PlaylistBrowse.tsx for the browse face);
the dock geometry consumer is `ui/src/screens/player/player.css:101` (377px @1920 -> 283px
frame citation).

Findings: S4 — render scale undocumented; no embedded provenance. Asset clean; its
comparison history (a02 GAPS -> i02 GAPS-with-measured-equality on dock width) is the
canonical cross-check chain for /player fidelity claims.
