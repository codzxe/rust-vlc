# AUDIT: ui/src/screens/not-found/not-found.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 64 / 5 rule blocks |
| File verdict | REAL — token-pure 404 hero sheet; all 8 vars resolve; consumer verified |
| Reachable from shipped entry point? | YES (App.tsx:310 `*` route -> NotFoundScreen -> imports this sheet; CTA class used at NotFoundScreen.tsx:87) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg token vars in src/tokens/generated/tokens.css (8/8 resolve, exit 0); rg `not-found__cta|NotFoundScreen` (exit 0 — consumer at :87) |

## 1. One-paragraph verdict
Small, clean, complete: every color/typography/spacing value is a token (8/8 resolve in the generated pipeline); the two raw px values (cone 76px, CTA min-width 100px) are frame-measured structural dimensions, documented per-line — exactly the discipline its header claims. No dead selectors: all five blocks are consumed by NotFoundScreen.tsx (class names cross-checked; `not-found__cta` at :87). The 404 route is the catch-all in the shipped tree. Zero findings.

## 2. What actually works (verified)
— verified by rg: 8/8 custom properties resolve; consumer file exists and uses the CTA class; route reachable as the `*` catch-all.

## 3. Findings
None. Checked: token purity (no raw hex/rgb), dead-selector scan (all classes consumed), reduced-motion (no animations in this sheet — nothing to reduce), light-theme (token-driven — adapts automatically).

## 4. Per-function table (rule blocks)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| .not-found | 11-19 | REAL | centered hero |
| __hero | 23-30 | REAL | ladder spacing |
| __cone | 34-38 | REAL | accent ink, documented scale |
| __title / __body | 41-57 | REAL | bright/regular + quiet secondary |
| __cta | 61-64 | REAL | 100px frame width |

## 5. Test honesty
NotFoundScreen.test.tsx mounts the route (L25) — rendering covered; CSS itself untested directly (no animations/states to test). UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (§3.2); the one raw px is the cone's frame-measured art-box width" (L6-7) | header | REAL (with a nit) | 8/8 tokens; actually TWO raw px (cone 76px L35, CTA 100px L63) — both documented structural values, header undercounts by one (cosmetic) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
src/tokens/generated/tokens.css. Imported by NotFoundScreen.tsx. No feature flags.

## 9. Open questions for the next agent
- None.
