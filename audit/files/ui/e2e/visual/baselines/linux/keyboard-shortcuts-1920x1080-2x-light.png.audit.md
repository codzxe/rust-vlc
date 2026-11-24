# AUDIT: ui/e2e/visual/baselines/linux/keyboard-shortcuts-1920x1080-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 68,389 / 040ef9736fec2cdc |
| Type / dims | PNG 8-bit RGB, 1920×1080 (caseId `keyboard-shortcuts-1920x1080-2x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Generated baseline for /keyboard-shortcuts, 1920×1080, dpi 2, light. Dimensions
match; distinct from 1× light twin (69,672 B, 8f44a412779d858b).
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 040ef9736fec2cdc….
— Consumer: manifest ROUTES "/keyboard-shortcuts" (:86); visual.spec.ts:102.
— Text-dense screen (shortcut table) — the dpi case pins downscaled
supersampled text AA.
## 3. Findings
F-001 [S4] systemic scale:"css" DPI note (see player-music-2x audit).
Confidence: HIGH.
