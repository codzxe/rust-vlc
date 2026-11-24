# AUDIT: ui/src/screens/player/dialogs/naturalSort.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 62 / 2 (naturalCompare, compareBookmarks) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — BookmarksDialog.tsx (sorts real engine bookmarks); naturalCompare re-exported via components/list (natural.ts, List.test.tsx) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers; read; 7 test cases in naturalSort.test.ts |

## 1. One-paragraph verdict
Correct digit-chunk-aware comparator with the right edge behavior:
numeric runs compare by stripped-length then byte-wise (no Number() —
arbitrarily long digit runs exact), "02"=="2" numerically, prefix rule puts
the exhausted string first, and the bookmark ordering is time-numeric ->
natural label -> creation id (stable). No findings.

## 2. What actually works (verified)
- Edge analysis (read + hand-executed reasoning): "Chapter 10" vs "Chapter 2"
  -> digit runs "10" vs "2", length 2>1 -> 10 later. Correct. "10:00" vs
  "2:00" -> "10" vs "2" -> later. Correct.
- Equal numeric runs with differing zero padding ("t02" vs "t2") advance both
  cursors — no false inequality.
- naturalSort.test.ts: 7 cases (unit-tested per header claim).

## 3. Findings
None. Checked: overflow (none — string length compare), locale claims
(case-sensitive byte compare documented deliberately), stability (id
tie-break), subtraction overflow in compareBookmarks (a.time-b.time on µs
Timestamps — safe for any real bookmark; >2^53 µs = 285 years).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| naturalCompare | 20-49 | REAL | exact for long digit runs |
| compareBookmarks | 56-62 | REAL | 3-key stable order |

## 5. Test honesty
7 cases would catch the classic natural-sort bugs (leading zeros, prefix,
"10"<"2"); mutation reasoning: swapping the length compare sign fails them.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "fully unit-tested here (naturalSort.test.ts)" | naturalSort.ts:9 | REAL | 7 cases found |

## 7. Dead / duplicated / unreachable code
None (naturalCompare has a second consumer in list/natural.ts — not dead).

## 8. Dependencies & flags
Generated Bookmark type. None.

## 9. Open questions for the next agent
- None.
