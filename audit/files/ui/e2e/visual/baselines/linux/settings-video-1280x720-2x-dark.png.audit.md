# AUDIT: ui/e2e/visual/baselines/linux/settings-video-1280x720-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A02) |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the nightly screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` with id == caseId of manifest.ts:142-145 (route /settings/video × 1280×720 @ 2x dark) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch (this run): 68,155 bytes, sha256 e8e7201916276bc4fc38ed7bab2416e5fdf4422a4b3389d16aa80754913cb752, head 89504e47…; `file -b` -> PNG 1280 x 720, 8-bit RGB non-interlaced; rg `settings-video` audit/runs/W16-A08.fingerprints.txt -> 0 hits (no prior second pin) |

Generated-vs-handmade: browser-captured golden (Playwright chromium, clip to #root, visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (manifest.ts:108). Lane: nightly only (dpi 2 → pr false, manifest.ts:157).
Honesty: load-bearing byte golden (threshold 0.1 / maxDiffPixelRatio 0.001, visual.spec.ts:96-101); a drifted settings-video screen fails the diff. Nightly-lane execution UNVERIFIED (no committed run log; no playwright in this environment). No feature-matrix.toml row cites it.
