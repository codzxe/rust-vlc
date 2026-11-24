# CLAIM AUDIT: design/figma/Player - Music.svg
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 3 |
| REAL / REAL-UNPROVEN / FALSE / UNVERIFIED | 2 / 1 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Valid Figma export, 1920x1080 music-player base screen (405 elements, XML
parses clean, 51 paths), cited as the primary `player.music` design source
by docs/08:156 and mined by w26 logs (gfix-F parsed its Controls group
"directly"). Album art rect is 738x737.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid SVG design asset | :1 | REAL | ElementTree parse OK; root 1920x1080, 405 elements; `file` -> SVG ASCII. |
| 2 | docs/08:156 — `player.music` maps to "`Player - Music.svg`, `-1`" | docs/08-ui-figma-fidelity.md:156 | REAL | Both files exist in design/figma/ (ls: `Player - Music.svg`, `Player - Music-1.svg`). |
| 3 | docs/08:156 "album art 737², waveform seek" | docs/08-ui-figma-fidelity.md:156 | REAL-UNPROVEN | `rg '<rect[^>]*737'` -> `<rect x="591" y="113.333" width="738" height="737" rx="20" fill="url(#pattern1_4767_23726)" ...>` — album art real at 738x737 (doc's "737²" rounds the height only); no `waveform` id/string in file (`rg 'waveform'` -> 0) — waveform claim unverifiable without rendering. |

## 3. Contradictions / Fabrication
None. w26.gfix-F.md:46 ("Parsed `Player - Music.svg` Controls group
directly: it DOES carry a band") is consistent with the `id="Controls"`
group present in this file (id listing verified).
