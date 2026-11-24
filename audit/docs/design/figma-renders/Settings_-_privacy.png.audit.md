# DATA AUDIT: design/figma-renders/Settings_-_privacy.png
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A07) |
| Date | 2026-11-24 |
| Type / dims | `file` -> PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced |
| Verdict | REAL (real render of a heavily-cited frame) |

Depicts: the Settings "Privacy" area frame — source `design/figma/Settings - privacy.svg`
(1048x865), containing the History + Network sections the privacy screen reproduces
(9 info ⓘ circles r7.5 — census verified by me: `rg -c 'r="7\.5"'` -> 9, matching
w26.j13's "privacy 9" census and w26.i14's circle extraction). Render is 900x743
(~0.86x scale of the 1048x865 frame).

Cited by:
- docs/waves/w26/findings/w26.g11.md:3 — lane scope "/settings/privacy... 3 frames:
  `Settings_-_privacy.png`..."
- docs/waves/w26/ratings/w26.h08.md:73 — "Privacy (/settings/privacy vs
  `Settings_-_privacy`)" 8-criterion table (layout 9.5 ... a11y 8.5)
- docs/08-ui-figma-fidelity.md:160 — settings areas map lists `privacy` (13-area set)

Consumer (ui/ component): `ui/src/screens/settings/areas/privacy.tsx` ("Settings area:
Privacy — 'Privacy Settings' (Settings - privacy.svg...)") and the split
`areas/privacy-network.tsx` (History + Network sections); test pinning in
`privacy-network.test.tsx:3,101` ("the frame's History + Network sections
(Settings - privacy.svg §C3)").

Findings: S4 only — no embedded provenance; scale factor undocumented. Asset itself clean;
the frame's ⓘ census is independently byte-reproducible (9).
