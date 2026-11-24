# AUDIT: ui/src/components/slider/slider.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 413 / 5 skins (base 28-108, vertical 121-156, EQSlider 168-209 + 373-413, Seekbar 216-255, VolumeSlider 263-288, states 292-367) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (Slider.tsx:3 `import "./slider.css"`; lumen-seekbar/lumen-volume/lumen-eq-slider all have production consumers — ControlBar.tsx:399 passes engine `state.buffered`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg class consumers (all live); rg token existence (gradient-volume-level, color-eq-fill-* ABSENT — fallbacks standing); sed quote verification |

## 1. One-paragraph verdict
Real primitive stylesheet with unusually well-documented pixel math: the
thumb/fill flush contracts (inset-2 anatomy, lines 66-74, 97-104, 150-155,
198-208, 404-409) state exact formulas and the pre-fix drift they killed, and
EQSlider.test/Slider.test pin them (referenced line 204). Unlike the equalizer
skin, the seekbar half of this sheet IS wired to a real backend: Seekbar.tsx
renders engine-reported buffer ranges (verified duration>0 guard at
Seekbar.tsx:118-122) and ControlBar.tsx:399 feeds `state.buffered` from the
engine view. One S4: two tokens the sheet "awaits" (--gradient-volume-level,
--color-eq-fill-*) do not exist in tokens.css — the fallbacks are declared
"the standing grammar, not a temporary state" (line 62), which is honest, but
the token lane has now been re-logged across at least three waves without
landing (k03 comment).

## 2. What actually works (verified)
— rg consumers: lumen-slider (Slider.tsx), lumen-seekbar (Seekbar.tsx +
ControlBar), lumen-volume (ControlBar), lumen-eq-slider (EQSlider/GraphicEQ).
— Buffer honesty: Seekbar.tsx:118 `(duration > 0 ? buffered : undefined)` —
no Infinity-derived 100% bars; guard matches the CSS comment (j03 P3-13).
— rg tokens.css: no gradient-volume-level / color-eq-fill-* definitions ->
fallback values apply exactly as documented.

## 3. Findings
F-001 [S4]
Where: ui/src/components/slider/slider.css:59-64
Code:
```
 * The dedicated
 * --color-eq-fill-* tokens win once the token lane lands them (still
 * absent from tokens.css as of w26.k03 — j03 P3-10, re-logged for the
 * token lane; the fallbacks below are the standing grammar, not a
 * temporary state); the semantic fallbacks render the frame-measured
```
What is wrong: Perpetual token-lane deferral — same pattern as states.css's 18
hex pins and equalizer.css's tick hex. The fallbacks are correct today
(accent-tinted == #371f06 exactly per comment), so this is documented debt,
not a rendering defect. Confidence: HIGH — Verification: rg tokens.css (absent).

## 4. Per-function (selector block) table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-slider base + track/fill/thumb | 28-108 | REAL | - | flush math documented + test-pinned |
| --vertical scope | 121-156 | REAL | - | mirrored flush contract |
| .lumen-eq-slider scope | 168-209 | REAL | - | flat fill replaces gradient, documented |
| .lumen-seekbar + __buffer | 216-255 | REAL | - | engine-fed buffer overlay |
| .lumen-volume scope | 263-288 | REAL | - | missing token fallback documented |
| states (hover/drag/disabled/pending/empty) + reduced-motion | 292-367 | REAL | - | data-pending = requested-target lock (§5.2.2) |
| .lumen-eq-slider vertical fader | 373-413 | REAL | - | measured pre-fix drift values (406-407) |

## 5. Test honesty
Slider.test.tsx:10 and EQSlider.test.tsx:7 read this CSS file as text and pin
the formula strings — presence tests that would catch formula deletion but not
computed-position regressions. Owned by another agent; noted.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "buffer sub-regions — engine-reported, honest overlay" | slider.css:233 | REAL | Seekbar.tsx:127 renders engine ranges; ControlBar.tsx:399 state.buffered |
| "accent.tinted == #371f06 exactly; accent-strong #ab6314 ≈ end #a5621a" | slider.css:63-64 | UNVERIFIED | token hex values not re-measured (token file owned by another lane) |

## 7. Dead / duplicated / unreachable code
None. Every selector family consumed; the seekbar empty-state guard
(355-358) is belt-and-braces, ordered last, documented (351-354).

## 8. Dependencies & flags
tokens.css (--radius-control, --radius-inset, surface/content/accent ramp,
--opacity-disabled). Custom properties --lumen-slider-fill/--thumb set by
Slider.tsx (consumed also by states.css preview seek overrides — verified
states.css:705,714).

## 9. Open questions for the next agent
Slider.tsx / Seekbar.tsx owners: verify pointer math (pointer events -> %
-> commit) and that keyboard seek exists; the CSS assumes --lumen-slider-fill
0-100 integers.
