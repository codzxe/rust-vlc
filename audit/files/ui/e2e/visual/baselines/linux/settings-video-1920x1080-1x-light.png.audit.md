# AUDIT: ui/e2e/visual/baselines/linux/settings-video-1920x1080-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 90,262 / ea8ec6ec00806837 |
| Type / dims | PNG 8-bit RGB, 1920×1080 (matches caseId `settings-video-1920x1080-1x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for /settings/video, 1920×1080 @1× light. Dimensions
match; provenance MATRIX-20261113.md re-pin (settings areas enumerated in
manifest.ts ROUTES :76).
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 ea8ec6ec00806837….
— Consumer chain: visual.spec.ts:102; manifest.ts ROUTES includes
"/settings/video".
— Distinct from 2× light twin (90,304 B, 5394060ed339b355).
## 3. Findings
None.
