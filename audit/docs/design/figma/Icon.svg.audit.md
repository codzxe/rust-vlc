# CLAIM AUDIT: design/figma/Icon.svg (chunks #part1-#part2, L1-1430)
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 3 / 1 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A real, well-formed Figma-exported icon sheet: 548x812 canvas with Figma's
dashed purple selection frame (`stroke="#8A38F5" stroke-dasharray="10 5"`),
90 distinct icon name groups (settings, accessibility, add, album, arrow,
artist, audio, backward, bookmark, ...), 206 glyph variants (158 two-tone=no,
48 two-tone=yes), 941 `id=` elements. XML parses clean. It is a design asset,
not code; it is actively cited as the glyph-mining source by docs/08-ui-figma-
fidelity.md and w26 logs.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | File is a valid SVG design asset | :1 | REAL | `file` -> SVG ASCII; `python3 xml.dom.minidom.parse` -> XML-VALID. `sed -n '1p'` -> `<svg width="548" height="812" viewBox="0 0 548 812" fill="none" xmlns="http://www.w3.org/2000/svg">` |
| 2 | It is an icon-variant sheet with metadata-encoded group ids | :4, :9 | REAL | `grep -o 'name=[a-z-]*' \| sort -u \| wc -l` -> 90 distinct names; group ids carry `name=settings, direction=default, state=default, ... EQ=default, two-tone=no` metadata (verbatim from :4). |
| 3 | docs/08-ui-figma-fidelity.md:141 — "`Icon.svg` / `Icon Wrapper.svg` \| Icon + 48px hit area \| default" | docs/08-ui-figma-fidelity.md:141 | REAL-UNPROVEN | Icon.svg exists and is the icon glyph source; the 48px hit-area claim maps to the separate Icon Wrapper.svg (not in this file); rendering the two-tone/hit-area pairing needs a renderer (not run). |
| 4 | w26.a07.md:15 — glyphs "verbatim design geometry mined from `design/figma/Icon.svg`" | docs/waves/w26/logs/w26.a07.md:15 | REAL-UNPROVEN | Source geometry exists here (34 `<path d=...>` + circles); byte-for-byte glyph reuse by UI code not re-verified in this audit (would need pixel comparison — UNVERIFIED at that level). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| none (inert asset) | w26.a01.md:43 claims icon stroke color #8F8F8F measured from Icon.svg | Icon.svg paths do use `stroke="#8F8F8F"` (`grep` on :7 `stroke-width="1.5" stroke="#8F8F8F"`) — consistent. |

## 4. Fabrication / inflation indicators
None. No prose claims inside the file; it is pure geometry + Figma ids.

## 5. Matrix rows
None — design asset, not a matrix chunk. No matrix row cites Icon.svg
directly (rg over docs/ found only fidelity doc + w26 logs).
