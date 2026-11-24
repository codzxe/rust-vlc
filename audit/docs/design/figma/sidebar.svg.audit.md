# CLAIM AUDIT: design/figma/sidebar.svg
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 1 (asset-integrity check) |
| REAL 1 / others 0 | |

## 1. Blunt verdict
Genuine Figma export, not a stub: `file` -> "SVG Scalable Vector Graphics image, ASCII text, with very long lines"; 2267 lines, 7,882,520 bytes; 482 `<path>` elements, 208 fill-attributed shapes, embedded base64 PNG rasters (`juxbox.png`, 1000x1000) in `<defs>`. Header: `<svg width="4492" height="1028" viewBox="0 0 4492 1028" ...>` with `id="sidebar"` and variant group `Property 1=Default`. It is a real design deliverable for the sidebar surface.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Design asset cited as sidebar reference by design/README.md and docs/08-ui-figma-fidelity.md | whole file | REAL | `rg -l 'sidebar\.svg'` -> design/README.md, docs/08-ui-figma-fidelity.md, 5+ w26 logs. File parses as SVG structure (checked head/tail); raster payload embedded. |

## 3. Contradictions
None. No doc claims this SVG *is* the implemented UI; it is reference art.

## 4. Fabrication / inflation indicators
7.9 MB for one sidebar sheet (mostly base64 raster) is repo bloat (S4 hygiene); not a truth claim. No glyph-mining claim attaches to this file specifically (w3g icon mining cites other exports).
