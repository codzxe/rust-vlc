# AUDIT: ui/e2e/visual/baselines/linux/miniplayer-1920x1080-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 19811 B / a15a3e13049c400ddf61bccc641f56807d2e39837ff7f712778e54e979d0a936 |
| Dimensions | 1920x1080 (bit depth 8, color type 2 RGB8, non-interlaced) |
| File verdict | REAL (pinned screenshot baseline; content is a real rendered screen) |
| Reachable from shipped entry point? | NO — test asset; consumed by ui/e2e/visual/visual.spec.ts via toHaveScreenshot + snapshotPathTemplate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. Verdict (binary-asset audit)
Real PNG baseline (not a placeholder): 266 unique pixel rows among the first 2000 decoded rows (1080 rows total, 6221880 raw bytes) — a solid-color or 1x1 placeholder would show 1-2. Dimensions 1920x1080 (matches the viewport encoded in the filename). Consumed exactly: `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` (ui/e2e/visual/visual.spec.ts:103) with `snapshotPathTemplate: "e2e/visual/baselines/{platform}/{arg}{ext}"` (ui/playwright.visual.config.ts:25) — the case-id `miniplayer-1920x1080-2x-light` maps to this file on Linux. Baseline set completeness verified: 216 files in baselines/linux == the manifest's claimed 216 nightly cases.

## 2. What was checked and how
- PNG magic/IHDR/dimensions/IDAT decode: audit/scratch/W16-A10/batch.py (single batch script; see batchmeta.txt).
- Consumer wiring: visual.spec.ts:103 + playwright.visual.config.ts:25 (Read).
- Gate wiring: rg "visual" in audit/runs/baseline/full_gate.txt and audit/runs/baseline/rc-smoke.txt -> NO visual section — the screenshot-diff harness is not run by any committed gate; runs are recorded claims in ui/e2e/visual/README.md (pasted output: "npm run visual:nightly / 32 passed (54.2s)"). Execution here: UNVERIFIED (playwright not run in this sandbox; no baseline artifact).
- Matrix evidence: no docs/feature-matrix.toml row cites baseline PNGs (rg: no hits); parity-report verified-only remains 0/252.

## 3. Findings
None beyond the harness-level gap recorded in every baseline audit: the screenshot-diff lane is not wired into any committed gate (full_gate/rc-smoke have no visual section). Harness-level, not file-level.
