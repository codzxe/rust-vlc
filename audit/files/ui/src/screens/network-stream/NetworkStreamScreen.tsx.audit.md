# AUDIT: ui/src/screens/network-stream/NetworkStreamScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 312 / 6 (isValidNetworkUrl, loadRecentStreams, saveRecentStreams, NetworkStreamScreen, exit, openStream) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (App.tsx route /open-network-stream:226; mounted from the app menu / open-media funnel). ENGINE LANE: session.open is a REAL catalog command — demo transport serves it (demo-transport.ts:201,408), CLI host serves it (baseline mp4-smoke open sessions). SHIPPED DESKTOP BINARY: NO (tauri unbuildable — baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg "session.open" demo-transport.ts:201,408; rg route App.tsx:226; test survey NetworkStreamScreen.test.tsx:46-179; sed openStream body; read Dialog/focus flow |

## 1. One-paragraph verdict
A genuinely wired screen — unlike the tools panels, this one dispatches a
real `session.open` with a url source through the client transport, persists
a real recents list in localStorage, and creates a real local playlist when
named. Tests are substantive (validation, submit→dispatch→route, playlist
record, recents refill) and passed in baseline (ui-vitest.log.txt). The
defects are in the open result handling: the command's response is thrown
away entirely (only .catch/.finally run), so a synchronous typed refusal
navigates the user to the player as if nothing happened, and every dismissal
hardcodes /player regardless of origin route.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): NetworkStreamScreen
tests execute frame anatomy, first-focus-on-field (not ✕), Escape/scrim
exit, invalid-URL rejection with zero sends, submit → session.open url
source → /player, named open → local playlist record, recents persistence
and refill.
— verified by read: isValidNetworkUrl is pure and total (http/https only,
honest about rtsp/ftp being a catalog question); loadRecentStreams validates
shape (array of strings, capped at 6) and never crashes on corrupt stores.
— verified by rg: demo-transport serves session.open (408) and emits
opening-progress events (756) — the happy lane is real in the dev shell.

## 3. Findings
### F-001 [S3] session.open's response is discarded — a synchronous typed refusal navigates to /player as success
Where: ui/src/screens/network-stream/NetworkStreamScreen.tsx:147-163
Code:
```
    void client
      .send({
        cmd: "session.open",
        requestId,
        args: { source: { kind: "url", url: target } },
      })
      .catch(() => {
        // Transport unreachable — the player surfaces its own honest error.
      })
```
What is wrong: The resolved response is never inspected — no
`response.result` read, no error branch. Two swallow paths: (a) transport
death → .catch is empty, justified only by the claim that the player screen
surfaces its own error (true: fresh /player mount probes fail and set
lastError); (b) the engine ANSWERS `error` without emitting events — for a
client-validated http(s) url this is a narrow path (host rejects the source
shape, capability gap), but when it happens the user is navigated to the
player which sits idle with no error anywhere: the failure is invisible.
Why it matters: Swallowed-error pattern; the async failure lane (engine
events → error overlay) works, but the sync refusal lane is a silent black
hole.
What correct looks like: Inspect the response like every other dispatch site
(context.tsx dispatch:305-323): on `"error" in response.result` with no
event-emitting code, surface it in THIS dialog (the role="alert" lane
already exists) instead of navigating.
Confidence: MEDIUM — Verification: read; which error codes the host answers
without events for url sources not traced end-to-end (desktop binary
unbuildable).
### F-002 [S3] Every dismissal hardcodes /player — the origin route is destroyed
Where: ui/src/screens/network-stream/NetworkStreamScreen.tsx:112-114
Code:
```
  const exit = (): void => {
    navigate("/player");
  };
```
What is wrong: Escape, scrim click, ✕ and Cancel all navigate to /player
regardless of the route the user came from (the dialog is reachable from the
app menu on any shell route).
Why it matters: Same defect family as SettingsScreen.tsx F-002 (my audit):
open from /playlist or /convert, cancel, land on /player — the "dialog"
metaphor eats the history.
What correct looks like: `navigate(-1)` with /player fallback.
Confidence: HIGH — Verification: read; tests 98-111 pin the current behavior
as contract.
### F-003 [S4] Dead supersede branch — the requestId guard can never fire
Where: ui/src/screens/network-stream/NetworkStreamScreen.tsx:156-160
Code:
```
      .finally(() => {
        // A newer submission superseded this one — it owns the exit (j16
        // P3-9: the counter used to increment without anyone checking, so a
        // late earlier response could clobber the newer one's navigation).
        if (requestId !== nextRequestId.current) return;
```
What is wrong: The double-send guard `if (opening) return;` (line 129) plus
the loading-disabled submit button make a second concurrent send impossible,
so `requestId !== nextRequestId.current` is unreachable — the j16 P3-9 fix
belt-and-suspendersed itself into dead code.
Why it matters: Reader cost + a false sense that superseded sends occur;
the comment describes a hazard the guard above already eliminates.
What correct looks like: Either drop the opening-guard and rely on the
counter (one mechanism), or drop the counter check.
Confidence: HIGH — Verification: read; all state writers of `opening`
traced (only openStream sets true; only this finally sets false).
### F-004 [S4] Recents are recorded before the open outcome is known
Where: ui/src/screens/network-stream/NetworkStreamScreen.tsx:135-137
Code:
```
    const next = [target, ...recents.filter((entry) => entry !== target)].slice(0, RECENT_LIMIT);
    setRecents(next);
    saveRecentStreams(next);
```
What is wrong: The URL enters the persisted history at submit time — a URL
the engine later fails to open (async error) still shows as a "recent
stream" forever.
Why it matters: Minor honesty drift in the affordance ("recent streams"
implies played); VLC's history behaves similarly, so blast radius is small.
What correct looks like: Record on the first engine event confirming the
session (e.g. tracks-ready), or accept and document the submit-time semantic.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| isValidNetworkUrl | 62-69 | REAL | — | pure, total, tested (46-54) |
| loadRecentStreams | 71-81 | REAL | — | shape-validated, capped, crash-safe |
| saveRecentStreams | 83-89 | REAL | — | quota-failure tolerated (comment honest) |
| NetworkStreamScreen | 91-312 | REAL-UNPROVEN | S3 (F-001/F-002) | executes in tests; frame-faithful dialog |
| exit | 112-114 | REAL | S3 (F-002) | hardcoded /player |
| openStream | 126-164 | REAL | S3/S4 (F-001/F-003/F-004) | dispatches real command; response discarded |
| focus effect | 122-124 | REAL | — | overrides Dialog trap's first-focusable; runs after child effect — correct order documented |

