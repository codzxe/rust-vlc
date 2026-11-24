# AUDIT: ui/src/components/form/form.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 622 / 8 selector grammars (field, browse, number, choice, radiogroup, segmented, swatchgroup, valuechip + reduced-motion + light-mode blocks) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by src/components/form/BrowseField.tsx (`import "./form.css"`) and the form components (TextField/NumberField/RadioGroup/SegmentedControl/SwatchGroup/ValueChip) used by settings/convert/onboarding screens routed in App.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -l` per class grammar (all 8 have TSX producers); `rg "form.css"` (import + 4 test files read the CSS); read of full file |

## 1. One-paragraph verdict
Real, fully-produced stylesheet: every class grammar (`lumen-field`, `lumen-browse`, `lumen-number`, `lumen-choice`/`--radio`, `lumen-radiogroup`, `lumen-segmented`, `lumen-swatchgroup`, `lumen-valuechip`) has live TSX producers, and the colocated tests read this very file to enforce the hover/pressed/focus-visible/disabled state matrix. Frame-measured raw values are each marked `frame:` with a stylelint-disable and a named pending token seam — disciplined. The two hygiene items: raw hex fallbacks (`#2a2a2a`/`#3a3a3a`) are dark-mode-only values that the light theme must override (acknowledged and remediated at L603-617, but the seam tokens `--color-surface-input`/`--color-stroke-input` are still pending — every new light surface must remember the override), and the 9 swatch hexes are hand-duplicated outside the generated token system.

## 2. What actually works (verified)
— verified by `rg -l` per grammar: all 8 class families produced by real components (TextField/BrowseField/NumberField/RadioGroup/SegmentedControl/SwatchGroup/ValueChip + ConvertScreen usage of `lumen-browse`); tests read the CSS (`TextField.test.tsx`, `SegmentedControl.test.tsx`, `ValueChip.test.tsx`, `BrowseField.test.tsx` via readFileSync). Baseline: vitest 1705 passed, npm build passes (baseline §13).

## 3. Findings
F-001 [S4]
Where: ui/src/components/form/form.css:62, 68
Code:
```
  border: 1px solid var(--color-stroke-input, #3a3a3a); /* stylelint-disable-line color-no-hex -- frame: W26 input border #3a3a3a; token seam: --color-stroke-input pending (token lane) */
```
What is wrong: token seam still pending — every consumer of `.lumen-field__control`/`.lumen-number__stepper` in light mode depends on the manual L603-617 overrides instead of a semantic token; any new light surface that forgets the override inherits near-black wells (the defect the comment says vision already flagged on /settings/interface + /convert). Confidence: HIGH — Verification: read; `--color-surface-input` absent from the token set (grep in generated tokens not run — UNVERIFIED beyond the comment's own admission).
F-002 [S4]
Where: ui/src/components/form/form.css:501-536
Code:
```
/* stylelint-disable color-no-hex -- frame: Settings - global.svg (9 accent swatch primaries, frame-measured) */
.lumen-swatchgroup__swatch[data-accent="orange"] .lumen-swatchgroup__dot {
  background: #d37918;
}
```
What is wrong: the 9 accent hexes are hand-written duplicates of the design tokens, maintained outside the generated token pipeline — drift risk between this list and the generated `:root[data-accent]` blocks (the comment admits per-accent primary tokens "do not exist"). Confidence: HIGH — Verification: read.

## 4. Per-function table (selector blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-field* | 33-118 | REAL | — | full state matrix incl. aria-invalid |
| .lumen-browse* | 125-173 | REAL | — | produced by BrowseField + ConvertScreen |
| .lumen-number* | 179-236 | REAL | — | steppers; app affordance (no frame) disclosed |
| .lumen-choice* (checkbox) | 242-282 | REAL | — | native input, accent-color |
| .lumen-radiogroup* / --radio | 288-363 | REAL | — | `:has()` selectors for checked/focus — modern-engine dependency |
| Switch section | 365-371 | N/A (moved) | — | skin moved to button.css w26.k01; comment-only block, no dead rules |
| .lumen-segmented* | 378-438 | REAL | — | state matrix + error ring |
| .lumen-swatchgroup* | 446-549 | REAL | S4 | F-002 hex duplication |
| .lumen-valuechip | 559-576 | REAL | — | tabular-nums; documented frame-vs-spec ink deviation |
| reduced-motion | 578-587 | REAL | — | covers all animated primitives |
| light-mode remediation | 603-622 | REAL | S4 | compensates F-001 pending tokens |

## 5. Test honesty
Colocated tests read this file's text and assert state-matrix rules exist (verified imports). Mutation reasoning: deleting a `:focus-visible` rule would fail TextField/SegmentedControl tests. Not executed here individually — baseline vitest 1705 passed covers the suite.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "State matrix (hover/pressed/focus-visible/disabled) enforced by the colocated tests" (L28-29) | header | REAL | 4 test files readFileSync this css (rg) |
| "The w26.d2 light-mode ... override" moved to button.css (L370-371) | comment | REAL-UNPROVEN | button.css contains the switch light block (rg shows the moved-from comment); not line-verified |
| 9 accent swatches frame-measured, orange first (L496-500) | comment | REAL | L502-536 order matches |

## 7. Dead / duplicated / unreachable code
No dead selectors found (all 8 grammars produced). The dead `.lumen-radiogroup__label` rule was already deleted (comment L302-304) — verified absent.

## 8. Dependencies & flags
Consumes design tokens only (contract §4); raw hexes carry stylelint-disables + frame provenance. No feature flags.

## 9. Open questions for the next agent
- `:has()` usage (L336-346, 360-363): confirm the shipped Tauri webview engines (WebKitGTK min version on Linux targets) support `:has()` — if the sandbox GTK is too old the radio-row checked styling silently degrades. UNVERIFIED (desktop build blocked).
