# AUDIT: ui/src/screens/player/layout.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 129 / 12 tests across 2 describes |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped layout parser (ui/src/screens/player/layout.ts consumed by PlayerScreen); runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "layout.test" audit/runs/baseline/ui-test.txt` → `138: ✓ src/screens/player/layout.test.ts (12 tests) 8ms`; full file read |

## 1. One-paragraph verdict
Clean unit tests of the layout-config parser: default-layout region ordering/anchors, menu tree with ui-action seams, custom-layout order sorting, fallback-to-default semantics, and six distinct REJECTION cases (wrong schema, unknown anchor, duplicate ids, unknown condition token, unresolvable label_key, zero regions) each asserted with the exact error text pattern. Condition-token evaluation covered true/false/undefined.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L138 `✓ ... (12 tests) 8ms` (inside 1705 passed).
- Rejection asserts use both the typed error (LayoutParseError, L65) and message regexes (L75, 89, 100, 110, 114).

## 3. Findings
None. Zero-findings checklist: tautology scan — every expect consumes parser output or thrown error of the real module; no mocks at all in this file (pure unit). Skip scan — no it.skip; baseline 12 passed. Name-vs-body scan — verified per test. Mutation reasoning: accepting unknown anchors fails L75; accepting duplicates fails L89; silent fallback on invalid source fails L65 (still falls back only for missing/blank, L59-61); token mis-evaluation fails L124-127.

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parseLayoutConfig | 11-116 | REAL | — | default composition, menus, sorting, fallback, 6 rejection cases |
| evaluateCondition | 118-129 | REAL | — | token matrix incl. undefined |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 12 | YES | exact output equality + typed-error regexes; verified green baseline L138 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Card 4.C1 — layout-config parsing tests (doc 07 §7.2 consumption side)" | header L1 | REAL | tests exactly the parse/evaluate surface |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
vitest only; real layout module.

## 9. Open questions for the next agent
- None.
