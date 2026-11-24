# AUDIT: ui/e2e/visual/baselines/linux/settings-network-1280x720-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 65,699 / 97fcaab83203a1b0 |
| Type / dims | PNG 8-bit RGB, 1280×720 (caseId `settings-network-1280x720-1x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — PR-lite lane (1280×720 @1×, manifest.ts:157) + nightly; visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for /settings/network, 1280×720 @1× light — PR-lane case.
Dimensions match; provenance MATRIX-20261113.md re-pin.
## 2. Verified
— python3 IHDR: 1280×720 depth 8 colortype 2; sha256 97fcaab83203a1b0….
— Consumer: visual.spec.ts:32 pr-filter; ui/package.json:18 `visual:pr`.
— Distinct from 2× light twin (65,938 B, b27652f6205a6911).
## 3. Findings
None (1× file; systemic DPI note n/a).
