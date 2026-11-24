# AUDIT: ui/e2e/visual/baselines/linux/settings-recording-1280x720-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A02) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — PR-lite lane member (1280×720 @1× light, pr=true) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 75,963 bytes, sha256 851bca4f728c0120b48896c3093d4120ebde05b1111aa9f
1b246b7a6d4bff512, IHDR 1280×720 RGB8, 2765520 decompressed bytes, 524/720 unique scanlines —
matches MATRIX-20261113.md:26's own provenance note ("recording 10, frame-exact" rows came in
the wave-i settings C-area work). Dimensions match filename. Consumer chain: manifest.ts:142
caseId ("/settings/recording" × 1280×720 × 1× × light, pr=true) → visual.spec.ts:103 →
playwright.visual.config.ts:27; disk/manifest 1:1 (script check). PR-lite lane runs on every
push/PR via committed CI job `visual` (.github/workflows/ui.yml:148).

F-001 [S4] Local gates never run the lane (rg scripts/full_gate.sh + scripts/rc-smoke.sh: zero
visual matches). Confidence: HIGH (rg).
F-002 [S4] Consuming job mislabeled: ui.yml:149 "30 cases" / comment "15 routes × 2" vs the
actual 50-case PR lane (MATRIX-20261113.md:51) — stale counts on the committed workflow.
Live diff UNVERIFIED here (no browser budget; 5th-re-pin CI proof deferred —
MATRIX-20261113.md:153).
