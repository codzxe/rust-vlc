# CLAIM AUDIT: design/figma-renders/Player-6.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R4) |
| Claims extracted | 2 (integrity + fidelity-evidence use) |
| REAL 2 / others 0 | |

## 1. Blunt verdict
Genuine render: `file` -> "PNG image data, 900 x 506, 8-bit/color RGB,
non-interlaced"; python3 struct confirms 900x506, color-type 2 (RGB), 21,952
bytes. SVG source pair `design/figma/Player-6.svg` present. This is the
"everything hidden" player composition frame (rail + sidebars + panels gone),
heavily cited by the f09 composition work.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Render of the /player fully-unchromed state (rail + sidebars hidden) | whole file | REAL | `rg -ln 'Player-6' docs/` -> w26.f09.md:21 "Player-6 (rail + sidebars hidden) — FIXED via a documented composition rule"; g03.md, l01.md, g04.md, e02.md, h02.md also cite it. |
| 2 | Fidelity-evidence usage (composition oracle for the f09 `:has()` rule) | whole file | REAL | w26.f09.md:22 "player.css now composes Player-6 from the two published state seams" + :37 "Player-6 `:has()` rule" — the frame drives a real CSS composition rule in player.css. |

## 3. Contradictions / 4. Fabrication indicators
None. Consumers use the frame as the target state for real CSS work
(player.css `:has()` composition), not as proof of shipped behavior.
