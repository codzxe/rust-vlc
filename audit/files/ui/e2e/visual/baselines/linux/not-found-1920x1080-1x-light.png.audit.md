# AUDIT: ui/e2e/visual/baselines/linux/not-found-1920x1080-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 32,251 / e9fcbfedd9ba951b |
| Type / dims | PNG 8-bit RGB, 1920×1080 (caseId `not-found-1920x1080-1x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for the catch-all route, 1920×1080 @1× light. Dimensions
match; sibling set verified consistent (dark 32,609 B / 2× dark 32,587 B —
all distinct).
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 e9fcbfedd9ba951b….
— Consumer: manifest ROUTES "/not-found" (:90); visual.spec.ts:102.
— Theme is forced post-mount via the `data-theme` seam (visual.spec.ts:68-75).
## 3. Findings
None.
