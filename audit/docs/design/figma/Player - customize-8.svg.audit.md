# DATA AUDIT: design/figma/Player - customize-8.svg
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 658 lines, ASCII SVG; 1920x1080 canvas; 347 shape/group elements; ~200 color fills |
| File verdict | REAL (Figma export) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (S4 bloat class shared by all design/figma assets) |
| Commands run | file(1); rg censuses; consumer rg |

## 1. What it depicts
Frame "Player - customize" variant 8 of 9 (customize + customize-1..8): the player screen
in state 8 — customization panel CLOSED, queue dock OPEN, media playing. Native canvas
1920x1080 with a pattern-fill media backdrop (screenshot-as-texture), real UI part ids
(Controls, Current Time, Duration, "15 items", tabular timecodes "00:25:40"/"00:02:10",
play glyph). file(1): "SVG Scalable Vector Graphics image, ASCII text, with very long
lines (13785)".

## 2. Consumer / fidelity
- Consumers: docs/waves/w26/findings/w26.l02.md:24 (state-8 MATCH row, its PNG twin
  Player_-_customize-8.png), w26.g09.md:26 (state-8 PARTIAL row: dock 216px vs frame 320px
  scaled — a cross-lane discrepancy recorded against THIS svg), w26.i05.md:182-186
  (state-8 dock-width adjudication "customize-8 says 320").
- Fidelity: ground-truth design asset for the customize state machine (the w26 analysis
  lane treats SVG XML as measurement source); internally consistent vector export, no
  rasterization except the deliberate media pattern fill.
- S4: 658-line machine export with inline pattern image data (bloat class of all
  design/figma exports) — cosmetic only, no claim depends on it.

## 3. Verdict
REAL asset, correctly consumed as the state-8 oracle by three w26 verification docs; the
g09 dock-width cross-lane finding (216 vs 320) is recorded downstream, not hidden.
