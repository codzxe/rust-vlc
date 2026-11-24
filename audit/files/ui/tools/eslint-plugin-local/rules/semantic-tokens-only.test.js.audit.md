# AUDIT: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.test.js
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 99 / 1 test (RuleTester) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as lint-infrastructure for the shipped UI tree; runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "semantic-tokens-only.test" audit/runs/baseline/ui-test.txt` → `144: ✓ tools/eslint-plugin-local/rules/semantic-tokens-only.test.js (1 test) 82ms`; full file read |

## 1. One-paragraph verdict
Real RuleTester coverage of the token-discipline rule: valid cases pin var(--token) dimensions, non-dimension px tolerance, test/generated/css-file exemptions, .ts coverage (the v2 hole), the `/* frame: <Frame.svg> */` escape (both preceding-line and same-line citation forms), and the message-catalog-as-data exemption; invalid cases pin raw hex in tsx AND ts, raw px on font-size keys and JSX radius attributes, rgb() smuggled through template literals, and the non-frame-comment non-exemption (prose before a hex does not exempt). Both sides of every exemption edge are covered.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L144 `✓ ... (1 test) 82ms` (inside 1705 passed).
- The v3 regression hole (hex in .ts helpers) is pinned as invalid at L80-81; the frame-escape is pinned valid at L45-50 and its abuse invalid at L84-88.

## 3. Findings
None. Zero-findings checklist: tautology scan — RuleTester asserts real rule verdicts; no self-comparisons. Mock-of-subject scan — none; real rule module loaded. Skip scan — no it.skip; baseline 1 passed. Name-vs-body scan — verified. Mutation reasoning: dropping .ts coverage breaks L80-81 (case becomes valid → RuleTester failure); accepting any comment as frame-citation breaks L84-88; ignoring template literals breaks L68-73.

## 4. Per-function table (<100 LOC; still listed)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| token discipline matrix | 24-97 | REAL | — | 10 valid + 7 invalid, exemptions pinned from both sides |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the test | YES | RuleTester valid/invalid contract; verified green baseline L144 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the contract §4.4 `/* frame: <Frame.svg> */` escape for frame-measured ports" | header L4-6 | REAL | escape forms pinned L45-50, abuse pinned L84-88 |
| "the message catalog is DATA (displayed copy ...), not paint code; the discipline lives at the consumption site" | comment L51-53 | REAL | exemption pinned L54-57 with filename src/i18n/messages.ts |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
eslint RuleTester, vitest, real rule module.

## 9. Open questions for the next agent
- None.
