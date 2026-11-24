# AUDIT: ui/src/screens/playlist/PlaylistPanel.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 457 / 8 (2 components, 2 nav seams, 1 msg hook, 1 menu builder, 1 helper, 1 inner handler) |
| File verdict | REAL component, DEAD in the shipped desktop host — every queue action dispatches a `playlist.*` command the shipped host refuses, and the refusal is swallowed with no surface |
| Reachable from shipped entry point? | YES (App.tsx:319 /playlist -> PlaylistScreen; PlayerScreen.tsx:563 PlaylistProvider -> docked panel) — reachable UI, unreachable backend |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | rg `PlaylistChanged|playlist.changed` apps/desktop apps/cli crates/playback-engine (exit 0 — hits only in events.rs:99/730/831/1986, "wired-by-later"); rg `case "playlist.` demo-transport.ts (exit 0 — full simulation); sed demo-transport.ts:485-560; sed shell.rs tests (playlist.load refused: shell.rs:553-556, 836-838); rg error codes commands/src/error.rs:123 + ui generated/commands.ts:42 |

## 1. One-paragraph verdict
This is the UI half of Wave 10's unreachable-playlists finding, and it is worse than "unreachable": the panel is a fully-built, accessible, drag-reorderable queue whose EVERY action (prev/next/shuffle/repeat/clear/enqueue/play/play-next/play-later/remove/move, L180-222, L354-368) dispatches `playlist.*` commands over the real transport — and the shipped desktop host (lumen-tauri -> lumen_cli::dispatch SERVED list, apps/cli/src/dispatch.rs:41-56) refuses every one of them typed `unsupported-for-host`. The engine never emits `playlist.changed` (crates/playback-engine/src/events.rs:99 "wired-by-later (no playlist model in the engine; Phase 4)"; :1986 asserts not engine-emitted), so the queue in the shipped shell is permanently empty and every button is a silent no-op: the provider's dispatch (PlaylistProvider.tsx:197-215) returns null on any error and only raises the honesty flag on `feature-not-exposed` — the demo transport's code — never on the real host's `unsupported-for-host`. The honest "not connected" note (L243) therefore renders only in degenerate cases; in the product build the user gets dead buttons with zero feedback. The component only does real work against the browser DEMO transport, which simulates the whole playlist surface (demo-transport.ts:492-560) — mock-as-real by architecture.

