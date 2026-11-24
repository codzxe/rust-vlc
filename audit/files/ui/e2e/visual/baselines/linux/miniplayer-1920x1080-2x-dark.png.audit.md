# AUDIT: ui/e2e/visual/baselines/linux/miniplayer-1920x1080-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 19,476 / 386529b0fa07b077 |
| Type / dims | PNG 8-bit RGB, 1920×1080 (caseId `miniplayer-1920x1080-2x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Generated baseline for /miniplayer, 1920×1080, dpi 2, dark — smallest route
family on disk (miniplayer is the compact overlay screen). Dimensions match;
distinct from 1× dark twin (19,238 B, f6af4434a622cf00).
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 386529b0fa07b077….
— Consumer: manifest ROUTES "/miniplayer" (:64); visual.spec.ts:102.
## 3. Findings
F-001 [S4] systemic scale:"css" DPI note (see player-music-2x audit).
Confidence: HIGH.
