# AUDIT: ui/src/screens/player/states/StateRouter.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 76 / 2 (resolveOverlay, StateRouter) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by PlayerScreen.tsx (rg) -> App routes -> main.tsx; fed by states/context-bridge engine view |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln "StateRouter\|resolveOverlay"` consumers; StateRouter.test.tsx case scan; baseline transcript correlation check (audit/runs/baseline/cargo-test-workspace.txt:2775 event stream) |

## 1. One-paragraph verdict
Pure, honest state mapping: engine-truth view -> exactly zero-or-one overlay,
never optimistic UI. The mapping matches the real engine event stream recorded
in the orchestrator baseline (`session.state-changed` Idle->Opening->Buffering-
>Playing, cargo-test-workspace.txt:2775) — the states this switch handles are
the states the engine actually emits. Error handling distinguishes typed
`unsupported` codes from generic errors with no invented payloads; progress
overlays are determinate only when the engine reported a number (test-pinned
"no fake number" case).

## 2. What actually works (verified)
— Code read: total switch over the 8-state union (idle/opening/buffering/
error/playing/paused/stopped/ended); `data-state-overlay` marker; onRetry/
onOpenMedia passed through to affordances. — Tests: full state matrix
(StateRouter.test.tsx:69 table-driven), unsupported-vs-generic error split
(L83, L88), determinate-vs-indeterminate progress honesty (L97-121),
aria-busy (L129). Not run here (no node_modules; baseline vitest 1705 passed
cited). — Engine correlation: the baseline open-transcript's event kinds
(session.state-changed / session.buffering with progress) are exactly the
inputs context-bridge folds into PlayerEngineView.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/states/StateRouter.tsx:70
Code:
```
      return <UnsupportedOverlay error={view.error as NonNullable<typeof view.error>} onOpenMedia={onOpenMedia} />;
```
What is wrong: type-cast papers over the invariant (kind === "unsupported"
implies view.error !== null only via resolveOverlay's L38 logic). Safe today,
fragile if the resolver changes. A NonNullable-typed helper would remove the
cast.
Confidence: HIGH — Verification: read.

Zero S0-S3 findings. Checked and how: optimistic-UI risk (none — view comes
from context-bridge engine events only, header contract); state-machine
completeness (all 8 states handled, table-tested); fake payloads (none —
progress honesty test-pinned); dead branches (none).

## 4. Per-function table
N/A (<100 LOC — compact audit). resolveOverlay (L28-45) and StateRouter
(L60-76) both REAL; OverlayKind/StateRouterProps types consumed.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| state matrix (L69) | YES | table-driven, every state -> expected overlay |
| "loading without engine progress is indeterminate — no aria-valuenow, no fake number" | YES | absence assertion (L106) |
| unsupported-for-host split (L88) | YES | typed-code routing |
Not run here; baseline cited.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine view comes from context-bridge ... never from optimistic UI state" | StateRouter.tsx:2-4 | REAL | view prop from usePlayerEngineView (context-bridge); no local state in this file |
| "Exactly zero or one overlay is ever mounted" | StateRouter.tsx:57-58 | REAL | single switch return (L62-75) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
states/context-bridge (PlayerEngineView), error-display (isUnsupportedError),
StageOverlays (5 overlays). No feature gates; no direct transport import —
correctly consumes the engine view seam.

## 9. Open questions for the next agent
- context-bridge.ts + StageOverlays.tsx: confirm the view folding (events ->
  PlayerEngineView) and the overlay copy are equally honest (not my files).
