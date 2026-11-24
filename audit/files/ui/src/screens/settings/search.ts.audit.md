# AUDIT: ui/src/screens/settings/search.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 51 / 2 (normalize, searchSettings) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by SettingsScreen.tsx (the settings search field) -> routed settings screen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "searchSettings"` (1 consumer); search.test.ts case scan (5 behavioral cases) |

## 1. One-paragraph verdict
Honest cross-area settings search: schema entries only, feature-gated entries
excluded, empty/unresolved labels degrade to no-hit (never a crash, never an
invented match). The header is notable for documenting the DELETION of a
~40-line in-area filter API that could never execute (results replace the
area column) — dead code removed rather than left simulating a feature, with
the honest limits (hand rows/actions invisible to search; query clears on
navigation) written down instead of faked. Zero findings.

## 2. What actually works (verified)
— Code read: normalize (trim+lowercase, undefined-safe), featureExposed gate
(L44), schema-order preservation (L42-49), empty query -> [] (L40). — Tests
(search.test.ts): empty query, case-insensitive substring, cross-area order,
feature-gate hiding, honest no-match — all behavioral. Not run here (no
node_modules; baseline vitest 1705 passed cited).

## 3. Findings
None. Checked and how: invented results (none — subset filter only); crash
paths (labelOf failures degrade to empty via normalize); dead code (none —
the previously-dead in-area API was deleted, documented at L10-17); reachability
(SettingsScreen consumer verified).

## 4. Per-function table
N/A (<100 LOC — compact audit). searchSettings (L34-51) REAL; normalize
(L30-31) REAL; SearchHit type consumed by SettingsScreen.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "hides feature-gated unexposed entries from results" | YES | gated entry excluded assertion (L49) |
| "matches across areas and keeps schema order" | YES | ordered hit list assertion (L40) |
| "reports an honest no-match for gibberish" | YES | empty result assertion (L55) |
Not run here; baseline cited.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the result is the honest subset of schema entries whose labels match; nothing is invented" | search.ts:3-5 | REAL | filter-only implementation (L44) |
| "Labels that the catalog has not resolved yet ... can never match — a catalog gap degrades to 'no hit', never a crash" | search.ts:5-8 | REAL | normalize(undefined) -> "" (L30-31) |
| "hand rows ... are actions, not [[setting]] rows, and stay invisible to search" | search.ts:18-20 | REAL | schema-entries-only walk (L42-44) |

## 7. Dead / duplicated / unreachable code
None. (The in-area filter API it deleted is documented at L10-17 — good
hygiene, contrast with the staged surfaces elsewhere.)

## 8. Dependencies & flags
generated/schema types only. Pure function; no transport, no feature gates.

## 9. Open questions for the next agent
- SettingsScreen: confirm the search-results-replace-column behavior and the
  query-clear-on-navigation claim (j12 P1-1) — screen audit's job.
