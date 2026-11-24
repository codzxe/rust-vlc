# AUDIT: ui/e2e/visual/baselines/linux/settings-playback-1920x1080-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A02) |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the nightly screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` with id == caseId of manifest.ts:142-145 (route /settings/playback × 1920×1080 @ 1x dark) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch (this run): 99,867 bytes, sha256 5dd2b194c0912627e75e865902084bb6548dbab877cd763d524b897e3c058a60, head 89504e47…; `file -b` -> PNG 1920 x 1080, 8-bit RGB non-interlaced; rg over docs/feature-matrix.toml for `baselines/linux` -> 0 hits |

Generated-vs-handmade: browser-captured golden (Playwright chromium screenshot of the dev shell clipped to #root, visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (manifest.ts:108). Lane: nightly only — pr flag is false for this combo (manifest.ts:157 `pr: w === 1280 && h === 720 && dpi === 1`); the PR lane never diffs this baseline (visual.spec.ts:32 filters `c.pr`), so drift here is caught only if the nightly lane actually runs (no committed nightly execution log found — UNVERIFIED lane execution, environment: no playwright here).
Evidence consumers: .github/workflows/ui.yml visual lane. No feature-matrix.toml row cites it.
Honesty: load-bearing byte-exact golden (threshold 0.1/maxDiffPixelRatio 0.001, visual.spec.ts:96-101); a drifted settings-playback screen fails the diff.
