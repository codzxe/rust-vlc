# DATA AUDIT: design/figma-renders/Duration.png
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A06; R3's abandoned tail #50) |
| Date | 2026-11-24 |
| Type / dims | `file` -> PNG image data, 900 x 1295, 8-bit/color RGBA, non-interlaced |
| Verdict | REAL (real render; the duration-timecode control-part reference) |

Depicts: the Duration control-part grammar — source `design/figma/Duration.svg` (tall
component sheet): default face "00:25:40" total duration + the click face "-00:23:30"
(remaining-time form). Render 900x1295 RGBA (component-sheet export).

Cited by:
- docs/waves/w26/CONTRACT.md:58 — Control parts row: `Duration.png` among
  Controls/Button/Toggle/Values/Current_Time/volume/play
- docs/waves/w26/findings/w26.g04.md:3,13 — "default 00:25:40; click face '-00:23:30'
  (remaining) | static span; click face unported (f13-logged) | GAPS (P1-4)"
- docs/waves/w26/ratings/w26.h04.md:6,116,412 — "4 | Duration.png | 9.4 | YES"
- docs/waves/w26/findings/w26.l04.md:32 — control-parts HOLD row (timecodes tabular)

Consumer (ui/ component): `ui/src/screens/player/controls/ControlBar.tsx` — :4 cites
"Duration.svg" as its frame; :23-25 documents the duration readout as a
"remaining-toggle button" (Duration.svg's '-00:23:30' click face), :126 pins the frame
timecode format; the tabular-numerals rule rides `.timecode` in lumen.css.

Findings: S4 only. NOTE: g04's "click face unported" (P1-4, 2026-11-12) is stale —
ControlBar.tsx:23-25 documents the remaining-toggle click face as implemented (f13/f-wave
follow-up). Cite the component comment, not g04, for current state.
