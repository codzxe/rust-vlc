# AUDIT: ui/e2e/visual/baselines/linux/convert-1280x720-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A02) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — nightly lane only (2× DPI → pr=false) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 60,195 bytes, sha256 840d17646786d4ad86e854f67c462a77247e50d73ae8ac084
0fcd62c07b6b197, IHDR 1280×720 RGB8 (CSS-scale capture, correct for 2× DPI), 2765520
decompressed bytes, 359/720 unique scanlines. The route itself is one of the nine w26.k20
additions (manifest.ts header "convert + 8 of the 13 settings areas" — docs/waves/w26/logs/
w26.k20.md:184 records "+72 PNGs ... (convert + 8 ..."). Consumer chain: manifest.ts:142 caseId
("/convert" × 1280×720 × 2× × light, pr=false) → visual.spec.ts:103 →
playwright.visual.config.ts:27; disk/manifest 1:1 (script check). Committed CI consumer:
`visual-nightly` (.github/workflows/ui.yml:205).

F-001 [S4] No local gate runs the nightly lane (rg full_gate.sh/rc-smoke.sh: zero visual
matches). Confidence: HIGH (rg).
F-002 [S4] Consuming job mislabeled: ui.yml:206 "136 cases" vs actual 216 (MATRIX-20261113.md:41).
Live diff UNVERIFIED here (no browser budget; 5th-re-pin CI proof deferred — line 153).
