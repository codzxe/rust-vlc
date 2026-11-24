# AUDIT: ui/e2e/visual/baselines/linux/not-found-1920x1080-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 32,609 / 1799969235f86763 |
| Type / dims | PNG 8-bit RGB, 1920×1080 (caseId `not-found-1920x1080-1x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for the catch-all `*` route (manifest.ts:88-90 "No literal
/not-found route exists: navigating to an unmatched path renders the
catch-all"), 1920×1080 @1× dark. Dimensions match.
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 1799969235f86763….
— Consumer: manifest ROUTES "/not-found" (:90); visual.spec.ts:102.
— Distinct from 2× dark twin (32,587 B, 4129766de5a708da) and from the light
1× (32,251 B, e9fcbfedd9ba951b).
## 3. Findings
None.
