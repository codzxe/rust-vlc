# AUDIT: ui/e2e/visual/baselines/linux/settings-global-1920x1080-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the e2e screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` where id == caseId of manifest.ts |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 batch script (audit/runs/W16-A08.fingerprints.txt); `file -b` dims; manifest.ts caseId cross-check |

PNG baseline, 85,250 bytes, sha256 a1261f8d54dc3b699442f40b143def682b06cd8b23278b8d14516c5d5f531b90, dims 1920 x 1080 (8-bit RGB, non-interlaced).
Matrix case (manifest.ts): route /settings/global × 1920×1080 @ 1x light — lane: nightly (1080 viewport axis).

Generated-vs-handmade: browser-captured golden (Playwright chromium screenshot of the dev shell, clipped to #root — visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (cited by manifest.ts:108).
Evidence consumers: the PR-lite/nightly visual lanes (.github/workflows/ui.yml); NO feature-matrix.toml row cites visual baselines as evidence (rg `baselines/linux|toHaveScreenshot` over docs/feature-matrix.toml → 0 hits; its screenshot rows are the video.take-screenshot engine feature).
Honesty note: a drifted UI would fail `toHaveScreenshot` against this byte content in the lane (deterministic clip + animations disabled per visual.spec.ts:18-21); the baseline is load-bearing, not decoration.
