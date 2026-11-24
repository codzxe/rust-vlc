# AUDIT: ui/e2e/visual/baselines/linux/settings-controls-1920x1080-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 94,303 / 0e474007f366c434 |
| Type / dims | PNG 8-bit RGB, 1920×1080 (matches caseId `settings-controls-1920x1080-1x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated Playwright baseline for /settings/controls, 1920×1080 @1×, light,
default accent. Dimensions match; provenance MATRIX-20261113.md (2026-11-13
re-pin). 1× twin (93176 B, fa8e7e690e44bc0a) is dark — this is the light/dark
pair member.
## 2. Verified
— python3 IHDR: 1920×1080, depth 8, colortype 2; sha256 batch 0e474007f366c434….
— Consumer: ui/e2e/visual/visual.spec.ts:102; template
ui/playwright.visual.config.ts:27.
— No doc/matrix row cites this file directly (MATRIX-20261113.md cites the
route, not the case file) — matrix evidence is lane-level.
## 3. Findings
None beyond the systemic S4 recorded in the player-music-2x audit (n/a for
1× files: no dpi collapse applies).
