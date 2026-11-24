# AUDIT: ui/src/screens/playlist/PlaylistProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 320 / 15 (PlaylistProvider + 13 inner fns/effects, usePlaylist, usePlaylistOptional) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — mounted at PlayerScreen.tsx:563 `<PlaylistProvider>` -> PlayerRoute (App.tsx:303-305) -> main.tsx; dispatches over useClient() transport (TauriTransport in desktop shell, DemoTransport in browser) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg -n "playlist" apps/desktop/src/dispatch.rs`; `rg -n "playlist\." ui/src/generated/commands.ts`; `rg -n "playlist" crates/commands/src/catalog/command.rs`; `sed -n '10,15p'` quote check; App.tsx route inspection |

## 1. One-paragraph verdict
This is the master-question answer for the playlist surface: a REAL bridge
client. Every action dispatches a genuine wire command (`playlist.play-item`,
`next`, `prev`, `skip`, `remove`, `move`, `clear`, `set-shuffle`, `set-repeat`,
`enqueue`, `load`, `system.get-state-snapshot`) that exists in the generated
command catalog (ui/src/generated/commands.ts:24) AND in the Rust catalog
(crates/commands/src/catalog/command.rs:287-352); state is kept authoritative by
`playlist.changed` events, with an honest `feature-not-exposed` / unreachable
fallback. No mocks, no fake state machine. Real defects: remounting the provider
(any route navigation away from /player and back) permanently blanks the
displayed queue because there is no `playlist.list` command; and every command
error other than `feature-not-exposed` is swallowed silently.

## 2. What actually works (verified)
— Command names cross-checked 3 ways: UI dispatch strings (this file) ==
generated catalog (commands.ts:24, quote includes `cmd: "playlist.enqueue"` …)
== Rust catalog (command.rs:288-351 renames). — Desktop shell really routes
playlist files to `playlist.load` (apps/desktop/src/dispatch.rs:233-255). —
Tests are spy-based dispatch assertions (PlaylistProvider.test.tsx:126-218,
incl. the mid-flight moved-event race, L218) — they would fail on wrong args.
Not run here (no node_modules; baseline vitest 1705 passed cited). End-to-end
desktop execution UNVERIFIED (desktop build fails at gdk-sys in sandbox —
baseline environment limit, not a repo defect).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/playlist/PlaylistProvider.tsx:10-15, 99, 128-153
Code:
```
// CATALOG GAP (honest): there is no `playlist.list` command — item DETAILS
// (title/source/duration) cannot be enumerated over the wire yet. Entries
// are seeded via `initialEntries` (tests use M3U-shaped fixtures; the app
```
What is wrong: entries live ONLY in provider state (useState(initialEntries),
shipped mount passes none: PlayerScreen.tsx:563). The snapshot effect seeds
currentId/shuffle/repeat but never membership. `<Route>`-based navigation
(App.tsx:291-307: /player <-> /keyboard-shortcuts etc.) unmounts
PlayerRoute/PlaylistProvider; on return the engine still holds the queue but
`items-added` never replays, so the panel renders an EMPTY queue (with a
current item) until the user adds something. Same on webview reload.
Why it matters: realistic user flow (open shortcuts, come back) silently
erases the visible queue — displayed state contradicts engine state.
What correct looks like: a `playlist.list`/snapshot-membership command, or
persisting entries outside the provider until one exists.
Confidence: HIGH — Verification: read (mount site + App.tsx routes + no
membership seeding in snapshot effect L137-141).

F-002 [S3]
Where: ui/src/screens/playlist/PlaylistProvider.tsx:197-215
Code:
```
        (response) => {
          if ("completed" in response.result) return response.result.completed;
          if ("error" in response.result) {
            if (response.result.error.code === "feature-not-exposed") setNotExposed(true);
            return null;
          }
          return null; // accepted — completion arrives via playlist.changed
```
What is wrong: every error except `feature-not-exposed` (invalid id, engine
failure, parse error on playlist.load) returns null with NO user-visible
surface — remove/move/play-item can fail silently forever.
Why it matters: dead-feeling buttons whose clicks do nothing are
indistinguishable from working ones; the honest-unknowns doctrine covers
state, not failures.
Confidence: HIGH — Verification: read (all actions are `void dispatch(...)`
fire-and-forget).

F-003 [S4]
Where: ui/src/screens/playlist/PlaylistProvider.tsx:128-153
Code:
```
        if (summary.current !== undefined) applyCurrentId(summary.current ?? null);
```
What is wrong: a `current-changed` event arriving between the snapshot request
and its response is overwritten by the older snapshot value. Narrow window,
self-corrects on the next event.
Confidence: MEDIUM — Verification: read (effect ordering analysis only).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlaylistProvider | 84-303 | REAL | — | context provider; real transport wiring |
| — requestId | 94-97 | REAL | — | monotonic counter |
| — applyEntries / applyCurrentId | 114-125 | REAL | — | ref+state lockstep (w26.k14) — verified pattern |
| — snapshot effect | 128-153 | REAL | S2 (F-001) | seeds summary only; no membership |
| — playlist.changed effect | 156-195 | REAL | — | items-added/removed/moved/cleared/shuffle/repeat/current all handled; moved clamps to bounds L176 |
| — dispatch | 197-215 | REAL | S3 (F-002) | swallows all errors except feature-not-exposed |
| — enqueue | 217-227 | REAL | — | returns id or null |
| — actions (11) | 229-295 | REAL | — | all real wire commands; playNext reads live refs (race fix documented) |
| — value memo | 297-300 | REAL | — | stable identities (w26.k14) |
| usePlaylist | 305-309 | REAL | — | throws outside provider |
| usePlaylistOptional | 318-320 | REAL | — | used by local-scope screens (MediaList, ThumbnailGrid, toolbarItems — rg verified) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "dispatches playlist.play-item with the stable id" | YES | transport spy asserts exact cmd+args (L135) |
| "play-next reads the queue at dispatch time — a mid-flight moved event shifts the move" | YES | emits event between enqueue and move, asserts move target (L218) |
| 8 others (L127-218) | YES (spot-read) | spy assertions on args/order; no tautologies |
Mocks: tests use a spy transport — mocking the TRANSPORT, not the provider logic under test (legitimate).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the app shell will wire `playlist.load` + a real listing source — handoff to card 4.H1" | PlaylistProvider.tsx:13-14 | SHALLOW | playlist.load wired (desktop dispatch.rs:238); "real listing source" still absent — F-001 |
| "null means 'engine has not reported', never a disguised default" | PlaylistProvider.tsx:5-7 | REAL | state fields all default null; verified L100-102 |

## 7. Dead / duplicated / unreachable code
None. `load()` action: called only from desktop shell path / enqueueSources
(audit below); kept for the 4.H1 seam — reachable.

## 8. Dependencies & flags
useClient (client/ClientProvider), transport types, ./queue (indexOfId,
playNextTargetIndex). No feature gates. No direct Tauri imports — transport-
agnostic by design (real in desktop shell, DemoTransport simulation in browser,
badged per app-transport.ts:58-84).

## 9. Open questions for the next agent
- Engine side: does `playlist.enqueue`/`move` etc. have a REAL handler in the
  playback engine (not just catalog definitions in command.rs)? (commands/
  engine domain.) Catalog existence ≠ execution.
- Does PlaylistPanel render an honest "queue hidden (reload)" state for the
  empty-entries-with-current case of F-001, or a plain empty list?
