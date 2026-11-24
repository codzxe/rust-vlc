# AUDIT: ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-dark-cyan.png
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| File verdict | REAL (pinned visual baseline) |
| Reachable from shipped entry point? | YES (test asset for the visual lane: ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`{id}.png`, { clip: box })` — case /onboarding 1280x720 @1x dark, accent cyan) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file -b + sha256 batch (audit/runs/W16-A05.datahashes.tsv); caseId mapping verified against manifest.ts:142-144 |

Binary-asset audit. PNG image data, 1280 x 704, 8-bit/color RGB, non-interlaced; 25362 bytes; sha256 9405062e3925760b… (full hash in audit/runs/W16-A05.datahashes.tsv). The id `onboarding-1280x720-1x-dark-cyan` parses exactly per manifest.ts:142-144 `caseId` (`<route>-<viewport>-<dpi>x-<mode>[-<accent>]`): route /onboarding, viewport 1280x720, DPI 1, theme dark, accent cyan — an ACCENT_SAMPLES case (nightly accent sampling, manifest.ts:167-178). PR lane: NO — nightly accent-sample case (pr:false, manifest.ts:176). Pixel dims 1280 x 704: Playwright `toHaveScreenshot` default `scale: "css"` stores CSS-pixel geometry, so @2x buffers are viewport-sized (1920x1080, not 3840x2160); heights below the viewport elsewhere in the set reflect the `#root` bounding-box clip (visual.spec.ts:81-84).
Provenance & execution: ui/e2e/visual/MATRIX-20261113.md:66-69 — nightly capture `216/216 re-generated, 216 passed (8.2m), exit 0`; PR capture `npm run visual:update — 50 passed (1.7m)`; PR compare `npm run visual:pr — 50 passed`. The 216 on-disk baselines map 1:1 to the 216-case MANIFEST (200 GRID + 16 ACCENT_SAMPLES, manifest.ts:181). No finding: current-manifest-shaped, hash recorded, compare lane's committed result green.
