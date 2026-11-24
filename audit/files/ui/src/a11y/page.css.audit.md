# AUDIT: ui/src/a11y/page.css
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A04) |
| Date | 2026-11-23 |
| Lines / functions | 14 / 1 rule (`body`) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — main.tsx:37 `import "./a11y/page.css"` (entry stylesheet chain, loaded after tokens.css/lumen.css/focus.css) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg token definitions in tokens.css (exit 0); rg page.css importers (exit 0); sed lumen.css:50-72; rg axe/4.F4 references (exit 0) |

## 1. One-paragraph verdict
Two-declaration paint floor: body background + text color from semantic tokens.
Both tokens resolve in both theme modes (tokens.css:54 `--color-surface:
#090909` dark / :130 `#c2c2c2` light; :39/:123 `--color-content-primary`) —
the "both theme modes resolve" claim verified. Loaded last in main.tsx
(:32 lumen.css, :36 focus.css, :37 page.css) so the floor wins any specificity
tie. The duplication with lumen.css:58-62 is real but documented on both sides
(lumen.css:59-60 "the body floor (a11y/page.css) paints the same token; kept
here so the base sheet alone never shows raw white") with identical values —
deliberate defense-in-depth, zero behavioral divergence.

## 2. What actually works (verified)
— verified by `rg -n -- "--color-surface:" src/tokens/generated/tokens.css` ->
  dark #090909 (line 54) and light #c2c2c2 (line 130) definitions exist.
— verified by `rg -n "page.css|lumen.css" src/main.tsx` -> import chain at
  lines 32/37 with page.css last.
— Baseline ui-build.txt: Vite build compiles the full CSS chain (exit 0).

## 3. Findings
None. Checked: raw hex literals (none — tokens only), dead rule (none — main.tsx
imports it; body always matches), mode coverage (both modes define both
tokens), conflicting body rules (lumen.css:58 sets the same values + fonts;
documented, identical), specificity wars (same selector, last import wins,
values identical anyway).

## 4. Per-function table
Not applicable — CSS, single rule, < 100 LOC.

## 5. Test honesty
No test pins the body background rule directly. UNVERIFIED whether any axe run
asserts body paint; tokens.test.ts:134-138 pins token values, not this rule.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Tokens only, both theme modes resolve" | page.css:9 | REAL | tokens.css:54/130 + :39/:123 define all values in dark+light |
| "caught by the 4.F4 axe suite" | page.css:5 | REAL-UNPROVEN | 4.F4 contrast tests exist (Toggle.test.tsx:147, List.test.tsx:545 axe color-contrast audit mention) but no current test pins the body floor itself |
| "Screen roots still paint their own surfaces" | page.css:6-7 | UNVERIFIED | Not re-verified per-screen this run (playlist/media-info screens' own paint not inspected) |

## 7. Dead / duplicated / unreachable code
Documented duplication with lumen.css:58-62 (identical background/color
values). Intentional per comments in both files; appshell.css also references
page.css. No dead code.

## 8. Dependencies & flags
Tokens from src/tokens/generated/tokens.css (generated file, drift-gated per
W15-A02's generated/README audit). No feature gates.

## 9. Open questions for the next agent
If a strict-single-source rule is ever enforced, one of the two body rules
(here vs lumen.css:58) should be dropped — currently harmless.
