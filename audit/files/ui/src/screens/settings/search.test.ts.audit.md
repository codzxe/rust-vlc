# AUDIT: ui/src/screens/settings/search.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 58 / 5 tests in 1 describe |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for the shipped settings search; runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "search.test" audit/runs/baseline/ui-test.txt` → `160: ✓ src/screens/settings/search.test.ts (5 tests) 5ms`; full file read |

## 1. One-paragraph verdict
Honest search-semantics tests over the real schema and real labeler: blank queries return nothing (search inactive), label-substring matching case-insensitive both directions against a LIVE frame row, cross-area matching preserving schema order (exact array equality on area ids and entry keys), feature-gated unexposed entries hidden from results, and honest no-match for gibberish. The labeler is built the way generated.test.ts does (registered catalog + lane pre-registration keys) with the scaffold workaround documented in-file.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L160 `✓ ... (5 tests) 5ms` (inside 1705 passed).
- Exact order pin (L43-46) and gated-entry exclusion (L49-53) are the strongest contracts here.

## 3. Findings
None. Zero-findings checklist: tautology scan — all expects consume searchSettings output over the real SETTINGS_AREAS. Mock-of-subject scan — none; useMsg + lane catalogs are real modules. Skip scan — no it.skip; baseline 5 passed. Name-vs-body scan — verified. Mutation reasoning: case-sensitive regression fails L33-37; order scramble fails L43-46; gated-entry leak fails L51-52.

## 4. Per-function table (<100 LOC; still listed)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| blank query | 23-26 | REAL | — | empty/blank → [] |
| case-insensitive substring | 28-38 | REAL | — | both directions on a live row |
| cross-area + schema order | 40-47 | REAL | — | exact arrays |
| gated entries hidden | 49-53 | REAL | — | hw-decode gate off |
| honest no-match | 55-57 | REAL | — | gibberish → [] |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 5 | YES | exact output equality over the real schema; verified green baseline L160 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "useMsg is a plain lookup in the scaffold (no real hooks inside) ... each test builds its labeler the same way generated.test.ts does" | comment L10-14 | REAL-UNPROVEN | consistent with passing tests; generated.test.ts not my file |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
vitest; real search + schema + i18n modules; no mocks.

## 9. Open questions for the next agent
- None.
