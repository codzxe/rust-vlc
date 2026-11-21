# AUDIT: ui/src/screens/player/context.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 572 / 15 (usePlayerControls, usePlayerControlsOrNull, isToggleCommand, isStateProbeRefusal, bufferRangesOf, selectionFieldFor, PlayerControlsProvider, applyCompleted, dispatch, requestId, actions-memo, onEvent-effect, probe-effect, trackDisplayName, tracksOfKind, selectedTrackId) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (main.tsx → App → PlayerScreen.tsx:560 mounts `<PlayerControlsProvider>`; `usePlayerControls` consumed by ControlBar/MiniPlayer/canvas menu). SHIPPED DESKTOP BINARY: NO — tauri host unbuildable at HEAD (baseline #2 exit 101; W14-A01 E0425 app.rs:301) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | rg "session.open" (callers: OpenMediaDialog.tsx:136, NetworkStreamScreen.tsx:149, open-path.ts); sed context.test.tsx test list; sed wire-time.ts:1-42; sed dispatch.rs:55-75 (request_id echo-only); rg "requestId" transports |

## 1. One-paragraph verdict
The player state seam is genuinely well-engineered: all state derives from
engine events or command completions, unit crossings (µs↔s) go through one
named module, toggle-pending resolution covers error/transport-death/async
failure (tested, context.test.tsx:165-310), and probe refusals are honestly
classified as states. It executes under the baseline vitest run (1705 passed,
audit/runs/baseline/ui-vitest.log.txt). The one doctrinal hole: on session
close only `mediaTitle` is cleared, while `durationSeconds`, `tracks`,
`selections`, `position`, `seekable`, `buffered`, `volume` survive into the
NEXT session — the file's own comment claims "stale truth never survives into
the next session (doc 05 §1 C6)" and the code implements that for exactly one
field. `pendingSeek` has the same leak via a different path.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/ui-vitest.log.txt (1705 passed):
context.test.tsx executes pendingToggle resolution (error answer, dead
transport, happy path, non-toggle isolation), tracks.changed kind guard
(data/secondary-audio no-op; three kinds land), playback.error resolution,
desktop cold-start probe exemptions, failed-user-toggle still surfacing.
— verified by read: every µs↔s crossing routes through wire-time.ts constants
(usToSeconds/durationToSeconds/secondsToUs — single factor 1_000_000); the
w26.k17 family fix is applied consistently at lines 254, 264, 273, 284-285,
384, 441, 456, 460.
— verified by sed apps/cli/src/dispatch.rs:55-75: `request_id` is echoed, not
correlated server-side → per-provider counters are safe.

