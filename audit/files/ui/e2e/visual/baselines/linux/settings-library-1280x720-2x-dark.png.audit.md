# AUDIT: ui/e2e/visual/baselines/linux/settings-library-1280x720-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A02) |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the nightly screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` with id == caseId of manifest.ts:142-145 (route /settings/library × 1280×720 @ 2x dark) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch (this run): 70,122 bytes, sha256 31331f7aed36a6a8e26a8d6edc2427648ed01481208115c2f5df0afcbebb139e, head 89504e47…; `file -b` -> PNG 1280 x 720, 8-bit RGB non-interlaced |

Generated-vs-handmade: browser-captured golden (Playwright chromium, clip to #root, visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (manifest.ts:108). Lane: nightly only (dpi 2 → pr false, manifest.ts:157). Sibling of the 1x PR-lite baseline (same route/theme, +1,077 bytes for the 2x render) — both exist, consistent with the grid enumerating DPIS [1,2] per manifest.ts:55.
Honesty: load-bearing byte golden (threshold 0.1 / maxDiffPixelRatio 0.001, visual.spec.ts:96-101). No prior second sha pin (rg over W16-A08.fingerprints.txt -> 0). No feature-matrix.toml row cites it. Nightly-lane execution UNVERIFIED.
