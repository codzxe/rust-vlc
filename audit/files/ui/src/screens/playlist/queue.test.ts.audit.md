# AUDIT: ui/src/screens/playlist/queue.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 69 / 12 tests across 4 describes |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped queue helper functions (ui/src/screens/player uses dragToMoveIndex semantics — see variants.test.tsx L328-333); runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "queue.test" audit/runs/baseline/ui-test.txt` → `162: ✓ src/screens/playlist/queue.test.ts (12 tests) 5ms`; full file read |

## 1. One-paragraph verdict
Pure-function unit tests with genuine edge coverage: repeat-mode cycle order, play-next insertion index (after-current, append-on-none, clamp-at-end), drag→move index math in both directions with the post-removal coordinate subtlety, same-slot no-op, out-of-range clamping both sides, duration formatting (m:ss / h:mm:ss) with honest null for unknown/NaN/negative, source location rendering for file/url/null, and id lookup with -1 sentinel.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L162 `✓ ... (12 tests) 5ms` (inside 1705 passed).
- Edge math verified by reading: dragToMoveIndex(4,1,5)=3 (down shifts by one), (0,3,5)=0 (up keeps), (2,2,5)=2 (no-op), clamps 9→4 and -3→0.

## 3. Findings
None. Zero-findings checklist: tautology scan — every expect consumes real function output including clamps and nulls. Mock-of-subject scan — none (pure functions, no mocks at all). Skip scan — no it.skip; baseline 12 passed. Name-vs-body scan — verified. Mutation reasoning: off-by-one in move math fails L35/L38; wrong cycle order fails L14-18; NaN formatting fails L56-58.

## 4. Per-function table (<100 LOC; still listed)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cycleRepeat | 13-19 | REAL | — | full cycle |
| playNextTargetIndex | 21-31 | REAL | — | 3 edges |
| dragToMoveIndex | 33-47 | REAL | — | 4 cases incl. clamps |
| formatDuration/sourceLocation/indexOfId | 49-69 | REAL | — | formats + honest nulls + lookup |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 12 | YES | exact-value pure-function assertions; verified green baseline L162 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the catalog is never second-guessed here" | header L1-2 | REAL | tests assert helper semantics only |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
vitest; real queue module; no mocks, no DOM.

## 9. Open questions for the next agent
- None.
