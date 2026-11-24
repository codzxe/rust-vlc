# CLAIM AUDIT: design/figma/Add Subtitles to Playlist/Folder-1.svg
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 3 |
| REAL 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Valid SVG design asset (state variant of the add-subtitles flow), correctly inventoried.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Well-formed SVG | — | REAL | python3 ElementTree parse OK; `file` -> SVG ASCII text |
| 2 | Dimensions 1048×800 | doc 08:165 flow table | REAL | width="1048" height="800", viewBox="0 0 1048 800" |
| 3 | Part of `Add Subtitles to Playlist/Folder{,-1..3}.svg` flow (`state=from computer/from opensub/to this file/to this folder`) | docs/08-ui-figma-fidelity.md:165 | REAL | file exists at that path with `-1` suffix; python3: `folder`/`subtitle` strings present in content; design/README.md Playlists row lists `Add Subtitles to Playlist/Folder*.svg` (4) |

## 3. Contradictions
None.
