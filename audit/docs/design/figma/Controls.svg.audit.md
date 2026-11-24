# CLAIM AUDIT: design/figma/Controls.svg
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 4 (asset integrity + 3 fidelity claims citing this sheet) |
| REAL 4 / REAL-UNPROVEN 0 / FALSE 1 (w26.a04 "no gradients" file-scope reading) / others 0 | |

## 1. Blunt verdict
Genuine Figma export of the player control-bar sheet: `file` -> "SVG Scalable Vector
Graphics image, ASCII text"; 3744 lines / 1,681,020 bytes; canvas 1960x11139; 15 variant
states (`Property 1=default|audio|video|subtitles|playback|queue|bookmarks|chapters|music|
waveform|sub timeline|audio settings|video settings|subtitle settings|Variant10` — rg
census); 874 `<path>` elements; ZERO base64 rasters (pure vector). 1.68 MB for one sheet is
repo bloat (S4 hygiene), not a truth defect.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Asset cited as component-level export "Controls.svg" | design/README.md:24 | REAL | `rg -n 'Controls\.svg' docs/ design/` -> design/README.md:24 + 8 w26 logs (a03:16, a04:98, a07:25-30, gfix-C:7,26, i03:27,97, g04:30,65,86, l01:72). File exists, parses (head/tail read). |
| 2 | "play pill **54x28 rx6 #D37918** with dark glyph" | docs/waves/w26/logs/w26.a03.md:16 | REAL | `sed -n '17p'` -> `<rect x="78" y="33" width="54" height="28" rx="6" fill="#D37918"/>`; glyph fill #161616 (dark). Byte-verified. |
| 3 | "skip-to-previous" Vector 16/18 geometry | docs/waves/w26/findings/w26.g04.md:30 | REAL | `sed -n '13,15p'` -> `M53 52V42H51V52H53Z` (2x10 bar) + left-pointing triangle `M60 52V42L54 47L60 52Z`. Matches g04's quoted path data exactly. |
| 4 | "(`Equalizer.svg`, `Controls.svg`) contain **no gradients**" | docs/waves/w26/logs/w26.a04.md:98 | FALSE at file scope / true only for EQ-tool fills | `rg -o 'linearGradient\|radialGradient' Controls.svg \| sort \| uniq -c` -> "30 linearGradient"; used via `fill="url(#paint0_linear_491_1780)"` etc. The 30 gradients live at :3590-3607 (90px-wide right-cluster elements). The narrow reading ("EQ/tool fills are FLAT") is plausible; the file-scope parenthetical is contradicted by the file. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| w26.a04.md:98 "Controls.svg contain no gradients" | Controls.svg itself has 30 linearGradient defs (:3590+) | The file wins; claim is file-scope FALSE, context-scope (EQ fills) UNVERIFIED (gradients sit on right-cluster elements, not audited which). |

## 4. Fabrication / inflation indicators
None against the SVG itself: consumers use it as reference truth for UI gap claims
(w26.g04:65 "None of the frame's three tool buttons or two popovers exist" — a claim about
the UI code, not about this file). No doc claims the UI *matches* this sheet.

## 5. Reachability (asset, not code)
Not code. Consumer chain: design/README.md (index) -> docs/waves/w26/* (frame evidence for
UI fidelity audits). No build step reads this file (`rg 'Controls\.svg'` outside docs/design:
0 — search bounded to docs/ design/ after full-repo scan timed out; UI-code citation
count = UNVERIFIED for repo-wide, verified 0 for docs+design).

Severity findings: F-001 [S4] w26.a04.md:98 "no gradients" contradicted by 30 linearGradients.
