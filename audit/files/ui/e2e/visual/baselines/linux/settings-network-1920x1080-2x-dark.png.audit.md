# AUDIT: ui/e2e/visual/baselines/linux/settings-network-1920x1080-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 79,378 / 18e4c013d44669d3 |
| Type / dims | PNG 8-bit RGB, 1920×1080 (caseId `settings-network-1920x1080-2x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Generated baseline for /settings/network, 1920×1080, dpi 2, dark. Dimensions
match caseId; distinct from its 1× dark twin (79,399 B, 159c2409f35a7eaf) —
dpi case carries resampling info.
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 18e4c013d44669d3….
— Consumer: visual.spec.ts:39-42 deviceScaleFactor rides test.use; shot at
visual.spec.ts:102.
## 3. Findings
F-001 [S4] systemic (see player-music-2x audit): pinned at CSS resolution
(scale:"css", playwright.visual.config.ts:40) — DPI axis never compares
device-pixel output. Confidence: HIGH.
