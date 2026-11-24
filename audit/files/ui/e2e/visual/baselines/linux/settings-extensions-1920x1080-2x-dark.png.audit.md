# AUDIT: ui/e2e/visual/baselines/linux/settings-extensions-1920x1080-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 55,139 / 383f596b07d1035a |
| Type / dims | PNG 8-bit RGB, 1920×1080 (caseId `settings-extensions-1920x1080-2x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Generated baseline for /settings/extensions, 1920×1080, dpi 2, dark.
Dimensions match; distinct from 1× dark twin (55,164 B, 8c78f6aaebe763d7).
Extensions area was one of the 8 routes added by w26.k20 (manifest.ts:18-19).
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 383f596b07d1035a….
— Consumer: manifest ROUTES "/settings/extensions" (:84); visual.spec.ts:102.
## 3. Findings
F-001 [S4] systemic scale:"css" DPI note (see player-music-2x audit).
Confidence: HIGH.
