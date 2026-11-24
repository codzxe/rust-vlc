# AUDIT: ui/e2e/visual/baselines/linux/open-network-stream-1280x720-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the e2e screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` where id == caseId of manifest.ts |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 batch script (audit/runs/W16-A08.fingerprints.txt); `file -b` dims; manifest.ts caseId cross-check |

PNG baseline, 41,242 bytes, sha256 3bcd6539e364ce9a516863fa1836d757c57f774a3111c5a48e516b656933db9c, dims 1280 x 720 (8-bit RGB, non-interlaced).
Matrix case (manifest.ts): route /open-network-stream × 1280×720 @ 2x dark — lane: nightly (2x DPI axis).

Generated-vs-handmade: browser-captured golden (Playwright chromium screenshot of the dev shell, clipped to #root — visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (cited by manifest.ts:108).
Evidence consumers: the PR-lite/nightly visual lanes (.github/workflows/ui.yml); NO feature-matrix.toml row cites visual baselines as evidence (rg `baselines/linux|toHaveScreenshot` over docs/feature-matrix.toml → 0 hits; its screenshot rows are the video.take-screenshot engine feature).
Honesty note: a drifted UI would fail `toHaveScreenshot` against this byte content in the lane (deterministic clip + animations disabled per visual.spec.ts:18-21); the baseline is load-bearing, not decoration.
