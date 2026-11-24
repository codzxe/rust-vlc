# AUDIT: ui/e2e/visual/baselines/linux/settings-network-1280x720-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A02) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — nightly lane only (2× DPI → pr=false) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 65,938 bytes, sha256 b27652f6205a6911fce58478bd695006ab02f03bb459ccbbf
ea6a53e146493a40, IHDR 1280×720 RGB8, 2765520 decompressed bytes, 501/720 unique scanlines.
Notable (correct) detail: the 2×-DPI case still has CSS-pixel image dims — Playwright's
toHaveScreenshot captures at `scale: "css"`, so IHDR 1280×720 for a 2× case is the expected
shape, not a defect. Consumer chain: manifest.ts:142 caseId ("/settings/network" × 1280×720 ×
2× × light, pr=false) → visual.spec.ts:103 → playwright.visual.config.ts:27; disk/manifest 1:1
(script check). Committed CI consumer: `visual-nightly` (.github/workflows/ui.yml:205).

F-001 [S4] No local gate runs the nightly lane (rg full_gate.sh/rc-smoke.sh: zero visual
matches). Confidence: HIGH (rg).
F-002 [S4] Consuming job mislabeled: ui.yml:206 "136 cases" vs actual 216 (MATRIX-20261113.md:41).
Live diff UNVERIFIED here (no browser budget; 5th-re-pin CI proof deferred — line 153).
