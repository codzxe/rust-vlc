# DATA AUDIT: design/figma/origin.svg
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A07) |
| Date | 2026-11-24 |
| Type / dims | `file` -> SVG, ASCII, long lines (14,716 chars); svg tag `width="240" height="525"` |
| Verdict | REAL (Figma export; the Add-subtitles source-dropdown frame) |

Depicts: the subtitle-origin dropdown frame — g15.md:17 identifies it precisely: "the
subtitle add-flow 'From Computer/OpenSubtitles/Link'... dropdowns — NOT fader
origin/target markers". 240x525 component sheet (sibling target.svg 240x520). No <text>
nodes (outlined glyphs — no string census possible).

Cited by:
- docs/waves/w26/CONTRACT.md:58 — Control parts row (`origin.png`, `target.png`)
- docs/waves/w26/findings/w26.g15.md:5,17 — the identification ruling (subtitle flow,
  no EQ action required)
- docs/waves/w26/findings/w26.l04.md:23 — live verification: origin pill
  "From Computer ✓/From OpenSubtitles (gated when destination≠file — k15 ruling)";
  triggers 200x34 r6 #151515 frame-exact (computed)
- docs/waves/w26/findings/w26.e12.md:17 — compact grammar reference
- docs/08-ui-figma-fidelity.md:142 — Settings/library atoms row lists `origin.svg`

Consumer (ui/ component): `ui/src/screens/playlist/subtitles/AddSubtitlesDialog.tsx`
(:58 "Where the subtitles come from ('From Computer' / 'From OpenSubtitles')") with the
origin rows keyed through messages.ts:2245 `playlist.subtitles.source.computer`; the
dialog's three "Folder" frames carry the source/destination pairing (AddSubtitlesDialog.
tsx:7-10 header cites Folder.svg/Folder-3.svg).

Findings: S4 — the ui code carries no direct "frame: origin.svg" citation (unlike
sibling frames); its grammar arrives via the Dropdown primitive + l04's measured
200x34 pill. Asset itself clean; render twin (origin.png) exists in figma-renders.