## 2. What actually works (verified)
— verified by rg + sed: the queue works end-to-end ONLY under demo-transport (playlist.enqueue/play-item/next/... simulated at demo-transport.ts:492-560 with playlist.changed events at :890-963); the "Open media…" entries dispatch the real shell-wide event (L432) whose listener exists (OpenMediaDialog.tsx:56); "Media info" routes to the real /media-info screen (L453).
— verified by rg `case "system` demo-transport.ts:398-404: demo simulates system.get-state-snapshot, so the provider seeds shuffle/repeat/current in the demo shell.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:180-222 and 354-368 (every action) vs apps/cli/src/dispatch.rs:41-56 (SERVED) and crates/playback-engine/src/events.rs:99
Code:
```
          <IconButton label={m("playlist.controls.prev")} size="sm" onClick={() => playlist.prev()}>
          ...
          <IconButton label={m("playlist.controls.clear")} size="sm" onClick={() => playlist.clear()}>
```
What is wrong: every wired action maps to `playlist.prev/next/set-shuffle/set-repeat/clear/play-item/move/remove/enqueue` — NONE of the 15 SERVED commands is a playlist command; the desktop host answers each with typed `unsupported-for-host` (dispatch.rs unserved(), apps/cli/src/dispatch.rs:152-156 region). The provider swallows the error (PlaylistProvider.tsx:202-207: non-matching error -> `return null`, no state change). Net effect in the shipped app: 10+ interactive controls render enabled, accept clicks, and do nothing, with no error surface. Why it matters: this is the definition of a dead-button surface — the panel advertises queue control the product cannot perform. What correct looks like: disable/annotate actions when the host's refusal arrives (the panel already has the pattern — the menu's favorite/save/add-audio rows are honestly disabled, L408/424/446). Confidence: HIGH — Verification: read (SERVED list, provider dispatch, panel handlers).
F-002 [S2]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:243 (the honesty note) vs PlaylistProvider.tsx:142-148, 202-211
Code:
```
      {playlist.notExposed ? <p className="lumen-playlist__note">{m("playlist.panel.not-exposed")}</p> : null}
```
What is wrong: the note ("Playlist commands are not exposed by this build — showing seeded data only.", messages.ts:1550) is gated on `notExposed`, which the provider sets ONLY for error code `feature-not-exposed` (the DEMO transport's refusal, demo-transport.ts:389) or an unreachable transport — never for the REAL host's `unsupported-for-host` (commands/src/error.rs:123). In the one shell where playlist commands are truly dead, the disclaimer never renders; in the demo shell where they work, it also never renders. The honesty mechanism is dead code in both shells. Confidence: HIGH — Verification: read (provider L142-148/202-211 + error-code strings both sides).
F-003 [S3]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:288
Code:
```
      <footer className="lumen-playlist__footer">{m("playlist.panel.load-handoff")}</footer>
```
What is wrong: the footer copy is "Pick a file or drop it anywhere — playlists arrive with the desktop build." (messages.ts:1551) — a roadmap note shipped as permanent product chrome, and it is FALSE in the shipped desktop build: the desktop host refuses `playlist.load` too (shell.rs:553-556 "playlist.load is not served by this engine build"). The UI's own footer promises the exact thing the shipped host refuses. Confidence: HIGH — Verification: read + sed shell.rs:541-556.
F-004 [S3]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:406-446 (menu rows 2/4/7)
Code:
```
    { id: "favorite", labelKey: "playlist.queue.menu.favorite", icon: "favorite", iconAccent: true, disabled: true },
    ...
    { id: "save", labelKey: "playlist.queue.menu.save", icon: "save", iconAccent: true, disabled: true },
    ...
    { id: "add-audio", labelKey: "playlist.queue.menu.add-audio", icon: "audio", iconAccent: true, disabled: true },
```
What is wrong: 3 of the 8 context-menu rows are permanently disabled placeholders (no favorites store, no queue-export, no per-item track-add) — honestly labeled, but shipped dead chrome replicated per-row on every queue item. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:149-165
Code:
```
      const raw = event.dataTransfer.getData("text/plain");
      if (raw.trim() === "") return;
      from = Number(raw);
```
What is wrong: external drops decode `Number(raw)` and index `playlist.entries[from]` — guarded by `Number.isInteger` and the `entry` existence check, so no crash, but a foreign text payload that parses as an integer (e.g. "3" dragged from another app) silently moves queue row 3. Minor, deliberate tradeoff documented at L153-155. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useQueueMenuMsg | 60-66 | REAL | — | lane-key fallback |
| NavigationSeam / RoutedNavigation | 74-85 | REAL | — | router-safe nav |
| openMediaDialog | 91-93 | REAL | — | wired to real dialog listener |
| PlaylistPanel | 95-300 | REAL UI / dead backend | S2 | all actions refused by shipped host (F-001); note never fires (F-002); false footer (F-003) |
| onRowDrop (inner) | 149-165 | REAL | S4 | F-005 external-drop decode |
| PlaylistRow | 302-378 | REAL UI / dead backend | S2 | play/play-next/play-later/remove all refused in product |
| queueRowMenu | 386-456 | REAL | S3 | 3/8 rows permanently disabled (F-004); shuffle row checked-state is engine-null-honest |

## 5. Test honesty
No tests in this file. enqueueSources.test.tsx / queue tests exist in the lane. All of them mount a FAKE transport that answers playlist commands — precisely the case the shipped host never produces; no test asserts behavior under `unsupported-for-host`, which is why F-001/F-002 survived. Baseline vitest 1705 passed proves the mock path only. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "HONESTY: current marker, shuffle and repeat reflect ENGINE-reported state only (playlist.changed / snapshot)" (L11-13) | module header | SHALLOW | in the shipped shell neither playlist.changed (events.rs:99) nor the snapshot (not SERVED) ever arrives — the "engine-reported" source is demo-only |
| "real seams only — Play (playlist.play-item), Shuffle (playlist.set-shuffle...)" (L19-21) | module header | FALSE in product | both commands are typed-refused by the shipped host (SERVED list); "real seam" is wire-shaped, not backend-real |
| "never theater (PlaylistScreen's honest-Play precedent)" (L25-26) | module header | FALSE in product | enabled buttons that silently no-op ARE theater-by-omission (F-001) |

## 7. Dead / duplicated / unreachable code
In the shipped desktop shell: the entire interactive surface of this panel (toolbar 6 buttons, row actions 4 per row, drag-reorder) is unreachable-backend dead UI. The `notExposed` note branch (L243) is dead in both shells (F-002). Menu rows favorite/save/add-audio dead by design everywhere (F-004).

## 8. Dependencies & flags
components/{button,icon,menu,list}, open-media OPEN_MEDIA_EVENT, enqueueSources, PlaylistProvider, AddSubtitlesDialog, options/sorting, queue helpers, playlist.css. No feature flags. The panel inherits liveness from whatever transport ClientProvider mounts (tauri in product, demo in browser).

## 9. Open questions for the next agent
- PlaylistProvider.tsx owner (not my assignment): why does dispatch() treat only `feature-not-exposed` as dead-surface evidence? Suggest flagging `unsupported-for-host` too — that single change would light the L243 note in the real shell.
- demo-transport owner: "Honest simulation" of shuffle flips the flag but keeps sequential order — do any UI tests assert reordering on shuffle (they would pass against a lie)?
