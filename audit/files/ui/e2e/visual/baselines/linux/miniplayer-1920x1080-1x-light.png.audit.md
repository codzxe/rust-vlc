# AUDIT: ui/e2e/visual/baselines/linux/miniplayer-1920x1080-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 19,613 / e45d62e12a43b7c2 |
| Type / dims | PNG 8-bit RGB, 1920×1080 (caseId `miniplayer-1920x1080-1x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for /miniplayer, 1920×1080 @1× light. Dimensions match;
distinct from 2× light twin (19,811 B, a15a3e13049c400d).
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 e45d62e12a43b7c2….
— Consumer: manifest ROUTES "/miniplayer" (:64); visual.spec.ts:102.
— MATRIX-20261113.md records the w26.i miniplayer 391px lane change as part
of the re-pin rationale (music left-dock + miniplayer 391px).
## 3. Findings
None.
