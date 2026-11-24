# AUDIT: ui/e2e/visual/baselines/linux/open-network-stream-1920x1080-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A05) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — nightly lane only (1920×1080 + 2× DPI → pr=false) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216, 0 orphans); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 50,365 bytes, sha256 934cd75e2f7895d8f2cb2a9e15538034afffcd432480dd5
20a32d52b2dc11a37, IHDR 1920×1080 RGB8 (CSS-scale capture — correct for a 2× DPI case),
6221880 decompressed bytes, 385/1080 unique scanlines (sparse form screen, consistent with
the route). Dimensions match filename. Consumer chain: manifest.ts:142 caseId
("/open-network-stream" × 1920×1080 × 2× × dark, pr=false) → visual.spec.ts:103 →
playwright.visual.config.ts:27; disk/manifest 1:1 (script check). Committed CI consumer:
`visual-nightly` job (.github/workflows/ui.yml:205, cron 03:41 UTC).

F-001 [S4] No local gate runs the nightly lane (rg scripts/full_gate.sh + scripts/rc-smoke.sh:
zero visual matches). Confidence: HIGH (rg).
F-002 [S4] Consuming job mislabeled: ui.yml:206 "136 cases" vs the actual 216-case nightly
grid (MATRIX-20261113.md:41) — stale pre-k19 count on the committed workflow.
Live diff UNVERIFIED here (no browser budget; the 5th re-pin's CI proof explicitly deferred —
MATRIX-20261113.md:153).
