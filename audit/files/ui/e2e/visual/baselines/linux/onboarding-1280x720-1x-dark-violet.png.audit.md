# AUDIT: ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-dark-violet.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 25,357 / 94dbf2d940d8dfd1 |
| Type / dims | PNG 8-bit RGB, 1280×**704** (caseId `onboarding-1280x720-1x-dark-violet`) |
| File verdict | REAL (generated accent-sample baseline) |
| Reachable? | YES — nightly ACCENT_SAMPLES lane (manifest.ts:167-178); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Verdict
Accent-sample baseline: /onboarding 1280×720 @1× dark, accent `violet`.
Image height 704 px (#root clip). Distinct from the other 7 accent siblings —
real accent coverage; unlike the DEFAULT-accent onboarding grid cases, this
pair family carries no dpi twin (accent samples are pinned @1× only by
design, manifest.ts:164-178).
## 2. Verified
— python3 IHDR: 1280×704 depth 8 colortype 2; sha256 94dbf2d940d8dfd1….
— Consumer: visual.spec.ts:68-75 + :102; manifest ACCENTS includes "violet"
(:126).
## 3. Findings
F-001 [S4] filename-vs-content: 720 claimed, 704 captured. Confidence: HIGH.