## 3. Findings
### F-001 [S2] Session close clears ONLY mediaTitle — six session-scoped fields survive into the next session, contradicting the file's own doc-05 §1 C6 claim
Where: ui/src/screens/player/context.tsx:418-428
Code:
```
        case "session.state-changed":
          setState((prev) => ({
            ...prev,
            playbackState: event.to,
            pendingToggle: false,
            // A closed session (idle) owns no media title — stale truth
            // never survives into the next session (doc 05 §1 C6).
            mediaTitle: event.to === "idle" ? null : prev.mediaTitle,
```
What is wrong: When the engine returns to `idle`, `durationSeconds`,
`position`, `seekable`, `tracks`, `selections`, `buffered` keep the PREVIOUS
media's values. The next `session.open` (dispatched by
OpenMediaDialog.tsx:136 / NetworkStreamScreen.tsx:149) transitions
idle→opening→buffering — none of which reset them either — so until the new
media's `TimingDurationKnown`/`MediaInfoReady`/first tick arrive, the seekbar,
track menus and media-info readouts show media A's facts while media B is
opening.
Why it matters: Realistic sequence (stop file A, open file B): the UI asserts
A's duration/tracks for B during the opening window — exactly the "stale truth
surviving into the next session" the comment claims is impossible. A user can
even commit a seek against A's stale duration (seek is enabled while
`durationSeconds` non-null).
What correct looks like: On `event.to === "idle"`, reset the session-scoped
subset to INITIAL_STATE values (position 0, durationSeconds null, seekable
null, tracks [], selections null, buffered [], pendingSeek null) — mediaTitle
already does this.
Confidence: HIGH — Verification: read; no test in context.test.tsx covers
idle-reset of these fields (test list: 165-403 — pendingToggle, kind guard,
probes only).
### F-002 [S3] `pendingSeek` is never cleared on session end — a seek outstanding at stop/error sticks forever
Where: ui/src/screens/player/context.tsx:443-450
Code:
```
              position,
              pendingSeek:
                prev.pendingSeek !== null &&
                Math.abs(position - prev.pendingSeek) > SEEK_CONFIRM_TOLERANCE_SECONDS
                  ? prev.pendingSeek
                  : null,
```
What is wrong: pendingSeek clears only via seek-completed, seek-failed, or a
time-tick within 0.75 s of the target (this quote). If the session ends
(state-changed → idle/error) while a seek is outstanding, no further ticks
arrive and the engine never emits seek-completed/failed for a dead session —
`pendingSeek` stays set indefinitely; the seekbar keeps rendering the pending
marker (Seekbar receives `pending={state.pendingSeek ?? undefined}`) into the
next session until a coincidentally-near tick clears it.
Why it matters: The pendingToggle flag got exactly this treatment (cleared on
state-changed and playback.error, lines 422, 513) — pendingSeek was left
asymmetric; header line 10-11 documents the three clearing paths without
noting the stuck case.
What correct looks like: `case "session.state-changed"`: when `event.to` is
"idle" or "error", also `pendingSeek: null` (same patch as mediaTitle).
Confidence: MEDIUM — Verification: read; engine's stop-path event sequence
not traced end-to-end (would need a live host — desktop binary unbuildable,
baseline #2).
### F-003 [S4] Per-provider requestId counters start at 1 in every provider — duplicate ids on the wire
Where: ui/src/screens/player/context.tsx:356-359
Code:
```
  const requestId = (): number => {
    nextRequestId.current += 1;
    return nextRequestId.current;
  };
```
What is wrong: Every provider instance (PlayerControlsProvider,
MediaInfoProvider, …) has its own counter starting at 1, so the same wire
session sees request_id 1,2,3… from multiple components. Harmless today
(dispatch.rs echoes request_id without correlation, verified), but it defeats
log-based correlation ("request 3 failed" is ambiguous) and would silently
break any future host that correlates.
Why it matters: Latent protocol hygiene, zero user impact today.
What correct looks like: A module-level monotonic counter (or
`crypto.randomUUID()`-style id) shared by all senders.
Confidence: HIGH — Verification: read + sed dispatch.rs:55-75.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| usePlayerControls | 128-134 | REAL | — | throws without provider; consumed widely (verified: MiniPlayer, ControlBar via rg) |
| usePlayerControlsOrNull | 143-145 | REAL | — | nullable twin for useChromeAutoHide (caller verified by rg) |
| isToggleCommand | 158-159 | REAL | — | covers play/pause/toggle-play-pause; matches toggle-pending contract |
| isStateProbeRefusal | 181-200 | REAL | — | probe cmds vs user cmds correctly split; tested (context.test.tsx:313-403) |
| bufferRangesOf | 202-205 | REAL | — | µs→s at read edge, correct |
| selectionFieldFor | 207-209 | REAL | — | identity mapping; exists for typing only |
| PlayerControlsProvider | 215-534 | REAL-UNPROVEN | S2 (F-001) | runs in tests; idle-reset hole |
| applyCompleted | 233-298 | REAL | — | every result kind handled or deliberately ignored (applied-selector re-queries); unit fixes applied |
| dispatch | 301-354 | REAL | — | errors surfaced (not swallowed) except documented probe refusals; catch maps transport death to typed error |
| requestId | 356-359 | REAL | S4 (F-003) | per-instance counter |
| actions memo | 361-412 | REAL | — | 9 actions, all dispatch real commands; no optimistic state |
| onEvent effect | 415-521 | REAL | S2/S3 (F-001/F-002) | subscribe/unsubscribe correct; µs crossings correct; tracks.changed guard tested |
| probe effect | 526-530 | REAl | — | 3 mount probes; re-runs only on transport change (deps [dispatch]) |
| trackDisplayName | 537-548 | REAL | — | name → language → "Unknown N"; honest |
| tracksOfKind | 551-556 | REAL | — | trivial filter |
| selectedTrackId | 563-572 | REAL | — | selections truth first, TrackInfo.isSelected fallback, honest null |

## 5. Test honesty (context.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| toggle answers ERROR clears pending + surfaces error (166-180) | YES | asserts both pendingToggle false and lastError set |
| dead transport clears pending + not-ready (182-195) | YES | send() rejects; asserts typed not-ready error |
| non-toggle failure does not clear toggle (219-249) | YES | isolation pin against over-clearing |
| data/secondary-audio track change no-op (252-287) | YES | asserts no stray `selections.data` key (regression pin for old cast bug) |
| playback.error resolves pending (289-311) | YES | asserts spinner cleared + fatal → error state |
| desktop probes exempt / demo not-found exempt (313-396) | YES | asserts NO error surfaced for probe refusals, error STILL surfaced for user toggle |
Mutation check: delete `mediaTitle: event.to === "idle" ? null : …` → NO test
fails (no idle-reset test exists) — the F-001 gap is untested, the
mediaTitle-clearing line is itself unprotected. Delete the tracks.changed kind
guard → tests 252-287 fail. Good coverage except idle-reset.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "never optimistic/predicted values" (header line 6-7) | file header, doc 08 §5 | REAL | all patches traced to events/completions |
| "stale truth never survives into the next session (doc 05 §1 C6)" | lines 423-425 | FALSE for 6 fields | F-001: only mediaTitle is cleared |
| "pendingSeek … cleared on timing.seek-completed / seek-failed / position catching up" | lines 9-11 | SHALLOW | F-002: no path clears it on session end |
| "errors are surfaced, never swallowed (§5.4)" | line 300 | REAL (with documented probe exemption) | dispatch:315-317 + tests 313-403 |

## 7. Dead / duplicated / unreachable code
None found. All exports have consumers (rg: usePlayerControlsOrNull →
useChromeAutoHide; trackDisplayName/tracksOfKind/selectedTrackId →
SettingsPanel/canvas menu/state-mapping).

## 8. Dependencies & flags
Depends on client/{ClientProvider,wire-time,transport} and generated
commands — all in-tree. No feature flags. Note for cross-reference: W14-A01
found the UI never sends `session.close` (only demo-transport does) — this
provider's stop path relies on `playback.stop` + engine events, consistent
with that finding.

## 9. Open questions for the next agent
- Does the real engine (apps/cli host) emit `timing.seek-failed` when a
  session is stopped mid-seek? If yes, F-002 is unreachable in practice;
  static analysis says the idle path still leaks.
- Should `volume`/`muted` survive sessions? (Engine-global vs session-scoped
  is a product question; current code keeps them, which is defensible for
  audio settings — but then the doc-05 C6 comment over-claims.)
