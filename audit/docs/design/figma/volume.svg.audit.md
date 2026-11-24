# CLAIM AUDIT: design/figma/volume.svg (binary/text asset)
| Field | Value |
|---|---|
| Auditor | W19-A08 — RECOVERED by W19-R2 |
| Type | SVG, ASCII text (`file`: "SVG Scalable Vector Graphics image, ASCII text") |
| Size / Lines | 17,569 B / 262 lines (wc -c / wc -l) |
| Dimensions | 130×646 (`width="130" height="646" viewBox="0 0 130 646"`, :1) |
| sha256 | 33dea04a4c8b8f82c420710b1e470989df1a3a7b1b62122e1d66e62e70465a86 |
| Verdict | REAL (design source asset) |

## 1. Blunt verdict
Genuine Figma-export vector source: pure ASCII SVG, 12 gradients, frame rect `0.5,0.5 129x645`
rx 4.5 stroke `#8A38F5` (frame guide), variant group `id="volume"` with `Property 1=0..N`
states. No script/href payload; no foreignObject; static paint only.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 130×646 vertical volume-slider frame | :1 | REAL | `sed -n 1p` byte-verified; `file(1)` agrees (SVG text). |
| 2 | 4-stop level gradient `#95D318→#B2A918→#D37918→#FF3737` cited by w26.a04.md:129 | whole | REAL | `rg -o 'stop-color'` -> exactly those 4 stop-colors, unique set. |
| 3 | Design->code trace: volume wedge used by ControlBar | whole | REAL | consumers: ui/src/screens/player/controls/ControlBar.tsx:4,26; controls.css:339 ("89px wedge track, left cap ≈11px → right cap ≈28px"); w26.gfix-C.md:8. |
| 4 | Matrix evidence use | whole | UNVERIFIED | feature-matrix.toml Volume rows (:1661,:3238) cite vlc_name only — no direct .svg citation (rg); renders lane N/A for source SVG. |

## 3. Contradictions
None. No doc claims volume.svg beyond geometry/color, both verified byte-exact.
