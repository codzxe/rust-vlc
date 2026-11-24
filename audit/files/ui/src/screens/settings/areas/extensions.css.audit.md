# AUDIT: ui/src/screens/settings/areas/extensions.css
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 89 / n.a. (pure stylesheet, 4 class families) |
| File verdict | REAL-UNPROVEN (presentational; wired; no dead selectors) |
| Reachable from shipped entry point? | YES — imported by extensions.tsx:49 `import "./extensions.css"`; /settings/extensions via App.tsx:322 -> areas registry |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 dead-selector sweep (4/4 used); read of file + consumer spot-check (extensions.tsx:180-215) |

## 1. One-paragraph verdict
Pure presentation for the Extensions area skin (list well, toolbar search, action cluster), token-driven with frame-measured px provenance, light-mode recessed-well handling, and a documented specificity guard for the primary action's width. No dead selectors, no hex escapes, no hacks. Context from the consumer (another agent's file): the toolbar actions this sheet styles are honestly disabled (`data-staged="wasm-extensions"`, empty onClick) — the CSS supports the honest pattern, it does not fake anything.

## 2. What actually works (verified)
— verified by python3 sweep (dead: []) + import grep; extensions.test.tsx:130 reads the CSS (structure pinning).

## 3. Findings
None. Checked: dead selectors, import chain, hex escapes (none outside comments), `!important` (none), specificity doublings (documented, g14-P2-2).

## 4. Per-function table
n.a. — stylesheet. Families: `.settings-extensions-list`, `-toolbar`, `-toolbar__actions`, `-toolbar__find`.

## 5. Test honesty
extensions.test.tsx asserts CSS content (L130 readFileSync) — pins the skin's structure.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "raw px is structural (frame-measured dimensions with no token), marked `frame:`" | header L10-11 | REAL | every raw px carries a `frame:` comment |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Semantic tokens; settings input-well family. No feature flags.

## 9. Open questions for the next agent
None (extensions.tsx belongs to another lane).
