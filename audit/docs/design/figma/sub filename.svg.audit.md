# DATA AUDIT: design/figma/sub filename.svg
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A08) |
| Type / viewBox | SVG ASCII, 406×422, `g id="sub filename"` (file(1) verified) |
| Render pair | design/figma-renders/sub_filename.png (900×935) — both present |
| Matrix evidence citations | NONE directly (no feature-matrix row cites the asset) |

## Blunt verdict
Real design frame for the subtitle filename-matching surface, and it is the
IMPLEMENTED spec: ui/src/screens/playlist/subtitles/matching.ts:25 "The five
strategies of sub filename.svg (order = the frame's row order)" — code
documents its strategy list ORDER as pinned by this SVG. Cited by
e09.md:181 ("`sub filename.svg` rect x22 y22 362×48 rx6"), g19, gfix-E,
doc 08, ui-inventory. No fabricated-evidence exposure.

## Checks run
- `file` -> SVG Scalable Vector Graphics image, ASCII text (exit 0)
- `head -2` -> width="406" height="422" viewBox="0 0 406 422", `<g id="sub filename">`
- `rg 'sub filename' --glob '!audit'` -> 8 files (matching.ts, MatchedPairsList,
  harness.css, e09/g19/gfix-E, ui-inventory, doc 08); render pair on disk.

Findings: none. Short data audit per template 12.2.
