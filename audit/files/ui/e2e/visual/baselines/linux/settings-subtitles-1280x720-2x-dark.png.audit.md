# AUDIT: ui/e2e/visual/baselines/linux/settings-subtitles-1280x720-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 76,206 / de935a85671b0060 |
| Type / dims | PNG 8-bit RGB, 1280×720 (caseId `settings-subtitles-1280x720-2x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Generated baseline for /settings/subtitles, 1280×720, dpi 2, dark. Dimensions
match caseId. Provenance MATRIX-20261113.md re-pin. Distinct from its 1× dark
twin (76,090 B, a8cd223957f8bd1e) — the dpi case pins supersampled content
for this screen.
## 2. Verified
— python3 IHDR: 1280×720 depth 8 colortype 2; sha256 de935a85671b0060….
— Consumer: visual.spec.ts:102 with `test.use({ viewport, deviceScaleFactor:
c.dpi })` (visual.spec.ts:39-42).
## 3. Findings
F-001 [S4] systemic: pinned at CSS resolution, not 2560×1440, because
`expect.toHaveScreenshot.scale:"css"` (playwright.visual.config.ts:40)
downscales dpi-2 captures — the DPI axis never compares device-pixel output.
Confidence: HIGH (all 100 2× baselines verified CSS-size).
