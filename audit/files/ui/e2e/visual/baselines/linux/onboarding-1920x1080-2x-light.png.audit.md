# AUDIT: ui/e2e/visual/baselines/linux/onboarding-1920x1080-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 31,224 / 0c336fec5d1ce4ef |
| Type / dims | PNG 8-bit RGB, 1920×**1064** — byte-identical to `onboarding-1920x1080-1x-light.png` |
| File verdict | SHALLOW (baseline dup — the dpi axis adds nothing for this screen) |
| Reachable? | YES — nightly lane (`visual:nightly`, ui/package.json:19); visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:1 |

## 1. Verdict
This "2×" baseline is a BYTE-FOR-BYTE COPY of the 1× baseline
(sha256 0c336fec5d1ce4ef == 1× light, 31,224 B both). Unlike dev-tokens (a
flat swatch grid where identity is geometrically plausible), onboarding
carries text, the progress ring, and AA edges — a 2× capture downscaled to
CSS size reproducing the 1× render EXACTLY (every glyph, every AA ramp) is
not something I can confirm from bytes alone; either the lane captured it
that way (grayscale AA + hinting flags, see visual config rounds 3-4c) or the
2× files were seeded from the 1× pins during a re-pin. UNVERIFIED which.
The consequence is the same either way: this dpi-2 case pins zero
information beyond the 1× case.
## 2. Verified
— python3 IHDR: 1920×1064 depth 8 colortype 2.
— sha256 compare: identical to onboarding-1920x1080-1x-light.png (batch).
— Consumer: visual.spec.ts:39-42 + :102; scale:"css"
(playwright.visual.config.ts:40).
## 3. Findings
F-001 [S2]
Where: this file + onboarding-1280x720 1×/2× pairs (dark+light)
Code: sha256(onboarding-1920x1080-2x-light.png) == sha256(onboarding-1920x1080-1x-light.png) == 0c336fec5d1ce4ef…
What is wrong: 8 of the 100 dpi-2 cases in the 216-case nightly matrix pin
bytes identical to their 1× counterparts — DPI coverage double-counted.
Why it matters: matrix coverage overstates distinct verified render states;
a dpr-specific regression cannot fail these cases independently.
What correct looks like: device-scale lane or honest case count.
Confidence: HIGH for the byte-identity; UNVERIFIED for the cause (requires
re-running the visual lane — browser run, out of budget here).
F-002 [S4] filename-vs-content: 1080 claimed, 1064 captured. Confidence: HIGH.
