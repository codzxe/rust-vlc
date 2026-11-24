# AUDIT: ui/src/screens/player/context.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 572 / 13 (usePlayerControls, usePlayerControlsOrNull, isToggleCommand, isStateProbeRefusal, bufferRangesOf, selectionFieldFor, PlayerControlsProvider, dispatch, applyCompleted, actions, onEvent, trackDisplayName, tracksOfKind, selectedTrackId) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — PlayerScreen.tsx:562 `<PlayerControlsProvider>` -> useClient (ClientProvider.tsx:19) -> main.tsx:61 `selectAppTransport()` -> TauriTransport over `lumen_dispatch` inside the desktop shell (app-transport.ts:67-69); DemoTransport fallback in plain browser |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg provider mounts/transport chain; read context.test.tsx (414 LOC, 11 behavioral cases); read wire-time.ts |

## 1. One-paragraph verdict
This is the central player state seam and it answers the master question
affirmatively for the player screen: every control action dispatches a real
1.D4 wire command through a real transport (TauriTransport in the shipped
desktop shell — verified call chain), state is populated only from engine
events and command completions, and the file is scrupulous about honest
unknowns (null = not reported, no optimistic values, no fabricated titles).
Units are converted at one edge (wire-time.ts, µs<->s) with the family fix
documented per field. Remaining nits are small races/policy holes, none
fabricated.

## 2. What actually works (verified)
- Call chain to the real backend: context.tsx:27 `useClient` -> ClientProvider.tsx:19 -> main.tsx:61 `<ClientProvider transport={appTransport.transport}>` -> app-transport.ts:58-77 selectAppTransport -> `new TauriTransport(backend)` with console notice "every command via lumen_dispatch, events on lumen://event" (rg, read).
- Behavioral test suite context.test.tsx (414 LOC): pendingToggle resolution on ERROR / dead transport / playback.error / non-toggle failure isolation, tracks.changed kind-guard (no stray `selections.data`), desktop cold-start probe exemptions with a regression pin that a failed USER toggle still surfaces. Tests use a scripted transport emitting real Event shapes — they would fail if the reducer logic broke.

## 3. Findings
F-001 [S4]
Where: context.tsx:315-317 + 181-200
Code:
```
          if (!isStateProbeRefusal(request.cmd, response.result.error)) {
            patch({ lastError: response.result.error });
          }
```
What is wrong: the refusal exemption keys on the COMMAND only, not on
"dispatched as a passive mount probe" — any future caller of
media.get-info/media.get-current-time inherits the swallow. Today the only
dispatchers of those cmds are the mount probes (context.tsx:526-530), so the
blast radius is zero now. Confidence: HIGH — Verification: read + rg.
F-002 [S4]
Where: context.tsx:418-428
Code:
```
        case "session.state-changed":
          setState((prev) => ({
            ...prev,
            playbackState: event.to,
            pendingToggle: false,
```
What is wrong: unconditional pendingToggle clear — a rapid pause-issued-
before-play's-state-changed-arrives race loses the second spinner while the
second command is still outstanding. Sub-second UX race, no state corruption
(the later state-changed for the pause still lands). Confidence: HIGH —
Verification: read; race not executed (UNVERIFIED runtime).
F-003 [S4]
Where: context.tsx:378-388
Code:
```
      seek: (seconds: number) => {
        // Truthful pending: replace any outstanding request (doc 05 §4 — a
        // new input cancels the previous one client-side). The seam stays
        // seconds; the wire target is µs (w26.k17 family fix).
        patch({ pendingSeek: seconds });
```
What is wrong: pendingSeek is not cleared on session.state-changed to
idle/error — if the engine stops mid-seek without answering
seek-completed/seek-failed the flag can persist into the next session's
first render (cleared on the first time-tick within 0.75s or next seek).
Engine-side stop-vs-seek ordering UNVERIFIED. Confidence: MEDIUM —
Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlayerState/Actions interfaces | 43-124 | REAL | - | honest-null contract documented per field |
| usePlayerControls | 128-134 | REAL | - | throwing reader |
| usePlayerControlsOrNull | 143-145 | REAL | - | used by useChromeAutoHide legacy fallback (comment; consumer file audited by another agent) |
| isToggleCommand | 158-159 | REAL | - | |
| isStateProbeRefusal | 181-200 | REAL | S4 | see F-001 |
| bufferRangesOf | 202-205 | REAL | - | µs->s at read edge |
| PlayerControlsProvider | 215-534 | REAL | - | |
| — applyCompleted | 233-298 | REAL | - | per-result-kind patches; µs crossings documented |
| — dispatch | 301-354 | REAL | - | errors surfaced, transport-failure typed error, toggle pending cleanup on all failure paths |
| — actions | 361-412 | REAL | - | see F-002/F-003 |
| — onEvent | 416-521 | REAL | - | event-sourced state; tracks.changed kind guard correct (482-488) |
| — mount probes | 526-530 | REAL | - | snapshot + time + info |
| trackDisplayName | 537-548 | REAL | - | honest "Unknown N", caller localizes |
| tracksOfKind | 551-556 | REAL | - | |
| selectedTrackId | 563-572 | REAL | - | two truth sources ordered, null honest |

## 5. Test honesty
context.test.tsx: 11 cases over a scripted transport. Mutation reasoning:
deleting the pendingToggle clear in dispatch's error branch fails the
"toggle that answers ERROR clears the pending flag" case; deleting the
tracks.changed guard fails the "no stray selection key" case. Honest suite.
PENDING-seek cases are tested elsewhere (PlayerScreen/seek consumers) —
UNVERIFIED here which file covers seek-completed clearing.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "it carries ONLY engine-reported state ... never optimistic/predicted values" | context.tsx:5-7 | REAL | read of every patch site: all patches come from command results/events or explicit pending fields |
| "Inside the Lumen desktop shell ... the app runs on the REAL TauriTransport" | main.tsx:13-17 | REAL | app-transport.ts:58-77 + tauri-transport.ts exist; runtime execution UNVERIFIED in sandbox (no Tauri host) |
| "µs wire / seconds seam, one conversion edge" | wire-time.ts:20-37 | REAL | all conversions in context.tsx call wire-time helpers (rg verified usToSeconds/secondsToUs/durationToSeconds usage) |

## 7. Dead / duplicated / unreachable code
None. All exports consumed (usePlayerControlsOrNull by useChromeAutoHide per
comment — flagged for that file's auditor to confirm).

## 8. Dependencies & flags
React hooks, client/transport types, client/wire-time. No feature gates.
No Tauri imports (host-agnostic seam — transports injected).

## 9. Open questions for the next agent
- Confirm useChromeAutoHide actually uses usePlayerControlsOrNull (claim in
  lines 136-141).
- Engine behavior on stop-during-seek (F-003) — needs an engine-side audit
  (crates/player or commands dispatch) to settle whether seek-failed always
  follows a stop.
