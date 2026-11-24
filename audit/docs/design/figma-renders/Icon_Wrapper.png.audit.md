# DATA AUDIT: design/figma-renders/Icon_Wrapper.png
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A02) |
| Date | 2026-11-24 |
| Type / dims | `file` -> PNG image data, 900 x 1525, 8-bit/color RGBA, non-interlaced |
| Verdict | REAL (real render; heavily-cited icon-grammar reference) |

Depicts: the icon-wrapper component grammar render — source `design/figma/Icon Wrapper.svg`
(72x122; hover chip 30x32 rx4, two variants Default/hover). Render is 900x1525 (12.5x
nominal export scale of the small SVG — Figma multiplies small components up for the
renders sheet).

Cited by:
- docs/waves/w26/CONTRACT.md:58 — Control parts row: `Icon.png`, `Icon_Wrapper.png`
- docs/waves/w26/logs/w26.a07.md:101 — reference reads (the icon-grammar mining lane)
- docs/waves/w26/findings/w26.g07.md:9 — "Icon grammar cross-check vs Icon.png /
  Icon_Wrapper.png on both surfaces" (P3-1 rx4 ruling source)
- docs/waves/w26/findings/w26.l04.md:4,32 — control-parts family HOLD verification

Consumer (ui/ component): the IconWrapper grammar — `ui/src/components/icon/Icon.tsx`
(test pin: Icon.test.tsx:400 "IconWrapper (w26.a07, design/figma/Icon Wrapper.svg)") and
the appshell rail hits (appshell.css:225/250 "border-radius: 4px; frame: Icon Wrapper.svg
rx=4"); controls.css:23 carries the same grammar for the player bar.

Cross-reference: the SVG itself was audited twice (audit/design/figma/Icon Wrapper.svg.
audit.md [R7] + audit/docs/design/figma/Icon Wrapper.svg.audit.md) — R7's S3 stands:
AppShell.tsx:667's "44px hit frame (Icon Wrapper.svg)" cites geometry the SVG does not
contain (svg is 72x122, chip 30x32) — the render cannot supply it either.

Findings: S4 — render is a 12.5x upscaled export, scale undocumented anywhere; no
provenance embedded. Asset itself clean.
