# AUDIT: ui/src/screens/playlist/PlaylistProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 320 / 12 (PlaylistEntry/PlaylistState/PlaylistActions types, PlaylistProvider [requestId, state+live-refs, applyEntries/applyCurrentId, snapshot effect, playlist.changed subscription, dispatch, enqueue, actions memo, value memo], usePlaylist, usePlaylistOptional) |
| File verdict | REAL (with one S2 real-host dead-surface finding) |
| Reachable from shipped entry point? | YES (App.tsx:303 `/player` → PlayerScreen.tsx:563 `<PlaylistProvider>` around the whole player composition; ControlBar/queue panel/transport cluster consume it). Baseline executed its 17 tests (136/136 files, 1705 passed). Product caveat: no desktop binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | Grep `PlaylistChange::\|PlaylistChanged` over crates/ → **the engine NEVER constructs an ItemsAdded/current-changed/shuffle-set/repeat-set event in production code** (only tests do); Read crates/playback-engine/src/events.rs:727-734 (`EventId::PlaylistChanged` → `WiredLater { owner: "no playlist model in the engine (Phase 4, 4.D1)" }`) and :1984-1986 (`assert!(!is_engine_emitted(EventId::PlaylistChanged))`); Read apps/cli/src/dispatch.rs:38-54 (SERVED table: NO playlist commands, NO system.get-state-snapshot) + :15-18 ("Every other catalog id answers the honest typed refusal `unsupported-for-host`"); Read apps/desktop/src/dispatch.rs:42-44 (the desktop shell reuses the SAME CliHost) + apps/desktop/src/shell.rs:548-556 (test asserting `playlist.load` → `ErrorCode::UnsupportedForHost` — "not served by this engine build"); `rg -n 'unsupported-for-host' src/screens/player/context.tsx` (:196 — the player context DOES handle the real host's refusal code; the provider does not); Read demo-transport.ts:886-898 (the demo FABRICATES playlist.changed events the engine never emits); Read queue.ts (playNextTargetIndex math) + PlaylistProvider.test.tsx:127-296 (17 behavioral tests). Baseline: ui-vitest.log.txt 136/136, 1705 passed. |

## 1. One-paragraph verdict
As React code, this provider is disciplined and well-tested: engine-truth-only state (nulls for unreported), live-ref architecture that fixed a real stale-closure race (w26.k14, pinned by test :218), stable action identities (pinned :238), and honest-unknown field propagation for event-added ids — 17 behavioral tests execute in baseline. As PRODUCT code, its entire sync channel is dead against the real host, and its honest-degradation flag never fires: the engine has NO playlist model (events.rs:730-733 — `playlist.changed` is `WiredLater`, owner "no playlist model in the engine (Phase 4, 4.D1)"; asserted not-engine-emitted at :1986), the desktop shell serves NO playlist commands and not even `system.get-state-snapshot` (SERVED table, dispatch.rs:38-54; the shell's own test proves `playlist.load` → `unsupported-for-host`), and the provider keys its `notExposed` flag ONLY on `feature-not-exposed` (:142, :203) — the code the real host never sends for these ids (the player context handles the real code at context.tsx:196; this file doesn't). Net real-product behavior: the queue UI silently degrades to permanently-empty/disabled with "no media" reasons while `PlaylistPanel`'s honest not-connected note (keyed on notExposed) never renders. Meanwhile the DemoTransport fabricates the whole event family (items-added at :890, current-changed at :897), so the browser demo shows a living playlist the product cannot have — the wave's demo-theater pattern in its purest form.

## 2. What actually works (verified)
— verified by PlaylistProvider.test.tsx:127-296 (snapshot seeding, per-command dispatch args, play-next insertion math incl. the mid-flight `moved` race, action identity stability, honest unknown fields on items-added, feature-not-exposed flagging) + baseline (136/136 files, 1705 passed).
— verified by reading queue.ts + reasoning the index math: playNextTargetIndex is correct under BOTH event/response orderings (append-at-end and insert-after-current are invariant to a stale tail length; the w26.k14 live-ref fix covers the mid-flight reorder case its test pins).
— verified by PlayerScreen.tsx:563 → the provider is product-mounted around the whole player composition, and usePlaylistOptional (:318-320) gives the /playlist library page its honest "no engine scope" null.

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/playlist/PlaylistProvider.tsx:142-144 and 202-205 (the refusal-code keying); header claim at 10-15
Code:
```ts
        } else if ("error" in response.result && response.result.error.code === "feature-not-exposed") {
          setNotExposed(true);
        }
```
(and)
```ts
          if ("error" in response.result) {
            if (response.result.error.code === "feature-not-exposed") setNotExposed(true);
            return null;
          }
```
What is wrong: Three stacked mismatches against the real host. (1) The engine never emits `playlist.changed` — the event family this provider's header calls "authoritative" (line 14) is `WiredLater` with owner "no playlist model in the engine (Phase 4, 4.D1)" (crates/playback-engine/src/events.rs:729-733) and asserted not-engine-emitted (:1986). (2) The desktop shell (which reuses the CLI host, apps/desktop/src/dispatch.rs:42-44) serves none of the playlist commands and not `system.get-state-snapshot` either (SERVED, apps/cli/src/dispatch.rs:38-54); its own test pins `playlist.load` → `unsupported-for-host` (apps/desktop/src/shell.rs:548-556). (3) The provider flags honest degradation ONLY on `feature-not-exposed` — the MockTransport's code — while the real host answers `unsupported-for-host` (the wire knows both, generated/commands.ts:42; the player context handles the real one at context.tsx:196). So every real-host dispatch lands in the unhandled branch: `notExposed` stays false, PlaylistPanel's not-connected note (PlaylistPanel.tsx:243) never renders, and the queue surface shows "no media" emptiness that misattributes an unserved host to an empty playlist. The header's "the app shell will wire `playlist.load` + a real listing source — handoff to card 4.H1" is undelivered: 4.H1's shell exists and classifies playlist files into exactly the refusal its test pins.
Why it matters: The honest-degradation design this file exists to implement (doc 08 §5 truthful UI) is defeated by an error-code drift inside the UI itself: the one branch that would tell the user the truth never fires. And the demo lane (DemoTransport fabricates items-added/current-changed, demo-transport.ts:890/897) demonstrates a fully-alive playlist the real product cannot reproduce — features that work in the demo and are dead in the product.
What correct looks like: Key the flag on BOTH typed refusals (`feature-not-exposed` || `unsupported-for-host`) — one line in each of the two sites — so the panel's honest note renders against the real host today; keep the rest as-is until the engine's Phase-4 playlist model lands and the SERVED table grows.
Confidence: HIGH — Verification: all five citation sites read (engine ledger, SERVED table, shell test, player-context asymmetry, demo fabrication); runtime UNVERIFIED (no GUI binary — W14-A01), but the refusals are static certainties of the shipped host tables.
### F-002 [S4]
Where: ui/src/screens/playlist/PlaylistProvider.tsx:55-57 vs 145-148, 208-211
Code:
```ts
  /** True once any playlist command answers `feature-not-exposed` — the
   * panel then shows the honest not-connected note instead of pretending. */
  readonly notExposed: boolean;
```
vs
```ts
      } catch {
        // Host unreachable — leave honest nulls; the panel shows the note.
        setNotExposed(true);
      }
```
What is wrong: The state field's doc says "any playlist command answers `feature-not-exposed`", but the catch paths set the same flag for a DIFFERENT condition (transport unreachable). The field means "not exposed OR not connected" — the doc describes only the first.
Why it matters: Conflation in an honesty flag: a consumer cannot distinguish "the host cannot do playlists" from "the transport is down" (different user guidance, different retry semantics). Benign today because both render the same note; a lie to the next maintainer.
What correct looks like: Either rename (notConnected) or split the two states; update the doc comment to match reality.
Confidence: HIGH — Verification: read both paths.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PlaylistEntry`/`PlaylistState`/`PlaylistActions` | 39-80 | REAL | S4 | Honest-null grammar; notExposed doc drift F-002; playNext/playLater/load documented seams. |
| `requestId` | 93-97 | REAL | — | Stable counter. |
| state + live refs + `applyEntries`/`applyCurrentId` | 99-125 | REAL | — | The w26.k14 lockstep pattern; one mutation point for ref+state. |
| snapshot effect | 128-153 | REAL | S2 | Seeds current/shuffle/repeat once; against the real host the command is unserved and the error branch misses (F-001); cancellation flag correct. |
| `playlist.changed` subscription | 156-195 | REAL | S2 | Correct reducer logic (moved clamps post-removal; items-added honest nulls) — but the engine never emits any of it (F-001); demo-only traffic. |
| `dispatch` | 197-215 | REAL | S2 | feature-not-exposed-only keying (F-001); accepted-vs-completed honesty correct (completions drive state; accepted waits for events). |
| `enqueue` | 217-227 | REAL | — | Returns the item id from the completion; clean args shaping. |
| `actions` memo | 229-295 | REAL | — | All 11 actions dispatch real catalog ids with correct args; setShuffle/setRepeat update from COMPLETIONS (works even without events); playNext reads live refs (race-fixed, test-pinned); identities stable (deps all stable callbacks). |
| `value` memo | 297-300 | REAL | — | Spreads state+actions; changes only on state change. |
| `usePlaylist` | 305-309 | REAL | — | Throw-on-missing (guards misuse). |
| `usePlaylistOptional` | 311-320 | REAL | — | The /playlist library page's honest null scope. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :184/:197/:218 play-next math (incl. mid-flight moved) | YES | Asserts the exact toIndex dispatched through a fake transport with an onEnqueue seam that injects a concurrent event. |
| :238 action identities survive queue changes | YES | Pins the w26.k14 memo fix. |
| :259/:267/:278 event reducers | YES | DOM/state assertions per change kind. |
| :289 feature-not-exposed flagging | YES | Pins the flag's happy code — and pins the BUG: no test feeds `unsupported-for-host`, so F-001's missed branch is invisible to the suite. |
| :127 snapshot seeding | YES | Fake snapshot → seeded state. |
| Gap: no test drives the real host's refusal code; no test asserts the note does NOT show (F-001 unpinned). | | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`playlist.changed` keeps membership/order/current authoritative" | PlaylistProvider.tsx:14 | FALSE against the real engine; REAL only as the DemoTransport contract | events.rs:729-733 + :1986 (never emitted); demo-transport.ts:890/897 (fabricated). |
| "the app shell will wire `playlist.load` + a real listing source — handoff to card 4.H1" | PlaylistProvider.tsx:13-14 | FALSE (undelivered) | 4.H1's shell classifies playlist files into `playlist.load` and receives `unsupported-for-host` (shell.rs:548-556, its own test). |
| "True once any playlist command answers `feature-not-exposed`" | PlaylistProvider.tsx:55-56 | PARTLY (also set on transport failure) | F-002. |
| "only engine-reported state … plus honestly-labelled unknowns" | PlaylistProvider.tsx:4-6 | REAL | Nulls preserved end-to-end; tests pin honest unknown fields. |

## 7. Dead / duplicated / unreachable code
Dead: none in-file (the subscription is live code awaiting an engine). Duplicated: none significant. Unreachable-as-product: the whole event-reducer half (engine never emits) — reachable only via DemoTransport.

## 8. Dependencies & flags
Transport-agnostic (useClient); commands used: system.get-state-snapshot, playlist.{enqueue,play-item,next,prev,skip,remove,move,clear,set-shuffle,set-repeat,load} — all in the generated wire union (commands.ts:24), none in the shipped hosts' SERVED tables (F-001). No feature flags. queue.ts pure helpers shared with the drag-reorder surface.

## 9. Open questions for the next agent
- The one-line fix (accept `unsupported-for-host` in both keying sites) — who owns it, and does any UI test suite have a real-host-refusal fixture to pin it?
- DemoTransport emits `playlist.changed` BEFORE answering the enqueue response (demo-transport.ts:890-891), while doc 05 §4.2 guarantee 2 specifies responses precede their caused events — the demo inverts the documented wire order. Benign for this provider's math (verified §2) but a contract deviation the demo-transport owner should reconcile before the real playlist model lands.
- When Phase 4's playlist model lands, `initialEntries` seeding and the no-playlist.list gap collapse — the CATALOG GAP header should then be deleted, not updated.
