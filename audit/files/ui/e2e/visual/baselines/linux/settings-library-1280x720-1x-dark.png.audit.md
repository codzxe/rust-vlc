# AUDIT: ui/e2e/visual/baselines/linux/settings-library-1280x720-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A02) |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the PR-lite screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` with id == caseId of manifest.ts:142-145 (route /settings/library × 1280×720 @ 1x dark) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch (this run): 70,045 bytes, sha256 5520693ab54857b970e6edc14e2144bc9eb7eebdc1ab4962101de171088cc4ac, head 89504e47…; `file -b` -> PNG 1280 x 720, 8-bit RGB non-interlaced; rg `settings-library-1280` audit/runs/W16-A08.fingerprints.txt -> 0 hits (no prior pin) |

Generated-vs-handmade: browser-captured golden (Playwright chromium, clip to #root, visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (manifest.ts:108). Lane: PR-LITE — pr = true for 1280×720 @1x (manifest.ts:157), the lane every PR runs (VISUAL_MODE=pr default, visual.spec.ts:30-32); this baseline IS on the fastest-ratchet path.
Honesty: load-bearing byte golden (threshold 0.1 / maxDiffPixelRatio 0.001, visual.spec.ts:96-101). No feature-matrix.toml row cites it. PR-lane execution UNVERIFIED this session (no playwright in environment; lane wiring verified in config only).
