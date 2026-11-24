# AUDIT: ui/e2e/visual/baselines/linux/keyboard-shortcuts-1920x1080-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A05) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — nightly lane only (pr=false: PR lane is 1280×720@1×) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present, 216=216); rg over gates/workflows |

## 1. Verdict (data audit)
Real rendered screenshot: 69,672 bytes, sha256
8f44a412779d858b19642d82b7bb7cd735c5a1dc3488991b5b562082081ce28d, IHDR 1920×1080 RGB8,
6221880 decompressed bytes, 463/1080 unique scanlines — genuine content. Dimensions match the
filename. Consumer: manifest.ts:142 caseId over ROUTES "/keyboard-shortcuts" × 1920×1080 × 1× ×
light, diffed at visual.spec.ts:103, path via playwright.visual.config.ts:27; disk/manifest sets
match 1:1 (script check). The committed CI lane that executes it is `visual-nightly`
(.github/workflows/ui.yml:205, cron 03:41 UTC, `scripts/visual-run.mjs --mode=nightly`).

F-001 [S4] No local gate runs this lane (rg scripts/full_gate.sh + scripts/rc-smoke.sh: zero
visual matches) — enforcement is GitHub-runner-only.
F-002 [S4] Consuming job mislabeled: ui.yml:206 "136 cases" vs the actual 216-case nightly grid
(MATRIX-20261113.md:41). Stale count from the pre-k19 manifest.
Confidence: HIGH — Verification: read + rg + script.
Execution status: 5th-re-pin verify runs are local-sandbox records (MATRIX-20261113.md:147-151);
CI proof explicitly deferred ("The next CI visual run is the proof", line 153) → live diff
UNVERIFIED in this recovery environment.
