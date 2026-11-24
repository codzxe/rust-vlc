# AUDIT: ui/e2e/visual/baselines/linux/settings-network-1920x1080-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A03) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — nightly lane only (1920×1080 → pr=false) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 80,673 bytes, sha256 fd853d158dcf3bbce17fbb9cd6292a5be0285251fbfaa11c
2ff5bede01bb5a94, IHDR 1920×1080 RGB8, 6221880 decompressed bytes, 603/1080 unique scanlines.
Dimensions match the filename. Consumer chain: manifest.ts:142 caseId ("/settings/network" ×
1920×1080 × 1× × light, pr=false) → visual.spec.ts:103 → playwright.visual.config.ts:27;
disk/manifest 1:1 (script check). Committed CI consumer: `visual-nightly` job
(.github/workflows/ui.yml:205, cron 03:41 UTC, --mode=nightly).

F-001 [S4] No local gate runs the nightly lane (rg scripts/full_gate.sh + scripts/rc-smoke.sh:
zero visual matches). Confidence: HIGH (rg).
F-002 [S4] Consuming job mislabeled: ui.yml:206 "136 cases" vs actual 216-case nightly grid
(MATRIX-20261113.md:41) — stale pre-k19 count on the committed workflow.
Execution: 5th-re-pin nightly verify 216/216 is a local-sandbox record
(MATRIX-20261113.md:149-150); CI proof deferred by the doc's own line 153 → live diff UNVERIFIED
in this recovery environment (no browser budget).
