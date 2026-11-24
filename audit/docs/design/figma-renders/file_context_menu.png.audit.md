# DATA AUDIT: design/figma-renders/file_context_menu.png
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | PNG, 900 x 1983, 8-bit/color RGBA, non-interlaced (file(1) + IHDR) |
| File verdict | REAL (Figma render) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file(1); python3 IHDR; rg consumers |

## 1. What it depicts
The playlist media-row context-menu design sheet (tall multi-state sheet: the 12-row menu
plus its submenu states such as "Add to Playlist ›"). SVG ground-truth twin:
`design/figma/file context menu.svg` (space-named). Render width 900px = the standard
~2.26x family export scale.

## 2. Consumer / fidelity
- Implementer: ui/src/screens/playlist/options/fileRowMenu.ts — landed by w26.gfix-E:89
  ("g20-P2-4 · file_context_menu.png had no implementer — FIXED"), reproducing the 12 frame
  rows OCR-verbatim (Play · Mark as Watched · Favorite · Stream · Convert · Add to Queue ·
  Add to Playlist · Show in File Explorer · Go to Source · Add Subtitle Tracks · Add Audio
  Tracks · Properties & Metadata).
- Rater: w26.h18.md:99/:241 scores it 8.1 ALMOST and independently verifies the 12 rows in
  pinned order plus right-click AND hover-••• entry points.
- Cross-lane consumers: w26.e12.md, l04.md, g20.md, i16.md:320, gfix-E.md, h16.md.
- Fidelity: consistent 8-bit RGBA render of the sheet; no matrix-row evidence cites it as
  test proof (design-only consumer chain).

## 3. Verdict
REAL asset with a live code consumer; the gfix-E "FIXED" and h18 row-order verifications
are session-scoped (REAL-UNPROVEN on re-execution) but internally corroborated by three
independent w26 docs.
