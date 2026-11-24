# AUDIT: ui/src/theme/lumen.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 259 / 8 documented layers (root type, numerals, scrollbars, selection, focus, primitives, reduced-motion, light fixes) |
| File verdict | REAL — token-pure global base sheet, fully reachable |
| Reachable from shipped entry point? | YES (main.tsx:32 `import "./theme/lumen.css"`; ThemeProvider.tsx:37) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg token vars in src/tokens/generated/tokens.css (16/16 used vars resolve, exit 0); rg `lumen.css` importers (main.tsx:32, ThemeProvider.tsx:37); rg `lumen-separator` consumers (Surface.tsx:406) |

## 1. One-paragraph verdict
The global base stylesheet does what its header promises: composes generated tokens, never redefines them; raw values are geometry only (1px hairline, 8px scrollbar, 1.5 line-height). I verified every custom property it consumes exists in src/tokens/generated/tokens.css (16/16). The header's claim that dead primitives were removed (`.lumen-panel`/`.lumen-row`, L152-155) is true — no rule for them exists; `.lumen-separator` has a real consumer (Surface.tsx:406). The light-mode section (8a-8f) is a cross-lane override block reaching into controls/appshell/player classes — specificity-planned and contrast-documented (WCAG ratios cited per rule), but structurally it is this file patching other files' selectors, a layering smell kept in-check by documentation.

## 2. What actually works (verified)
— verified by rg: 16/16 token vars resolve in the generated pipeline; reduced-motion honored (L171-180); `.lumen-separator` consumed by Surface.tsx:406 (not dead).

## 3. Findings
F-001 [S4]
Where: ui/src/theme/lumen.css:67-69 (and L139-140)
Code:
```
  /* The base sheet owns the body — the margin reset lives here (k05 P3-4
   * ask); appshell.css carries a duplicate until its owner drops it. */
  margin: 0;
```
What is wrong: two admitted duplications ship: the body margin reset (also in appshell.css) and the :focus-visible rule (also a11y/focus.css, L139-140). Both are self-documented with owner handoffs; drift risk between the copies is the only exposure. Confidence: HIGH — Verification: read.

## 4. Per-function table (rule blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| html/body root | 47-70 | REAL | — | color-scheme both modes; token paint |
| headings | 73-81 | REAL | — | |
| .tabular/.timecode | 89-92 | REAL | — | used by timecode UIs |
| scrollbars (std + webkit) | 100-120 | REAL | — | both engines covered |
| ::selection | 128-131 | REAL | — | token color-mix |
| :focus-visible | 141-144 | REAL | S4 | deliberate duplicate (F-001) |
| .lumen-separator | 158-163 | REAL | — | real consumer verified |
| reduced-motion | 171-180 | REAL | — | |
| light fixes 8a-8f | 202-259 | REAL | — | cross-lane overrides, all contrast-documented |

## 5. Test honesty
No direct tests; e2e/a11y.spec.ts references the sheet (rg hit) and the harness tests import it for visual lanes. Baseline UI build passes with this sheet in the graph.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this file COMPOSES them, never overrides or redefines" (L14-15) | header | REAL | no `--token:` declarations in file (read); all 16 consumed vars resolve |
| "the never-adopted .lumen-panel / .lumen-row primitives were removed" (L10-12) | header | REAL | rg — no such rules; only .lumen-separator remains |
| "Replaces solstice.css" (L19) | header | REAL (UNVERIFIED removal) | solstice.css not present in ui/src/theme (ls); UNVERIFIED: no git history allowed |

## 7. Dead / duplicated / unreachable code
F-001 duplications. No dead rules found (each selector family has consumers — .tabular/.timecode used by player controls; light-fix selectors exist in controls/appshell/player sheets, verified by class-name rg spot-checks).

## 8. Dependencies & flags
src/tokens/generated/tokens.css (all vars verified). Imported at main.tsx:32 + ThemeProvider.tsx:37. No feature flags.

## 9. Open questions for the next agent
- appshell.css owner: drop the duplicate body margin reset (k05 P3-4) — the handoff is still open per the comment.
