# AUDIT: ui/src/screens/playlist/subtitles/matching.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 128 / 12 |
| File verdict | REAL — pure-logic unit tests of the five matching strategies, normalization, and pairing |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:135 `✓ …matching.test.ts (14 tests) 7ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
Clean pure-function tests, no DOM, no mocks: extension recognition
(case-insensitive, non-subtitle rejection), stem/extension split, separator
normalization, release-tag tail cutting, all five strategies with their
negative cases (partial-title NOT matching "extra", case-normalized exact,
"off" never matching, degenerate no-op in "any"), and the pairing rules —
natural order on both sides, first-match determinism, candidate
non-reuse, non-subtitle filtering, honest empty results. Every strategy
branch has both a positive and a negative assertion.

## 2. What actually works (verified)
— Baseline ui-test.txt:135: 14 tests green in 7 ms.
— Mutation reasoning: any wrong strategy verdict (e.g. "extra" accepting a
partial title) fails L72-73; natural-order regression (E10 before E2) fails
L95-99/L109; candidate reuse regression fails L112-121.
— Zero findings checklist (spec §12): checked every `it` for positive+negative
assertions (all 12 have at least one negative or boundary case except
"pins the frame option order" L48-51, which pins constants — checked the
constants are the real exported values via the import list L6-15); checked
for mocks (none — pure imports); checked for tautologies (none); checked
test names vs bodies (all match).

## 3. Findings
None (see §2 checklist).

## 4. Per-function table
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| extensions + normalization | 19-45 | REAL | 4 tests, negative cases included |
| five strategies | 47-80 | REAL | mode order + default pinned; per-mode ± cases |
| pairSubtitles | 82-127 | REAL | natural order, no-reuse, filtering, honest empties |

## 5. Test honesty
All 12 fail on their named regression (§2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "deterministic first-candidate pairing" | file L3-4 | REAL | L100 exact first pair; L109 single-match |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Pure imports from ./matching only; no DOM/no network/no timers.

## 9. Open questions for the next agent
- Does isSubtitleFile cover ".sub"/".idx"/".ass"? (Only SRT/VTT asserted
here — the full extension set lives in matching.ts, another agent's file.)
