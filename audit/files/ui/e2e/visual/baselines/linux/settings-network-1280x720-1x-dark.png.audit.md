# AUDIT: ui/e2e/visual/baselines/linux/settings-network-1280x720-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A02) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — PR-lite lane member (1280×720 @1× dark, pr=true) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 65,300 bytes, sha256 b9f0ece3c2a733023ecd1321f5bd0d7855b4b376a76d0bd5
04645814f927ff59, IHDR 1280×720 RGB8, 2765520 decompressed bytes, 499/720 unique scanlines.
Dimensions match the filename. Consumer chain: manifest.ts:142 caseId (ROUTES
"/settings/network" × 1280×720 × 1× × dark, pr=true) → visual.spec.ts:103 toHaveScreenshot →
playwright.visual.config.ts:27 snapshotPathTemplate; disk/manifest match 1:1 (script check).
Unlike the nightly-only files, this case IS in the PR-lite lane, which runs on EVERY push/PR via
the committed CI job `visual` (.github/workflows/ui.yml:148, `node scripts/visual-run.mjs
--mode=pr`) — the lane is committed and scheduled; the matrix records PR verify 50/50 for the
final pin (MATRIX-20261113.md:148, "this sandbox" record).

F-001 [S4] Local gates never run the lane (rg scripts/full_gate.sh + scripts/rc-smoke.sh: zero
visual matches) — GitHub-runner-only enforcement. Confidence: HIGH (rg).
F-002 [S4] Consuming job mislabeled: ui.yml:149 `visual (PR-lite lane — 30 cases vs pinned Linux
baselines)` and comment "15 routes × 2 = 30 cases" — the manifest's PR lane is 25 routes × 2 = 50
(MATRIX-20261113.md:51). Stale pre-k19 counts on a committed workflow.
Live diff UNVERIFIED here (no browser budget; 5th-re-pin CI proof explicitly deferred —
MATRIX-20261113.md:153).
