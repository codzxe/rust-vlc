# AUDIT: ui/e2e/visual/baselines/linux/player-music-1280x720-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 95,865 / d045513ee8d73a4d |
| Type / dims | PNG 8-bit RGB, 1280×720 (matches caseId `player-music-1280x720-2x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly visual lane: `ui/package.json:19` `visual:nightly`; consumed by `ui/e2e/visual/visual.spec.ts:102` via `snapshotPathTemplate` (`ui/playwright.visual.config.ts:27`) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Generated Playwright baseline (route /player/music, viewport 1280×720, dpi 2,
dark, default accent orange). Dimensions match the caseId; provenance per
ui/e2e/visual/MATRIX-20261113.md (2026-11-13 full re-pin, Playwright 1.64.1,
Chromium 156.0.8078.4 headless, deterministic font/color flags). Distinct
bytes from its 1× twin (687f5e40105ea4b4) — the dpi case carries resampling
information here.
## 2. Verified
— python3 IHDR parse: sig ok, 1280×720 depth 8 colortype 2.
— sha256 batch (this audit's run): d045513ee8d73a4d…, 95,865 B.
— Consumer chain: visual.spec.ts:102 `await expect(page).toHaveScreenshot(\`${id}.png\`, { clip: box })`.
## 3. Findings
F-001 [S4] systemic (see playwright.visual.config.ts audit): `expect.toHaveScreenshot.scale:"css"`
downscales the dpi-2 capture to CSS pixels, so the pinned image is 1280×720,
not 2560×1440 — the DPI axis never compares device-pixel output; it pins
supersampled-then-halved pixels. Confidence: HIGH (verified on all 100 2×
baselines: every one is CSS-size).
