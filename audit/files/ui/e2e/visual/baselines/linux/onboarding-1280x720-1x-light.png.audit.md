# AUDIT: ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the e2e screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` where id == caseId of manifest.ts |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 batch script (audit/runs/W16-A08.fingerprints.txt); `file -b` dims; manifest.ts caseId cross-check |

PNG baseline, 26,078 bytes, sha256 3c67c25021136fb3986735dd9ad988c262ba014b64d6215fc7ab35e2c9425d67, dims 1280 x 704 (8-bit RGB, non-interlaced).
Matrix case (manifest.ts): route /onboarding × 1280×720 @ 1x light — lane: PR-lite (pr=true: 1280x720 @1x).
CAPTURE DIM: file is 1280x704, NOT 1280x720 — legitimate: visual.spec.ts:79-102 clips the screenshot to the #root bounding box, and the onboarding route renders a 704px-tall root at a 720 viewport; the caseId encodes the VIEWPORT, not the capture height. Not a capture bug.
Generated-vs-handmade: browser-captured golden (Playwright chromium screenshot of the dev shell, clipped to #root — visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (cited by manifest.ts:108).
Evidence consumers: the PR-lite/nightly visual lanes (.github/workflows/ui.yml); NO feature-matrix.toml row cites visual baselines as evidence (rg `baselines/linux|toHaveScreenshot` over docs/feature-matrix.toml → 0 hits; its screenshot rows are the video.take-screenshot engine feature).
Honesty note: a drifted UI would fail `toHaveScreenshot` against this byte content in the lane (deterministic clip + animations disabled per visual.spec.ts:18-21); the baseline is load-bearing, not decoration.
