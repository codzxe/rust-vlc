# AUDIT: ui/e2e/visual/baselines/linux/settings-interface-1920x1080-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 104,042 / e70abbc77fc83efd |
| Type / dims | PNG 8-bit RGB, 1920×1080 (matches caseId `settings-interface-1920x1080-1x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for /settings/interface, 1920×1080 @1× dark, default
accent. Largest settings-area baseline on disk (104 KB — dense interface
area). Dimensions match caseId; provenance MATRIX-20261113.md re-pin.
## 2. Verified
— python3 IHDR: 1920×1080 depth 8 colortype 2; sha256 e70abbc77fc83efd….
— Consumer chain: visual.spec.ts:102 + playwright.visual.config.ts:27.
— Distinct from its 2× dark twin (104,023 B, d7a52ea41a344045) — dpi case
carries resampling info for this screen.
## 3. Findings
None (1× file; systemic scale:"css" note n/a).