## 5. Test honesty (NetworkStreamScreen.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| isValidNetworkUrl accepts/rejects (47-54) | YES | pure unit, enumerates protocols |
| frame anatomy (67-85) | YES | asserts dialog + fields + actions |
| initial focus on URL field (87-97) | YES | the h19 shame #4 regression pin |
| invalid URL: error + zero sends (112-122) | YES | transport spy asserts no send |
| submit → session.open url source → route (123-138) | YES | asserts command args and navigation |
| named open → playlist record (139-159) | YES | asserts playlistsStore write |
| Cancel / Escape / scrim exit (98-111, 160-166) | YES | asserts no send + navigation |
| recents render + refill + empty (167-185) | YES | localStorage roundtrip |
Mutation check: delete the `.finally` navigation → test 123 fails. The
response-swallow path (F-001) has NO test — no test simulates an error
RESPONSE (only transport death is exercised elsewhere in the suite family).
Minor gap: no test asserts the error clears while typing (line 216-218).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the open wiring is the untouched pre-W26 contract — session.open with the ONE url source" | header 25-27 | REAL | code + test 123-138 |
| "the player surfaces its own honest error" (transport death) | line 154 | REAL | /player mount probes fail → lastError (context.tsx:336-351) |
| (implied) engine-refused opens are visible to the user | — | SHALLOW | F-001: sync-refusal lane swallowed |
| "The frame's 'multiple links' promise still awaits an engine catalog command" | header 27-29 | REAL | doc 05 §2.9 has no multi-open; single-URL validation honest |

## 7. Dead / duplicated / unreachable code
- F-003 dead supersede branch.
- `nextRequestId` counter duplicates the per-component pattern from
  context.tsx / config-settings.ts (S4 family; request_id is echo-only —
  dispatch.rs:63 — so harmless, see my context.tsx audit F-003).

## 8. Dependencies & flags
react-router, components (Button/IconButton/Icon/Dialog/Separator),
form.css class grammar, client transport, playlistsStore, lane i18n. No
feature flags.

## 9. Open questions for the next agent
- Does the CLI host's session.open for url sources ever answer a synchronous
  `error` (no events)? If never, F-001 is theoretical; if yes, it is a real
  silent-failure lane. Host-side trace suggested (apps/cli dispatch route
  for session.open + url source validation).
- Should the recents list live in the settings-backed persistence seam
  (W11 found engine config volatile; localStorage here is at least real)?
