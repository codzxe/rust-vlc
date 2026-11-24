# AUDIT: ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-dark-gray.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 25,123 / 37b990242ab34d1f |
| Type / dims | PNG 8-bit RGB, 1280×**704** (caseId `onboarding-1280x720-1x-dark-gray`) |
| File verdict | REAL (generated accent-sample baseline) |
| Reachable? | YES — nightly ACCENT_SAMPLES lane (manifest.ts:167-178); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Accent-sample baseline: /onboarding at 1280×720 @1× dark with accent `gray`
(one of 8 non-default accents sampled on this route, manifest.ts:130-133).
Image height is 704 px (the onboarding page is 16 px shorter than the
viewport — `#root` clip semantics, same as dev-tokens). Distinct bytes from
the other 7 accent siblings (25,357–25,490 B each, distinct sha256) — real
accent coverage.
## 2. Verified
— python3 IHDR: 1280×704 depth 8 colortype 2; sha256 37b990242ab34d1f….
— Consumer: visual.spec.ts:68-75 forces data-accent=gray; :102 shot.
— All 8 onboarding accent variants pairwise-distinct (sha256 batch).
## 3. Findings
F-001 [S4] filename-vs-content: 720 claimed, 704 captured (viewport naming
convention). Confidence: HIGH.
