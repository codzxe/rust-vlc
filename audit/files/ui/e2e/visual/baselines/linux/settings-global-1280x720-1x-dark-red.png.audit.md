# AUDIT: ui/e2e/visual/baselines/linux/settings-global-1280x720-1x-dark-red.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 71,237 / 6d674daf8d5e61dc |
| Type / dims | PNG 8-bit RGB, 1280×720 (caseId `settings-global-1280x720-1x-dark-red`) |
| File verdict | REAL (generated accent-sample baseline) |
| Reachable? | YES — nightly ACCENT_SAMPLES lane (manifest.ts:167-178); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
One of the 16 nightly accent-sample cases: /settings/global at 1280×720 @1×
dark with non-default accent `red` (ACCENT_SAMPLE_ROUTES manifest.ts:130-133
+ ACCENTS :117-127; caseId appends the accent when ≠ DEFAULT_ACCENT orange,
manifest.ts:142-145). Dimensions match. Genuinely distinct bytes from the
other 7 accent siblings of this route (71,203–71,328 B each, distinct
sha256) — the accent sampling pins real color differences.
## 2. Verified
— python3 IHDR: 1280×720 depth 8 colortype 2; sha256 6d674daf8d5e61dc….
— Consumer: visual.spec.ts:68-75 forces `data-accent` before the shot.
— All 8 accent variants of settings-global differ pairwise (sha256 batch).
## 3. Findings
None.
