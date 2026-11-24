# AUDIT: ui/e2e/visual/baselines/linux/open-network-stream-1280x720-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A06) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — PR-lite lane member (1280×720 @1× dark, pr=true) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216, 0 orphans); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 41,224 bytes, sha256 dbfc427c5b4cb6b410e843f76b3e060f12889cac7727
df753095378a559fee57, IHDR 1280×720 RGB8, 2765520 decompressed bytes, 377/720 unique
scanlines — the sparsest PR-lane settings/stream screen (one input + button face), consistent
with a genuinely rendered form, not a fill. Dimensions match filename. Consumer chain:
manifest.ts:142 caseId ("/open-network-stream" × 1280×720 × 1× × dark, pr=true) →
visual.spec.ts:103 toHaveScreenshot → playwright.visual.config.ts:27; disk/manifest 1:1
(script). PR-lite lane runs on every push/PR via committed CI job `visual`
(.github/workflows/ui.yml:148, `scripts/visual-run.mjs --mode=pr`).

F-001 [S4] No local gate runs the lane (rg scripts/full_gate.sh + scripts/rc-smoke.sh: zero
visual matches) — GitHub-runner-only enforcement. Confidence: HIGH (rg).
F-002 [S4] Consuming job mislabeled: ui.yml:149 "30 cases" / comment "15 routes × 2" vs the
actual 50-case PR lane (MATRIX-20261113.md:51). Stale counts on the committed workflow.
Live diff UNVERIFIED here (no browser budget; 5th-re-pin CI proof deferred —
MATRIX-20261113.md:153).
