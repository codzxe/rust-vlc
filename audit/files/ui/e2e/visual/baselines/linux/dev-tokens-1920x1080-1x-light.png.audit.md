# AUDIT: ui/e2e/visual/baselines/linux/dev-tokens-1920x1080-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 19,978 / 1c2faf6a0d778924 |
| Type / dims | PNG 8-bit RGB, 1920×**244** (caseId `dev-tokens-1920x1080-1x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |

## 1. Verdict
Generated baseline for /dev/tokens, 1920 wide but **244 px tall** (#root clip
— the token grid page is shorter than the viewport). BYTE-IDENTICAL to its
dpi-2 twin `dev-tokens-1920x1080-2x-light.png` (sha256 1c2faf6a0d778924
both) — flat-color grid renders identically at 1× and downscaled 2×.
## 2. Verified
— python3 IHDR: 1920×244 depth 8 colortype 2; sha256 batch.
— Byte-identity verified across all 4 dev-tokens 1×/2× pairs (dark+light ×
both viewports).
— Consumer: manifest ROUTES "/dev/tokens" (:91); visual.spec.ts:102.
## 3. Findings
F-001 [S3] dpi-2 twin is byte-identical (see dev-tokens-1280x720-1x-dark
audit F-001) — the 2× case for this screen pins zero information beyond the
1× case. Confidence: HIGH — Verification: executed sha256 compare.
F-002 [S4] filename-vs-content: 1080 claimed, 244 captured. Confidence: HIGH.
