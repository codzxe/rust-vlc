# CLAIM AUDIT: design/figma/Player - Music-1.svg
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 3 |
| REAL / REAL-UNPROVEN / FALSE / UNVERIFIED | 2 / 1 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Valid Figma export, 1920x1080 music-player screen (540 elements, XML parses
clean, 93 paths). It is the `-1` variant of the `player.music` pair cited by
docs/08-ui-figma-fidelity.md:156. The album-art rect is real but measures
738x737 — "737²" in the citing doc is a rounding of the height only.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid SVG design asset, music player screen | :1 | REAL | `python3 ElementTree.parse` -> OK; root 1920x1080, 540 elements; `file` -> SVG ASCII, longest line 18070 chars. |
| 2 | docs/08:156 `player.music` — "`Player - Music.svg`, `-1`" — this file is the `-1` variant | docs/08-ui-figma-fidelity.md:156 | REAL | File exists as `Player - Music-1.svg` (ls verified); also cited by docs/waves/w26/logs/w26.i20.md:57 (`Player - Music-1.svg` docked column) and w26.gfix-F.md:7. |
| 3 | docs/08:156 "album art 737², waveform seek" | docs/08-ui-figma-fidelity.md:156 | REAL-UNPROVEN | Album-art rect exists: `rg '<rect[^>]*737'` -> `<rect x="591" y="113.333" width="738" height="737" rx="20" fill="url(#pattern1_4778_25123)" ...>` — 738x737, not 737²; "waveform" appears in no group id (`rg 'waveform'` -> 0 hits) — the waveform would have to be encoded in seek-bar geometry, unverifiable without rendering. |

## 3. Contradictions / Fabrication
None. Pure design asset, no prose. Doc 08's "737²" is an imprecise
restatement of a 738x737 rect (height-only rounding).
