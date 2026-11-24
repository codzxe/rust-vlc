# AUDIT: ui/e2e/visual/baselines/linux/settings-audio-1280x720-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary asset, 72780 bytes |
| File verdict | REAL-UNPROVEN (committed baseline; harness real; zero committed run evidence) |
| Reachable from shipped entry point? | NO (test artifact consumed by ui/e2e/visual/visual.spec.ts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (systemic S3/S4 filed under player-music audit) |
| Commands run | python3 PNG IHDR parse + sha256 (exit 0); `file` on sample set (exit 0) |

## 1. Verdict
Genuine PNG baseline (IHDR: 1280x720, 8-bit RGB), 72780 bytes, sha256 1af0f398203afa09...
(recomputed). Filename encodes the manifest case axes per caseId
(ui/e2e/visual/manifest.ts:142-146): route /settings/audio, 1280x720, DPI 2x, light, default accent.
Consumed by visual.spec.ts:104 toHaveScreenshot with clip=#root boundingBox;
`scale: "css"` in playwright.visual.config.ts explains 2x baselines matching 1x pixel size.

## 2. Systemic findings (cross-ref)
Same two systemic findings as player-music-1920x1080-2x-light.png audit:
F-S3 no committed visual-lane run evidence anywhere in audit/runs/baseline/;
F-S4 ui/README.md:135-136 "full 32-case matrix" rotted vs 216-case manifest.
UNVERIFIED: pixel content (no Playwright in sandbox; no run log committed).
