# AUDIT: ui/src/client/demo-transport.ts
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 992 / 34 (incl. 27 methods of `DemoTransport`) |
| File verdict | REAL-UNPROVEN (honest simulation; executes only as browser dev-shell fallback — NOT in the shipped desktop lane) |
| Reachable from shipped entry point? | NO for desktop lane; YES for `npm run dev` / visual harness. Chain: ui/src/main.tsx:56 `selectAppTransport()` → ui/src/client/app-transport.ts:62-75 `detectTauriBackend()` null ⇒ `new DemoTransport()`. Inside the Tauri shell `__TAURI_INTERNALS__` is present, so app-transport.ts:67 selects `TauriTransport` and this class is never constructed. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | Grep DemoTransport/__TAURI_INTERNALS (exit 0); sed quote verification (exit 0); ls ui/node_modules → absent (npm install banned ⇒ no self-run; baseline cited) |

## 1. One-paragraph verdict
This file is exactly what its header says: a browser-only simulated engine that speaks the real 1.D4 wire protocol with zero decoded media. The honesty contract holds — unsupported commands hit a typed `feature-not-exposed` default, engine info refuses, shuffle is flag-only and says so. It is NOT wired to any backend by design; in the shipped desktop build it is dead weight (never constructed). The master question's answer for the dev shell: every UI element you see at `npm run dev` is driven by this simulation, not the engine. Two defects: a stale header claim that the Tauri wiring is "a separate GitHub issue" (it is wired, app-transport.ts:67), and `applySelector` fabricating success for nonexistent track ids, which contradicts the file's own "nothing fabricates engine capability" rule.

## 2. What actually works (verified)
— verified by rg over ui/src (exit 0): `DemoTransport` constructed only at app-transport.ts:75 (prod) and in *.test.* files; `isDemoTransport` guards at ControlBar.tsx:924, StageOverlays.tsx:504, PlayerScreen.tsx:718, enqueueSources.ts:31, OpenMediaDialog.tsx:124-130 hide demo-only UI in the desktop shell.
— verified by baseline audit/runs/baseline/ui-test.txt:134 `✓ src/client/demo-bookmarks.test.ts (5 tests) 6ms` and :178-179 `Test Files 136 passed / Tests 1705 passed | 1 skipped` — the simulation's bookmark semantics, listing gate, refusals, per-media reset all pass under the orchestrator run. Self-execution UNVERIFIED (node_modules absent; npm install banned).

## 3. Findings
F-001 [S3]
Where: ui/src/client/demo-transport.ts:11-13
Code:
```
//   * The desktop host injects the real transport (TauriTransport,
//     client/tauri-transport.ts) at the main.tsx seam — that wiring is a
//     separate GitHub issue.
```
What is wrong: The comment asserts the desktop wiring is NOT done. It IS done: ui/src/main.tsx:56 `const appTransport = selectAppTransport();` and ui/src/client/app-transport.ts:62-71 probe `__TAURI_INTERNALS__` and construct `TauriTransport`. Why it matters: the module's scope contract is the reference doc for the whole demo/dev-shell posture; a reader concluding "the desktop app still runs the demo" is wrong. What correct looks like: delete the stale sentence. Confidence: HIGH — Verification: read (app-transport.ts:58-80, main.tsx:54-56).

