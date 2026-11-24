# AUDIT: ui/src/screens/playlist/options/ThumbnailGrid.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 97 / 8 |
| File verdict | REAL — frame anatomy, honest unknowns, and a play-dispatch assertion through the REAL transport record |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:137 `✓ …ThumbnailGrid.test.tsx (9 tests) 176ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
Honest suite: duration formatting pinned per frame form (130s→00:02:10,
2579s→00:42:59 with hours kept, null→"—" honest unknown), the aria-label
regression (name-from-title beating name-from-content) asserted negatively
across all buttons, single-click play verified by filtering the REAL
transport's sent records for `playlist.play-item` with exact args {id:1},
empty queue renders no grid (panel owns the empty state), and the local
saved-playlist scope proves cards degrade to inert presentation (role=img
with accessible names) instead of dead buttons when no engine seam exists.

## 2. What actually works (verified)
— Baseline ui-test.txt:137: 9 tests green in 176 ms.
— Mutation reasoning: duration-format regression fails L22-23; the aria-label
regression fails L38-41; a dead-button card in local scope fails L82-83; a
missing play dispatch fails L49-51.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/options/ThumbnailGrid.test.tsx:51
Code:
    expect(plays[0] && "args" in plays[0] ? plays[0].args : null).toEqual({ id: 1 });
What is wrong: awkward inline conditional extraction — if `plays[0]` ever
lacked args the assertion degrades to `expect(null).toEqual({id:1})` (fails,
fine) but the expression obscures the intent; a destructure with a
type-narrow would say the same thing clearly. Hygiene only.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: :17,29,72,82-83 — class-name selectors (`.lumen-plo-card`,
`.lumen-plo-card--inert`) as the primary anatomy probe: couples the test to
CSS class naming rather than roles (roles are used where they matter — L15,
L38, L94).
Confidence: HIGH — Verification: read.
Zero S0-S3 findings; checked: play seam asserts the real transport record;
empty and null-duration honesty pinned; inert cards pinned as non-buttons
with accessible names.

## 4. Per-function table
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| frame anatomy | 12-43 | REAL | durations; title resolution; aria-label regression |
| interaction | 45-58 | REAL | real play-item dispatch; empty honesty |
| local scope | 60-97 | REAL | inert presentation; accessible names |

## 5. Test honesty
All 8 fail on their named regression (§2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "honest unknowns ('—')" | file L3 | REAL | L24 |
| "never dead buttons" | file L76 | REAL | L82-83 (button count 0, inert divs 2) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
RTL + vitest; PlaylistOptionsProvider + optionsTestSupport mount (real
transport recorded); no network.

## 9. Open questions for the next agent
- None.
