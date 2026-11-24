# AUDIT: ui/src/screens/player/states/context-bridge.ts
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 235 / 7 (clamp01, applyEvent, bridgeReducer, initBridgeState, snapshotStateOf, usePlayerEngineView + WeakMap cache) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (PlayerScreen.tsx:676-677 `const client = useClient(); const bridgeView = usePlayerEngineView(client)` -> StateRouter overlay rendering) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg usePlayerEngineView call sites; rg transport interface (subscribe/send); rg context.tsx duplication; sed quote verification |

## 1. One-paragraph verdict
This is the affirmative answer to the master question for the player screen:
the state overlays are driven by a REAL engine bridge. The reducer consumes
only engine events (`session.state-changed`, `session.opening-progress`,
`session.buffering`, `playback.error`), clamps progress to 0..1, renders null
progress as indeterminate (no fabricated percentages), restores truth across
route remounts via a mount-time `system.get-state-snapshot` probe with correct
precedence (live events always win; late/failing probes change nothing), and
caches the last view per transport in a WeakMap. Correctness reading found no
state-machine defect. Two S4s: it duplicates the canonical C2 context's
snapshot machinery (documented "do NOT grow this file" debt), and a documented
demo-transport lie (recentOpenings order) is patched at the call site rather
than fixed at the source.

## 2. What actually works (verified)
— Call chain: PlayerScreen.tsx:676 useClient() -> :677 usePlayerEngineView(client)
-> transport.subscribe (191-196) + snapshot probe (202-224).
— transport.ts:38-40 defines send/subscribe — the interface the bridge codes
against exists and is the same one the generated commands use.
— Reducer precedence logic (133-163): live-vs-snapshot race handled; same-state
seed is a no-op; error-entry keeps cached payload.
— WeakMap cache (69, 165-168, 228-232): per-transport, no cross-wire leakage.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/states/context-bridge.ts:3-13
Code:
```
// RECONCILIATION NOTE: card 4.C2 owns ui/src/screens/player/context.tsx (the
// canonical player session context). ... When C2's context lands, this bridge should be deleted and
// the states/ components pointed at the canonical context
```
What is wrong: C2's context.tsx exists and also runs `system.get-state-snapshot`
(context.tsx:191,279,527) — two parallel engine-state reducers now ship, the
planned deletion was superseded (w26.f10 note) instead of executed. Divergent
truth pipelines are a standing regression risk; both must be updated for every
new event kind.
Confidence: HIGH — Verification: read + rg context.tsx.
F-002 [S4]
Where: ui/src/screens/player/states/context-bridge.ts (consumers) — PlayerScreen.tsx:711-718
Code:
```
  // ORDERING NOTE (client-lane handoff): the demo transport's
  // `recentOpenings` getter DOCUMENTS "most recent first" but SHIPS
  // oldest-first (rememberOpening prepends newest; the getter reverses an
  // already-newest-first list). The most recent opening is therefore the
  // LAST element today; this seam takes `.at(-1)`
```
What is wrong: A getter whose documentation contradicts its behavior ships,
and the consumer hard-codes `.at(-1)` to compensate — the lie is patched
around, not fixed. Any new consumer reading recentOpenings per its doc
comment gets the wrong element.
Confidence: HIGH — Verification: read PlayerScreen.tsx:711-718 (cross-file
evidence; demo-transport.ts is another agent's file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| clamp01 | 78-80 | REAL | - | NaN would pass Math.min/max → NaN; engine contract says f32 0..=1 |
| applyEvent | 82-117 | REAL | - | state transitions clear transient evidence (C6) |
| bridgeReducer | 133-163 | REAL | - | live/snapshot precedence correct |
| initBridgeState | 165-168 | REAL | - | WeakMap seed |
| snapshotStateOf | 171-177 | REAL | - | shape-guarded |
| usePlayerEngineView | 185-235 | REAL | - | subscribe effect + probe effect + cache effect |

## 5. Test honesty
No tests in this file; StateRouter/PlayerScreen tests exercise the bridge
through rendering (owned by other agents). UNVERIFIED: I did not execute the
remount-race tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no timer-based or optimistic values are ever synthesized" | context-bridge.ts:18-19 | REAL | reducer only maps events; no timers/date calls in file |
| "probe failures ... leave the honest last-known view untouched" | 38-39 | REAL | catch {} at 216-218 discards only the seed |
| "A late snapshot response never overrides live events" | 37-38 | REAL | state.live guard at 145 |

## 7. Dead / duplicated / unreachable code
Duplication with player/context.tsx (F-001). No dead exports — index.ts:9
re-exports the hook; PlayerScreen + StateRouter consume.

## 8. Dependencies & flags
client/transport (CommandTransport), generated/commands (PlaybackState,
CommandError). No flags.

## 9. Open questions for the next agent
demo-transport.ts owner: fix recentOpenings to match its doc (or its doc to
match the code) and flip PlayerScreen.tsx:719 — the seam comment promises a
failing test will catch it.
