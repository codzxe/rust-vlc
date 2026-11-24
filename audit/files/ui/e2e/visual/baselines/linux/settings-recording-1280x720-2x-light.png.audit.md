# AUDIT: ui/e2e/visual/baselines/linux/settings-recording-1280x720-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 76,013 / 63e7c8a6eb5865bb |
| Type / dims | PNG 8-bit RGB, 1280×720 (caseId `settings-recording-1280x720-2x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Generated baseline for /settings/recording, 1280×720, dpi 2, light. Dimensions
match; distinct from 1× light twin (75,963 B, 851bca4f728c0120).
## 2. Verified
— python3 IHDR: 1280×720 depth 8 colortype 2; sha256 63e7c8a6eb5865bb….
— Consumer: manifest ROUTES "/settings/recording" (:83); visual.spec.ts:102.
## 3. Findings
F-001 [S4] systemic scale:"css" DPI note (see player-music-2x audit).
Confidence: HIGH.
