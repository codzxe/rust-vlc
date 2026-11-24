# AUDIT: ui/src/screens/playlist/subtitles/FilenameMatchControl.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 61 / 5 tests in 1 describe |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for the shipped subtitles FilenameMatchControl; runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "FilenameMatchControl.test" audit/runs/baseline/ui-test.txt` → `161: ✓ src/screens/playlist/subtitles/FilenameMatchControl.test.tsx (5 tests) 166ms`; full file read |

## 1. One-paragraph verdict
Honest dropdown tests over the real control: trigger shows the current strategy's label, the five frame options render in exact order, the selected option is aria-selected with its label, keyboard selection (ArrowDown+Enter) reports "any" through onChange, and the scoped frame-scale class mounts. Values are the real match-mode strings (DEFAULT_MATCH_MODE imported from the real matching module).

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L161 `✓ ... (5 tests) 166ms` (inside 1705 passed).
- Exact option-order assertion (L40-41) and selected-state assertion (L47-49); keyboard path reports the correct next value (L57-59).

## 3. Findings
None. Zero-findings checklist: tautology scan — all expects bind to rendered roles/text/onChange payloads of the real component. Mock-of-subject scan — none; onChange is a spy, DEFAULT_MATCH_MODE from the real module. Skip scan — no it.skip; baseline 5 passed. Name-vs-body scan — verified (the frame-scale test asserts the class mount, exactly what its name says; the 48/44px figures are the comment's frame rationale, not the assertion's claim). Mutation reasoning: wrong option order fails L41; wrong selected state fails L47-49; broken keyboard selection fails L59.

## 4. Per-function table (<100 LOC; still listed)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| trigger label | 21-28 | REAL | — | current strategy visible |
| scoped class | 30-35 | REAL | — | .lumen-plsubs-match mounts |
| five options in order | 37-42 | REAL | — | exact array equality |
| selected option | 44-50 | REAL | — | aria-selected + label |
| keyboard selection | 52-60 | REAL | — | ArrowDown+Enter → onChange("any") |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 5 | YES | rendered-output assertions; verified green baseline L161 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the scoped frame scale hooks" (48px/44px) | header L3 | REAL-UNPROVEN | class mount asserted; the px values live in CSS (UNVERIFIED here — not this test's claim) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@testing-library/react, vitest; real FilenameMatchControl + matching module.

## 9. Open questions for the next agent
- None.
