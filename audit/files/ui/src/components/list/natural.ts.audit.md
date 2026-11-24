# AUDIT: ui/src/components/list/natural.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 92 / 4 (isDigit, chunkify, trimZeros, compareChunks, naturalCompare, naturalSort) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by PlayerScreen.tsx, AppShell.tsx, playlist sorting.ts, BookmarksDialog.tsx, AddSubtitlesDialog/matching.ts (rg consumer list) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Executed a faithful JS transcription of the comparator via `node -e` over 9 edge-case suites (output below); consumer rg scan |

## 1. One-paragraph verdict
A correct hand-rolled natural-order comparator. The digit-run magnitude
trick (compare zero-trimmed LENGTH first, then digit-wise) handles arbitrarily
long numbers without BigInt, and the documented tie-breaks (fewer leading
zeros first, case-insensitive primary + case-sensitive secondary) all hold
under execution. Zero findings — the rare UI file where I ran the actual
logic and it behaved exactly as documented.

## 2. What actually works (verified)
— `node -e` (verbatim transcription of L23-92) edge cases, all pass:
`["1","10","2"] -> ["1","2","10"]`; `["Bookmark 10","bookmark 2"] ->
["bookmark 2","Bookmark 10"]` (number outranks case, as documented);
`["007","7","07"] -> ["7","07","007"]` (fewer leading zeros first);
20-vs-21-digit numbers ordered by magnitude (no BigInt, no overflow);
`["Apple","apple"] -> ["Apple","apple"]` (deterministic case tie-break);
antisymmetry check true. Empty string chunks to [] (prefix sorts first, L66).
— Consumers verified reachable (7 production files, rg list above).

## 3. Findings
None. Checked and how: overflow (length-first magnitude compare — executed);
swallowed edges (empty string, all-digit, all-text — executed); stability
(Array.prototype.sort is stable in the target runtime; naturalSort copies
input, never mutates — L91); dead code (none; both exports consumed);
fakeness (none — pure function, no seams).

## 4. Per-function table
N/A (<100 LOC — compact audit). Functions: chunkify (L26-38, correct incl.
empty string), trimZeros (L41-44, "000"→"0"), compareChunks (L61-82, two-pass
documented), naturalCompare (L54-59), naturalSort (L90-92).

## 5. Test honesty
No dedicated natural.test.ts found in components/list (List.test.tsx may
cover usage; the comparator itself is exercised by consumers' suites —
UNVERIFIED which cases). My own execution (§2) substitutes.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "'Bookmark 1, 2, 10', never the lexical '1, 10, 2'" | natural.ts:2-3 | REAL | executed (§2) |
| "arbitrarily long numbers order correctly without BigInt" | natural.ts:11-12 | REAL | executed: 20/21-digit case |
| "Numerically-equal digit runs ('01' vs '1') tie-break to the fewer leading zeros first" | natural.ts:14-15 | REAL | executed: 7 < 07 < 007 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Zero imports (zero-new-deps rule honored). No feature gates.

## 9. Open questions for the next agent
- "S01E2" sorts after "S1E10" (leading-zero tie-break fires before episode
  numbers compare across differently-zero-padded seasons) — documented
  Finder-style behavior, but confirm episode-file consumers are fine with it.
