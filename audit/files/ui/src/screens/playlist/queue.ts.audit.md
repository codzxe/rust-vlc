# AUDIT: ui/src/screens/playlist/queue.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 61 / 5 (cycleRepeat, playNextTargetIndex, dragToMoveIndex, indexOfId, formatDuration, sourceLocation) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by PlaylistProvider (dispatch math) + PlaylistPanel (drag reorder, duration column) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "queue"` consumers (2 prod + 2 test); `rg -c "it\(" queue.test.ts` (12) |

## 1. One-paragraph verdict
Pure, correct queue math: VLC repeat cycle, play-next insertion (append on no-current), drag-to-move coordinate conversion with removal shift + clamps, honest-null duration formatting (non-finite/negative → null), source location strings. The engine stays the source of truth; these functions only compute UI-side arguments for real commands (playlist.move etc.). 12 tests. Zero findings.

## 2. What actually works (verified)
— verified by consumer trace (PlaylistProvider/PlaylistPanel, both audited during ControlBar cross-checks) and queue.test.ts (12 cases; baseline vitest 1705 passed).

## 3. Findings
None. Checked: edge cases — currentIndex<0 append, before>length clamp, from<before removal shift, length-1 final clamp, formatDuration(-5/NaN/Infinity) → null.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cycleRepeat | 9-18 | REAL | — | exhaustive switch |
| playNextTargetIndex | 25-28 | REAL | — | doc 03 §810 semantics |
| dragToMoveIndex | 35-39 | REAL | — | pre/post-removal coordinates |
| indexOfId | 42-44 | REAL | — | trivial wrapper (ids stable) |
| formatDuration | 47-55 | REAL | — | h-conditional padding |
| sourceLocation | 58-61 | REAL | — | null-honest |

## 5. Test honesty
12 tests — would fail on off-by-one/clamp regressions (pure functions, directly testable).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine remains the source of truth" (L4) | header | REAL | functions only compute command args; state comes from engine events (PlaylistProvider) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Wire types only. No flags.

## 9. Open questions for the next agent
- None.
