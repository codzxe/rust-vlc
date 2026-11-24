# AUDIT: ui/src/screens/player/customize/cycleState.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 86 / 5 (deepEqual, CYCLE_PAYLOADS, applyCyclePayload, applyCycleCompletion, applyCycleEvent) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by toolbarItems.tsx:57,101-130 (real ControlBar) and CustomizePanels.tsx (customize box) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg wire kinds in generated/commands.ts — all five exist (applied-rate/applied-repeat/applied-zoom, playback.rate-changed, repeat-set) |

## 1. One-paragraph verdict
Shared engine-confirmed cycle-state mapping (never optimistic — selections
update only from command completions and events, matched by args equality
against the tile registry). All wire kinds it switches on exist in the
generated command/event unions (verified). Clean and small; no findings.

## 2. What actually works (verified)
- Wire-shape fidelity: "applied-rate"/"applied-repeat"/"applied-zoom"
  (CommandResult kinds), "playback.rate-changed" (Event), "repeat-set"
  (PlaylistChange kind) — each present exactly once in generated/commands.ts (rg).
- Args-equality matching via deepEqual handles the nested options payloads; a
  payload matching NO option is a no-op (line 50 guard) — honest.
- The j08 P2-3 fix (rate stuck at 1.25 by re-dispatching defaults) is
  structurally prevented: selections only advance from confirmed truth.

## 3. Findings
None. Checked: switch exhaustiveness (default no-op honest), deepEqual
recursion (no cycles possible in option args — plain JSON payloads), tile
lookup guard (48 double-checks kind), duplication (single source shared by
box + real bar).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| deepEqual | 18-27 | REAL | plain-object recursion |
| CYCLE_PAYLOADS | 30-37 | REAL | 3 cycle commands |
| applyCyclePayload | 40-53 | REAL | no-op on unmatched payload |
| applyCycleCompletion | 58-69 | REAL | first-match return |
| applyCycleEvent | 73-86 | REAL | rate + repeat-set events |

## 5. Test honesty
Covered via CustomizePanels.test.tsx:831 "the real bar follows engine rate
events (the box's subscription parity)" — an event-driven assertion that
would fail if the mapping broke.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "completions and events map back by args equality — never optimistic" | cycleState.ts:6-7 | REAL | read: only confirmed sources mutate selections |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Generated transport types + panels registry. None.

## 9. Open questions for the next agent
- None.
