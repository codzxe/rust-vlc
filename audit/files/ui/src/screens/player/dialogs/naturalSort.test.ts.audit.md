# AUDIT: ui/src/screens/player/dialogs/naturalSort.test.ts
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 62 / 1 helper + 2 describe blocks (7 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:165 `✓ src/screens/player/dialogs/naturalSort.test.ts (7 tests) 4ms`; subject naturalSort.ts (62 L) ships in the bookmarks dialog row order |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `grep -n naturalSort audit/runs/baseline/ui-test.txt` -> :165; `wc -l` subject -> 62 L; read-through of all 7 tests |

## 1. One-paragraph verdict
Small, sharp comparator suite: digit chunks compare numerically (1 < 2 < 10, never lexical), plain strings byte-wise with 0 for equals, prefix-first, leading zeros fold to numeric value ("Track 02" == "Track 2"), empty string first; and the bookmark row order pins time-ascending numerically (µs wire values: 10,000,000 < 120,000,000 — the ms-vs-µs trap is exercised), equal-times natural label order with unnamed first, and creation-id final tie-break. Every ordering claim is asymmetric (asserted both directions where it matters).

## 2. What actually works (verified)
— verified by baseline ui-test.txt:165 (7/7 pass; lane 1705 passed :178-179). Mutation reasoning: a lexical comparator fails :9-12; a stringly time compare fails :42 (2:00 vs 0:10); a missing zero-fold fails :22; an unstable tie-break fails :56-58.

## 3. Findings
None. Checked for: tautologies (none — directional comparisons on distinct values), unwired tests (7/7 run), scope honesty (the two exported functions are both covered). Zero findings is the honest result.

## 4. Per-function table
File <100 LOC — naturalCompare suite (:7-30, 4 tests) and compareBookmarks suite (:32-60, 3 tests), all REAL.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| numeric chunks :8-13 | YES | directional asserts on the exact lexical trap |
| time ordering :39-44 | YES | wire-unit µs values |
| tie-breaks :46-59 | YES | label order + id order both pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 7 tests run | baseline ui-test.txt:165 | REAL | `✓ ... (7 tests) 4ms` |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Bookmark type from the real generated commands module.

## 9. Open questions for the next agent
None.