F-002 [S3]
Where: ui/src/client/demo-transport.ts:967-978
Code:
```
  private applySelector(
    kind: "video" | "audio" | "subtitle" | "secondaryAudio",
    selector: TrackSelector,
  ): CommandResponse["result"] {
    this.selections = { ...this.selections, [kind]: selector.kind === "id" ? selector.id : null };
```
What is wrong: No validation that `selector.id` exists in `this.info.tracks`; selecting track 999 on the bunny clip returns `applied-selector` success and emits `tracks.changed`. This contradicts the file's own honesty contract ("Nothing in this file fabricates engine capability", lines 13-14) — a real engine refuses unknown track ids, `tracks.get-info` (L459-471) does refuse them, so the two paths disagree about the same id. What correct looks like: mirror the `tracks.get-info` lookup and answer `not-found`. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: ui/src/client/demo-transport.ts:447-452
Code:
```
      case "playback.step-frame":
        this.seek({
          kind: "relative",
          deltaMs: request.args.direction === "next" ? 1000 / 24 : -1000 / 24,
```
What is wrong: Frame step hard-codes 24 fps for every clip; the live fixture it serves declares `frameRate: 30` (L106). Demo-scope inaccuracy. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: ui/src/client/demo-transport.ts:743 and 417-420
Code:
```
    this.session = 1;
```
```
      case "session.close":
        this.position = 0;
        this.transition("idle");
```
What is wrong: `session` is hard-coded 1 for every open and never cleared on close; `info` also survives close, so after `session.close` the still-listed `media.get-duration` (L581-584) and `media.get-current-time` (L570-580, `bufferedRanges` L691-701 returns the full range of the closed media) report stale data while `media.get-info` (L559) refuses. Inconsistent post-close face; harmless in a dev shell. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: ui/src/client/demo-transport.ts:540-548 — `playlist.set-shuffle` flips the flag, keeps sequential advancement. Documented honestly in-line (L542-543); listed here because the demo playlist therefore behaves differently from any real engine under shuffle. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: ui/src/client/demo-transport.ts:440-443 — `playback.set-rate` stores any rate (negative, 0, 10⁶) unclamped and echoes it. Demo-scope. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| knownUs | 83 | REAL | — | µs duration literal helper |
| sourceKey / sourceTitle | 161-176 | REAL | — | basename/hostname fallback, try/catch on URL |
| infoForSource | 179-194 | REAL | — | seeded catalog else synthesized infinite/live info |
| constructor | 327-344 | REAL | — | 500ms clock forever until dispose; seeds 2 fake volumes + delayed `devices.volume-added` events (L340-343) — fabricated device rows, dev shell only |
| dispose | 347-351 | REAL | — | clears timers + listeners |
| recentOpenings (get) | 354-356 | REAL | — | reversed copy, cap 6 |
| subscribe | 358-375 | REAL | — | replays volume-added to NEW subscribers with fresh seq; `as Event` cast (L370) is the DistributiveOmit round-trip |
| send | 377-657 | REAL | — | 44-case wire dispatch; refusals typed (`not-found`, `out-of-range`, `invalid-argument`, `feature-not-exposed`) |
| snapshot | 663-689 | REAL | — | fake devices `demo-stereo`/`demo-hdmi` (L666-669); µs time twin correct |
| bufferedRanges | 691-701 | REAL | — | full range for known, 30s sliding window for live |
| emit / emitIn / schedule | 703-723 | REAL | — | seq stamping, dispose-tracked setTimeout |
| transition / transitionIn | 725-733 | REAL | — | state-changed events |
| openSource | 739-760 | REAL | — | fixed 160/280/320ms choreography; F-004 session=1 |
| emitInfoBurst | 762-777 | REAL | — | info/tracks/duration/first tick |
| play | 779-793 | REAL | — | resume / restart / reopen-current semantics |
| seek | 795-828 | REAL | — | absolute µs, relative ms, percent, chapter; clamps to [0,known] |
| tick | 830-861 | REAL | — | 500ms × rate; end-of-media, repeat-one loop, auto-advance 480ms |
| durationSeconds / positionFraction | 865-880 | REAL | — | unit conversions via wire-time.ts; fraction 0 for live (documented) |
| enqueue / setCurrent / stepPlaylist | 886-919 | REAL | — | wrap-around stepping, honest no-op on empty |
| removeItem / moveItem | 921-965 | REAL | — | successor selection and index clamping correct; `item!` (L961) guarded by `from < 0` check |
| applySelector | 967-978 | SHALLOW | S3 | F-002: no id validation |
| rememberOpening | 980-986 | REAL | — | dedupe + cap 6 |
| isDemoTransport | 990-992 | REAL | — | instanceof guard used by 5 UI call sites |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| demo-bookmarks.test.ts (5 tests, baseline PASS) | YES | asserts real ids/renumber/reset/refusal semantics against the real class (would catch id-reuse or missing reset) |
| app-transport.test.ts:64,82 (baseline PASS) | YES | proves the selection rule both ways (demo fallback / real TauriTransport) |
| OpenMediaDialog/PlayerScreen/enqueueSources tests using DemoTransport | YES (for UI wiring) | they fail if the demo↔UI contract drifts; they prove nothing about the real engine — by design |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no bytes are decoded, no clocks are wall-locked, no audio leaves the speakers" | demo-transport.ts:9-10 | TRUE | no decode/audio code exists in the class; clock is setInterval-position arithmetic (L830-832) |
| "commands the demo cannot simulate … answer with the typed `feature-not-exposed`" | demo-transport.ts:14-17 | TRUE | L654-655 `default: return notExposed();` + L404-405 engine-info |
| "that wiring is a separate GitHub issue" (Tauri injection) | demo-transport.ts:12-13 | FALSE (stale) | app-transport.ts:62-71 + main.tsx:56 wire it today — F-001 |
| "The desktop shell runs the real TauriTransport and renders no badge" | App.tsx:132-133 | TRUE (code side) | app-transport.ts:67; badge gating outside this file |

## 7. Dead / duplicated / unreachable code
None dead inside the file. The whole class is unreachable in the shipped desktop lane (constructed only when `__TAURI_INTERNALS__` is absent — app-transport.ts:63-75); it is the live transport for `npm run dev`, the vitest suite, and the visual harness. `DEMO_CLIPS` URLs (`https://demo.lumen.dev/...`, soma.fm) are display strings in the sim — nothing ever fetches them.

## 8. Dependencies & flags
Imports: i18n MessageKey (type only), media-info fixtures (`fullMediaInfo`, `liveAudioMediaInfo`), wire-time conversions, generated/commands types, transport interface. No feature flags; selection is runtime probe, not build flag. No network, no DOM beyond setTimeout/setInterval.

## 9. Open questions for the next agent
- MediaSurface.tsx (not assigned to me): does the frame channel (`frame-channel.ts`) reach real `putImageData` under Tauri, and does anything render when DemoTransport is live? My read of app-transport suggests the desktop path is real; verify the `lumen_dispatch` side exists in the Rust host (crates side).
- Does the Rust host actually implement every command DemoTransport lists in LISTED_COMMANDS (L200-241)? A UI capability gate built on a demo listing can overstate the real engine.
