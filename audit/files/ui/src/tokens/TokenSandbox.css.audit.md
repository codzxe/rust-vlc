# AUDIT: ui/src/tokens/TokenSandbox.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 95 / 8 rule blocks (sandbox, controls, buttons, swatches) |
| File verdict | REAL — token-pure stylesheet for the dev fixture (see App.tsx F-001: the fixture ships unguarded) |
| Reachable from shipped entry point? | YES, and that is the defect: it styles /dev/tokens, which ships in production bundles (App.tsx:297, no DEV gate) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg token vars in src/tokens/generated/tokens.css (all 17 distinct vars resolve, exit 0) |

## 1. One-paragraph verdict
Clean proof-of-consumption sheet: every value is a var() into the generated pipeline (17/17 resolve — including radius-card, space-item-pitch, shadow-card, stroke-icon), zero raw hex/px, exactly as its header claims. Its only issue is inherited from the route, not the CSS: this styles a dev fixture that ships as a product route (App.tsx audit F-001) — ~95 LOC of dev-only styling in every production bundle.

## 2. What actually works (verified)
— verified by rg: 17/17 custom properties resolve in src/tokens/generated/tokens.css; the swatch roles match TokenSandbox.tsx's SWATCH_ROLES list (accent, accent-bright, accent-strong, accent-tinted, surface-raised, content-primary — cross-checked against TokenSandbox.tsx:20-27).

## 3. Findings
F-001 [S4]
Where: ui/src/tokens/TokenSandbox.css:1 (context)
Code:
```
/* TokenSandbox styles — card 4.A2 proof-of-consumption (doc 08 §3.2).
```
What is wrong: nothing in the CSS; the sheet is correct. It is shipped dead-weight styling for a dev fixture reachable in production (cross-ref App.tsx F-001 — the route is ungated). Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (rule blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .ts-sandbox (+h1) | 9-23 | REAL | — | tokens only |
| .ts-controls (+h2) | 25-41 | REAL | — | shadow-card token |
| .ts-controls button (+pressed) | 43-57 | REAL | — | aria-pressed state styled |
| .ts-swatches / .ts-swatch + 6 roles | 59-95 | REAL | S4 | roles match the TSX union |

## 5. Test honesty
No tests for the CSS; TokenSandbox harness tests exist in the tokens lane. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No raw hex/rgb/px values" (L3-6) | header | REAL | read — every declaration is var() |

## 7. Dead / duplicated / unreachable code
In-file: none. Product-reachability: the whole sheet is dev-fixture styling shipped in production (cross-ref).

## 8. Dependencies & flags
src/tokens/generated/tokens.css (all vars verified). Imported by TokenSandbox.tsx:16.

## 9. Open questions for the next agent
- None (see App.tsx audit for the /dev/tokens gating fix).
