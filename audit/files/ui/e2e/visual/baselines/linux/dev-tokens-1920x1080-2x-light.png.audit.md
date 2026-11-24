# AUDIT: ui/e2e/visual/baselines/linux/dev-tokens-1920x1080-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 19,978 / 1c2faf6a0d778924 |
| Type / dims | PNG 8-bit RGB, 1920×**244** — identical bytes to `dev-tokens-1920x1080-1x-light.png` |
| File verdict | SHALLOW (baseline dup — the dpi axis adds nothing for this screen) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:1 |

## 1. Verdict
This "2×" baseline is a BYTE-FOR-BYTE COPY of the 1× baseline
(sha256 1c2faf6a0d778924 == 1× light, 19,978 B both, 1920×244 both). The
nightly matrix counts this as one of its 216 cases (a dpi-2 case), but the
pinned pixels carry no dpi-2-specific information: the case can only fail
when the 1× case-equivalent rendering also changes. The matrix's DPI
coverage is nominally duplicated here.
## 2. Verified
— python3 IHDR: 1920×244 depth 8 colortype 2.
— sha256 compare: identical to dev-tokens-1920x1080-1x-light.png (this
audit's batch).
— Consumer: visual.spec.ts:39-42 (`deviceScaleFactor: c.dpi`) + :102; the
fresh 2× capture is compared against these bytes after `scale:"css"`
downscale (playwright.visual.config.ts:40).
## 3. Findings
F-001 [S2]
Where: this file + its 7 siblings (dev-tokens ×4 pairs, onboarding ×4 pairs)
Code: sha256(dev-tokens-1920x1080-2x-light.png) == sha256(dev-tokens-1920x1080-1x-light.png)
What is wrong: 8 of the 100 dpi-2 cases in the "216-case nightly matrix"
(MATRIX-20261113.md:56-58 "Nightly matrix = 216 cases") pin pixels
byte-identical to their 1× counterparts — the DPI dimension is provably
redundant for these screens, and structurally weakened for all screens by
`scale:"css"`.
Why it matters: coverage numbers overstate the distinct render states
verified; a dpi-2-only regression (hidpi asset, dpr-dependent layout) cannot
be caught by these cases. What correct looks like: a device-scale lane, or
an honest case count.
Confidence: HIGH — Verification: executed sha256 compare over all 216 files.
F-002 [S4] filename-vs-content: 1080 claimed, 244 captured. Confidence: HIGH.
