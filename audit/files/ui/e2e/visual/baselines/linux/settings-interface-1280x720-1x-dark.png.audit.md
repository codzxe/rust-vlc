# AUDIT: ui/e2e/visual/baselines/linux/settings-interface-1280x720-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 81,010 / 3bdb01661f43bd08 |
| Type / dims | PNG 8-bit RGB, 1280×720 (caseId `settings-interface-1280x720-1x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — PR-lite lane (pr=true for 1280×720 @1×, manifest.ts:157) + nightly; visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for /settings/interface, 1280×720 @1× dark — a PR-lane
case (`pr: w === 1280 && h === 720 && dpi === 1`, manifest.ts:157), so it
runs on every PR, not just nightly. Dimensions match; provenance
MATRIX-20261113.md re-pin.
## 2. Verified
— python3 IHDR: 1280×720 depth 8 colortype 2; sha256 3bdb01661f43bd08….
— Consumer: visual.spec.ts:32 filters `c.pr` for MODE=pr; ui/package.json:18
`visual:pr`.
— Distinct from its 2× dark twin (81,116 B, bf52e449be9b2cfb).
## 3. Findings
None.
