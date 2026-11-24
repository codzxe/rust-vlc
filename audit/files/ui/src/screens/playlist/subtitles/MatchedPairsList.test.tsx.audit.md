# AUDIT: ui/src/screens/playlist/subtitles/MatchedPairsList.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 46 / 4 `it` blocks |
| File verdict | REAL — executed, 4/4 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite (baseline ui-test.txt:168) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
Small, honest presentation-component test: labelled list with given order,
frame anatomy per row (2 icons + 1 leader + 2 names — exact counts),
honest empty state ("No subtitle files found" with NO list rendered), and
the accessible-name override. No mocks (pure props component).

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:168 (verbatim):
```
 ✓ src/screens/playlist/subtitles/MatchedPairsList.test.tsx (4 tests) 121ms
```
- Anatomy pin (29-33):
```
    expect(row!.querySelectorAll(".lumen-plsubs-pair__icon")).toHaveLength(2);
    expect(row!.querySelectorAll(".lumen-plsubs-pair__leader")).toHaveLength(1);
    expect(row!.querySelectorAll(".lumen-plsubs-pair__name")).toHaveLength(2);
```
- Honest empty state (36-40): empty copy present AND `queryByRole("list")` null.

## 3. Findings
None. Zero-findings justification: exact counts, order-sensitive content
checks, negative query on the empty branch; executed green.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| ordered list test | 17-25 | REAL | items[0] content pinned |
| anatomy test | 27-34 | REAL | exact element counts |
| empty state test | 36-40 | REAL | negative list query |
| label override | 42-45 | REAL | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 4 | YES | exact counts + content + negative queries |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "natural order passthrough and the honest empty state" | header :2-3 | REAL (tested) | 17-25, 36-40 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
vitest, @testing-library/react. No gates.

## 9. Open questions
None.
