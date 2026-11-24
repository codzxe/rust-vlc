# AUDIT: ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 25391 B / cda51b69146385e57891ff3dc4fd9321156e74f1b32ce51fd38d98c13206ea38 |
| Dimensions | 1280x704 (bit depth 8, color type 2 RGB8, non-interlaced) |
| File verdict | REAL (pinned screenshot baseline; content is a real rendered screen) |
| Reachable from shipped entry point? | NO — test asset; consumed by ui/e2e/visual/visual.spec.ts via toHaveScreenshot + snapshotPathTemplate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. Verdict (binary-asset audit)
Real PNG baseline (not a placeholder): 249 unique pixel rows among the first 2000 decoded rows (704 rows total, 2704064 raw bytes) — a solid-color or 1x1 placeholder would show 1-2. Dimensions 1280x704 (filename encodes 1280x720 — MISMATCH). Consumed exactly: `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` (ui/e2e/visual/visual.spec.ts:103) with `snapshotPathTemplate: "e2e/visual/baselines/{platform}/{arg}{ext}"` (ui/playwright.visual.config.ts:25) — the case-id `onboarding-1280x720-1x-dark` maps to this file on Linux. Baseline set completeness verified: 216 files in baselines/linux == the manifest's claimed 216 nightly cases.

## 2. What was checked and how
- PNG magic/IHDR/dimensions/IDAT decode: audit/scratch/W16-A10/batch.py (single batch script; see batchmeta.txt).
- Consumer wiring: visual.spec.ts:103 + playwright.visual.config.ts:25 (Read).
- Gate wiring: rg "visual" in audit/runs/baseline/full_gate.txt and audit/runs/baseline/rc-smoke.txt -> NO visual section — the screenshot-diff harness is not run by any committed gate; runs are recorded claims in ui/e2e/visual/README.md (pasted output: "npm run visual:nightly / 32 passed (54.2s)"). Execution here: UNVERIFIED (playwright not run in this sandbox; no baseline artifact).
- Matrix evidence: no docs/feature-matrix.toml row cites baseline PNGs (rg: no hits); parity-report verified-only remains 0/252.

### Filename/dimension mismatch
The case id encodes viewport 1280x720 but the pinned image is 1280x704. Cause: visual.spec.ts clips to the `#root` bounding box (visual.spec.ts:97-102), and the onboarding route's #root box is 16 px shorter than the viewport — the baseline bakes that layout state in. Comparison still works (it diffs against the pinned clip), but the name promises a dimension the pixels do not have.

## 3. Findings
F-001 [S4]
Where: ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-dark.png (content) vs the case id encoded in the filename
What is wrong: filename encodes 1280x720; IHDR says 1280x704. Why it matters: misleading asset naming; also evidence the onboarding route renders a #root box 16 px short of the viewport (layout quirk baked into the pinned baseline). What correct looks like: either fix the route layout or name/record the clip size honestly.
Confidence: HIGH — Verification: ran (batch.py IHDR parse).
