# AUDIT: ui/e2e/visual/baselines/linux/dev-tokens-1280x720-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 18,681 / 62a81d78d01d30f4 |
| Type / dims | PNG 8-bit RGB, 1280×**244** (caseId `dev-tokens-1280x720-1x-dark`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — PR-lite lane (1280×720 @1×, manifest.ts:157) + nightly; visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |

## 1. Verdict
Generated baseline for /dev/tokens (token swatch grid), 1280 wide but only
**244 px tall** — the clip is `#root`'s bounding box (visual.spec.ts:81-84),
and the dev-tokens page is shorter than the viewport. The filename's
"720" names the viewport, not the image height. More significant: this 1×
file is BYTE-IDENTICAL to its dpi-2 twin
`dev-tokens-1280x720-2x-dark.png` (sha256 62a81d78d01d30f4 both) — the
flat-color token grid renders identically at 1× and (downscaled) 2×, so the
2× case of this screen pins zero information beyond the 1× case.
## 2. Verified
— python3 IHDR: 1280×244 depth 8 colortype 2; sha256 batch.
— Byte-identity: dev-tokens 1× vs 2× pairs identical for dark AND light at
both viewports (4 pairs; sha256 compare).
— Consumer: manifest ROUTES "/dev/tokens" (:91); visual.spec.ts:102.
## 3. Findings
F-001 [S3]
Where: the dev-tokens dpi-2 baselines (byte-identical to 1×)
Code: sha256(dev-tokens-1280x720-1x-dark.png) == sha256(dev-tokens-1280x720-2x-dark.png) == 62a81d78d01d30f4…
What is wrong: the nightly matrix counts 4 dev-tokens dpi-2 cases as distinct
coverage; each compares pixels identical to its 1× case.
Why it matters: the DPI axis's coverage claim is weakest exactly where it is
counted; a dpi-2-only regression on this screen cannot fail the 2× case
without also failing the 1× case. What correct looks like: device-scale
comparisons (`scale:"device"` lane) or dropping the redundant cases from the
count. Confidence: HIGH — Verification: executed sha256 compare.
F-002 [S4]
Where: filename vs content — image is 1280×244, not 1280×720 (same for the
1920×1080 files: 1920×244). Naming convention conflates viewport with
captured extent. Confidence: HIGH.
