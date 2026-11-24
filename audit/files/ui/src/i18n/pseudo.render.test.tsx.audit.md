# AUDIT: ui/src/i18n/pseudo.render.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A04) |
| Date | 2026-06-14 |
| Lines / functions | 63 / 1 renderAt helper + 1 describe block (7 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:163 `✓ src/i18n/pseudo.render.test.tsx (7 tests) 178ms`; subject chain (messages/pseudo + App screens) ships in the UI bundle; `?locale=pseudo` is a supported runtime override (main.tsx bootstrap, header :10-12) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `grep -n pseudo.render audit/runs/baseline/ui-test.txt` -> :163; `rg -n PSEUDO_LOCALE_TAG ui/src/i18n/pseudo.ts` -> :19; read-through of all 7 tests |

## 1. One-paragraph verdict
Genuine i18n wiring proof: renders the real App at seven key routes with the pseudo locale active and asserts the honest triple — no crash, no `undefined`/`missing:` fallback text anywhere in the body, and the h1 actually carries pseudo brackets (proving the locale seam reached rendered copy, not just the hook). Locale state is set and restored around every test (beforeEach/afterEach :32-37). The one deliberate scope cut (pixel-level clipping belongs to the visual harness) is documented in the header rather than hidden.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:163 (7/7 pass; lane 1705 passed :178-179). Mutation reasoning: an unwired locale seam fails :60 (no brackets in the h1); a missing key leaking the debug fallback fails :55-56; a crashing screen under pseudo lengths fails :53.

## 3. Findings
None. Checked for: tautologies (none — the bracket assertion can only pass if the pseudo transform ran), route coverage honesty (7 routes incl. the catch-all not-found), state leakage (locale restored), unwired tests (7/7 run). Zero findings is the honest result.

## 4. Per-function table
File <100 LOC — renderAt (:22-30, real App under ClientProvider+MemoryRouter), locale lifecycle (:32-37), 7 route cases (:40-62). All REAL.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| any route case :52-61 | YES | no-crash + no-fallback + bracket presence |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 7 tests run | baseline ui-test.txt:163 | REAL | `✓ ... (7 tests) 178ms` |
| "pixel-level clipping half belongs to the visual harness" | header :10-12 | REAL-UNPROVEN | visual lane wiring exists (playwright.visual.config.ts + ui.yml :148) but lane execution UNVERIFIED in this environment |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
PSEUDO_LOCALE_TAG from the real pseudo module (:19); setActiveLocale from the real messages module.

## 9. Open questions for the next agent
None.
