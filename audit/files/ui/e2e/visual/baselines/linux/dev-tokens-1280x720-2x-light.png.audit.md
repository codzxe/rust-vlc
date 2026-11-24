# AUDIT: ui/e2e/visual/baselines/linux/dev-tokens-1280x720-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary asset, 18887 bytes |
| File verdict | REAL-UNPROVEN (committed baseline; height anomaly explained by clip design) |
| Reachable from shipped entry point? | NO (test artifact consumed by ui/e2e/visual/visual.spec.ts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `file` (exit 0); python3 PNG IHDR parse (exit 0) |

## 1. Verdict
Genuine PNG (IHDR 1280x244, 8-bit RGB), 18887 bytes, sha256 46e73cab13fc64cd...
(recomputed). `file`: "PNG image data, 1280 x 244, 8-bit/color RGB, non-interlaced".
Case: route /dev/tokens, 1280x720, DPI 2x, light, default accent.

## 2. The height anomaly (F-001 [S3])
The FILENAME promises 1280x720 but the bitmap is 1280x244. This is not corruption:
visual.spec.ts:80-84 clips to the live `#root` boundingBox
(`const box = await page.locator("#root").boundingBox();` / `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })`),
and the /dev/tokens page's root content is 244 CSS px tall, so the baseline records the
content height, not the viewport. Every other baseline in my assigned set is full
viewport-height because those pages fill it.
Why it still matters (S3): the dev-tokens row's baseline silently varies with page content
height — an unrelated change to the tokens page (adding a row) changes the bitmap HEIGHT,
which Playwright treats as a size mismatch and hard-fails with an opaque diff. The 1280x720
axis in the filename is a lie for this one row (intentional clip, misleading name).
Confidence: HIGH — Verification: read (spec clip logic) + ran (IHDR parse).

## 3. Systemic findings (cross-ref player-music audit)
F-S3: no committed visual-lane run evidence in audit/runs/baseline/ — all 216 baselines
REAL-UNPROVEN as regression evidence. F-S4: ui/README.md:135-136 "full 32-case matrix" rotted
vs the 216-case manifest. UNVERIFIED: pixel content (no Playwright in sandbox).
