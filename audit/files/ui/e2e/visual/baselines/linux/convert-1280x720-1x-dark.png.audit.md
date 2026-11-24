# AUDIT: ui/e2e/visual/baselines/linux/convert-1280x720-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 58,517 / bef813ba63f561a2 |
| Type / dims | PNG 8-bit RGB, 1280×720 (caseId `convert-1280x720-1x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — PR-lite lane (1280×720 @1×, manifest.ts:157) + nightly; visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for /convert, 1280×720 @1× dark — PR-lane case; route
added by w26.k20 route-inventory fix (manifest.ts:18-19 "the inventory
lagged the app by 9 routes (convert + 8 of the 13 settings areas)").
Dimensions match; provenance MATRIX-20261113.md re-pin.
## 2. Verified
— python3 IHDR: 1280×720 depth 8 colortype 2; sha256 bef813ba63f561a2….
— Consumer: manifest ROUTES "/convert" (:68); visual.spec.ts:102.
— Distinct from 2× dark twin (58,491 B, ae0b01b1dbc61500).
## 3. Findings
None.
