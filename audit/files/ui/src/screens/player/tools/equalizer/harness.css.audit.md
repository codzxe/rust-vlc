# AUDIT: ui/src/screens/player/tools/equalizer/harness.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 21 / 2 rule groups (body reset, .w26c1-stage) |
| File verdict | UNREACHABLE (dev-only harness sheet, correctly excluded from app routes; no shipped binary — W14) |
| Reachable from shipped entry point? | NO — imported only by ui/src/screens/player/tools/equalizer/harness.tsx:15, which is imported only by its own harness.test.tsx:12 (W26C1_HARNESS_OUT export pattern, verified by rg + dir listing). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Clean. A 21-line dev-only layout sheet that does exactly what its header says: mimics the EQ.svg canvas geometry (824px width, 20px insets/gap, each raw px carrying its frame citation), uses two semantic tokens that both exist, and scopes its one global rule (`body { margin: 0 }`) to a module that never loads in app routes — with the rationale documented. Zero findings; everything checked is listed below.

## 2. What actually works (verified)
— verified by `rg`: harness.css is imported only at harness.tsx:15; harness.tsx is imported only by harness.test.tsx:12; no app route pulls it (the "never imported by app routes" claim :1-2 is TRUE).
— verified by `rg` in tokens.css: --color-surface (:54 dark / :130 light) and --font-family-sans (:64) exist.
— verified by read: raw values (824px, 20px×3) each carry an inline frame citation per the repo's css discipline; no raw colors.

## 3. Findings
None. Checked: token existence (both exist), raw-value citation discipline (all cited), dead selectors (.w26c1-stage consumed in harness.tsx, body rule scoped to the dev module), global-rule blast radius (body margin only loads with the dev-only module — disclosed :6-7), import reachability (test-only).

## 4. Per-function table (rule groups)
| Rule group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| body reset | 8-10 | REAL | — | global but dev-module-scoped; rationale documented |
| .w26c1-stage | 12-21 | REAL | — | frame-cited geometry; 2 tokens, both exist |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| equalizer harness.test.tsx (adjacent) | smoke-level only | same shape as the options-harness test I audited: render-without-crash + env-gated export (asserts files exist when W26C1_HARNESS_OUT set); no css assertions — this sheet has none either |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY harness layout… never imported by app routes" | :1-2 | REAL | rg: only harness.tsx:15 → harness.test.tsx:12 |
| "Mimics the EQ.svg canvas (824px wide — panel x20–804 with 20px insets)" | :2-3 | REAL (internally consistent) | width 824 = 20px padding + 784px panel + 20px, matching the panel dimensions cited in effects.tsx's frame notes |
| "Not part of the product CSS" | :4 | REAL | unreachable from app routes |

## 7. Dead / duplicated / unreachable code
None dead; the whole sheet is unreachable from product code by design (verified).

## 8. Dependencies & flags
Tokens only. No imports. Dev tooling for the w26.c1 fidelity loop (screenshots vs the 900px figma render, per w26.e09's method record).

## 9. Open questions for the next agent
- None.
