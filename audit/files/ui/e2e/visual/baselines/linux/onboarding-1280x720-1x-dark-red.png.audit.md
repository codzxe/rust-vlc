# AUDIT: ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-dark-red.png
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the e2e screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` where id == caseId of manifest.ts |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 batch script (audit/runs/W16-A08.fingerprints.txt); `file -b` dims; manifest.ts caseId cross-check |

PNG baseline, 25,389 bytes, sha256 518d39e4d59d90fd63e256eddd4c07f22b39661563d71551dd27944a851cec62, dims 1280 x 704 (8-bit RGB, non-interlaced).
Matrix case (manifest.ts): route /onboarding × 1280×720 @ 1x dark — lane: nightly ACCENT SAMPLE (red — one of the 8 non-default accents pinned on /onboarding per manifest.ts:130-133).
CAPTURE DIM: 1280x704 (see the light onboarding baseline note — same #root clip mechanism).
Generated-vs-handmade: browser-captured golden (Playwright chromium screenshot of the dev shell, clipped to #root — visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (cited by manifest.ts:108).
Evidence consumers: the PR-lite/nightly visual lanes (.github/workflows/ui.yml); NO feature-matrix.toml row cites visual baselines as evidence (rg `baselines/linux|toHaveScreenshot` over docs/feature-matrix.toml → 0 hits; its screenshot rows are the video.take-screenshot engine feature).
Honesty note: a drifted UI would fail `toHaveScreenshot` against this byte content in the lane (deterministic clip + animations disabled per visual.spec.ts:18-21); the baseline is load-bearing, not decoration.
