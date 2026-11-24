# DATA AUDIT: design/figma-renders/Player_-_customize-5.png
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A07; R3's abandoned tail #48) |
| Date | 2026-11-24 |
| Type / dims | `file` -> PNG image data, 900 x 506, 8-bit/color RGB, non-interlaced |
| Verdict | REAL (real render of the 5th customize state frame) |

Depicts: variant 5 of 9 of the "Player - customize" frame set (source `design/figma/
Player - customize-5.svg`, 1920x1080): the customize panel open with an edited arrangement
— Save button enabled/full-opacity (vs the 50%-dimmed clean state in the base frame).
Render 900x506 (2.13x downscale of the 1920x1080 frame). SVG prior audit exists:
audit/docs/design/figma/Player - customize-5.svg.audit.md.

Cited by:
- docs/waves/w26/findings/w26.g09.md:4 — "frames `Player_-_customize-5/6/7/8.png`"
- docs/waves/w26/findings/w26.g09.md:23 — state table row 5: "Panel open, arrangement
  edited (Save enabled/full-opacity) | Player_-_customize-5.png | state5-after-*.png |
  UNREACHABLE — all 3 documented triggers dead (g09-P0-1); content diverges (g09-P1-2,
  g09-P2-4)"
- docs/08-ui-figma-fidelity.md:158 + design/README.md:18 cite the family "-1..8"
  collectively.

Consumer (ui/ component): the customize modal — PlayerScreen.tsx hosts the per-open
centered modal (w26.gfixA listener at :617-625) mounting `customize/CustomizePanels.tsx`;
the Save-enabled state maps to CustomizePanels' dirty-tracking ("Save enables only when
dirty", pinned by CustomizePanels.test.tsx per g07's verified-good list).

Findings: S4 — individually cited only by w26.g09 (one verifier); render scale
undocumented. NOTE: g09's "UNREACHABLE" verdict for this state was 2026-11-12 — the
gfixA listener landed after, so the state-5 surface is reachable today via the modal;
cite this audit, not g09's stale reachability row.
