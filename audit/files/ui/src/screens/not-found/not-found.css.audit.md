# AUDIT: ui/src/screens/not-found/not-found.css (quick check)
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 65 / 6 rule blocks (page, hero, cone, title, body, CTA) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (imported by NotFoundScreen.tsx; the 404 route — App.tsx catch-all; classes used at NotFoundScreen.tsx:75-87). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read all 65 lines; Grep `.not-found` classes in NotFoundScreen.tsx (:75-87 — all six consumed, incl. the test's cone query :43); token existence checked against tokens/generated/tokens.css (read in the extensions.css audit: --space-grid, --color-accent, --font-family-sans, --font-size-title, --font-weight-regular, --color-content-bright, --font-size-body, --color-content-secondary all present in the dark :root; content/bright/secondary re-swept for light). Baseline: ui build #22 OK. |

## 1. One-paragraph verdict
Small, clean hero skin: semantic tokens throughout, both theme blocks inherited via tokens (no light overrides needed here since every consumed token has a light value), layout-only raw values expressed as grid multiples, and the two frame-measured exceptions (cone 76px, CTA min-width 100px) both marked `frame:`. The single defect is a header miscount: it claims "the one raw px is the cone's frame-measured art-box width" while the file carries two raw px values — the cone's 76px and the CTA's 100px.

## 2. What actually works (verified)
— verified by Grep: 6/6 class selectors consumed by NotFoundScreen.tsx:75-87 (page, hero, cone, title, body, CTA).
— verified against tokens.css (read this session): every var() consumed here is defined in the generated sheet with light-mode values.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/not-found/not-found.css:6-7 vs :35 and :63
Code:
```css
 * Semantic tokens only (§3.2); the one raw px is the cone's frame-measured
 * art-box width (frame: Onboarding.svg 107px, scaled to the quiet hero).
```
(vs)
```css
  min-width: 100px; /* frame: Onboarding.svg "Next" 100×32 */
```
What is wrong: The header says there is ONE raw px (the cone's width, :35 `width: 76px`); the CTA block at :63 carries a second (`min-width: 100px`). Both are frame-marked, so the discipline holds — only the count is wrong.
Why it matters: Comment precision; a future sweep for "raw px debt" keyed on this header would miss the CTA value.
What correct looks like: "the two raw px are frame-measured art boxes (cone 76px, CTA 100×32)."
Confidence: HIGH — Verification: read both sites.

## 4. Per-function table
| Rule block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.not-found` | 11-19 | REAL | — | Centered flex, grid-multiple padding. |
| `.not-found__hero` | 23-30 | REAL | — | Column ladder, grid-multiple gaps, 440px measure cap. |
| `.not-found__cone` | 34-38 | REAL | — | 76px frame width, accent ink via token. |
| `.not-found__title` | 41-49 | REAL | — | Title scale, regular weight, bright token. |
| `.not-found__body` | 51-57 | REAL | — | 38ch measure, secondary token. |
| `.not-found__cta` | 61-64 | REAL | S4 | Second raw px (F-001 header count). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| NotFoundScreen.test.tsx:43 (queries .not-found__cone) | YES | Structural query ties the test to this skin's class. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (§3.2); the one raw px is the cone's … width" | not-found.css:6-7 | HALF (two raw px, both frame-marked) | F-001 (:35, :63). |
| "Panels are square-edged; rounding lives on the button (r6)" | not-found.css:5 | REAL | No border-radius declared here; the CTA is the Button primitive. |

## 7. Dead / duplicated / unreachable code
None — 6/6 selectors consumed.

## 8. Dependencies & flags
Semantic tokens only; no flags; no light-mode overrides needed (all consumed tokens carry light values in the sheet).

## 9. Open questions for the next agent
None.
