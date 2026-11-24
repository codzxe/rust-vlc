# AUDIT: ui/src/screens/player/customize/CustomizePanels.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 842 / 9 (CustomizePanels, CustomizeBand, useBandGeometry, bandVariant, useCustomizeMsg, applyCompletion/dispatch callbacks, moveTile/addToBand/removeFromBand, onTileKeyDown, closePanel/onDialogClose) |
| File verdict | SHALLOW (drag UI real; dispatch real for 7 catalog commands; 3 tiles dispatch commands that do not exist on the wire) |
| Reachable from shipped entry point? | YES — App.tsx → /player → PlayerScreen.tsx:1247 `{customizeOpen ? <CustomizePanels onClose={onCustomizeClose} /> : null}` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -n` over send sites / wire union; `sed -n` quote checks; read of panels.ts, capabilities.ts, toolbarLayout.ts, generated/commands.ts L24 |

## 1. One-paragraph verdict
The "Customize Player" modal is a real, elaborate drag-and-drop surface whose engine-facing half is honest for 7 commands (`playback.seek`, `playback.set-rate`, `playback.stop`, `playlist.set-repeat`, `playlist.set-shuffle`, `video.take-screenshot`, `video.set-zoom`) — dispatched via `wire.send` with engine-confirmed state only, never optimistic. But three tiles (Record, PiP, HDR tone-mapping) dispatch command ids that are **not in the generated wire union at all** (`video.record`, `video.toggle-pip`, `video.set-tone-mapping` — absent from generated/commands.ts L24), pushed through an `as unknown as CommandRequest` cast, and the capability gating that was supposed to hide them was retired by policy — so these are live buttons that can only ever produce a silently swallowed error. The band preview chrome is decorative by design (aria-hidden), which is legitimate.

## 2. What actually works (verified)
— verified by code-path read: tile click → `dispatch()` (L158-184) → `wire.send(request)` → transport (tauri `lumen_dispatch` in the desktop build) → `applyCompletion` only on `"completed" in response.result`. Event subscription (L188-204) maps `playlist.changed.shuffle-set` and cycle events into tile state. NOT executed live by me (no engine session) — REAL-UNPROVEN at runtime.
— verified by read: layout Save → `persistToolbarLayout(layout)` → localStorage `lumen.player.toolbar.v1` (toolbarLayout.ts:20) — UI-owned state per doc 08 §4, a legitimate localStorage use (toolbar arrangement is not engine state).
— CustomizePanels.test.tsx: 42 `it(` blocks covering drag reorder, place/remove, save/cancel, focus recovery.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/customize/CustomizePanels.tsx:158-169; panels.ts:128-136,150-165; capabilities.ts:1-15
Code:
```
      // Forward-declared commands (PiP, HDR) are not yet in the 1.D4 wire
      // union — the cast is the bridge until `cargo xtask codegen` regrows
      // the catalog (recorded in ui/NEXT-STEPS.md).
      const request = {
        cmd: tile.command,
        args,
        requestId: nextRequestId.current,
      } as unknown as CommandRequest;
```
What is wrong: THREE tiles dispatch commands absent from the wire catalog: `video.record` (panels.ts:135), `video.toggle-pip` (panels.ts:150), `video.set-tone-mapping` (panels.ts:158). The union in generated/commands.ts:24 contains no `video.record`, no `video.toggle-pip`, no `video.set-tone-mapping` (verified by rg over the union line). The comment admits only "PiP, HDR" — record is undocumented even in the cast's own apology. Worse, panels.ts:130-131 claims "forward-declared id, hidden until an engine lists it", but the hiding mechanism was deleted: capabilities.ts:3-6 states "the customize box renders the frame's FULL 31-tile catalog against ANY `system.list-commands` listing... The retired gating helpers (tileVisible/visibleTiles) are deleted". So a user clicks Record/PiP/HDR and the request goes out, the engine returns an unknown-command error, and the catch swallows it — a button that can never do anything, with no disabled state and no feedback. Why it matters: dead controls presented as live ones — exactly what doc 05 §3 "no dead controls" forbids. What correct looks like: either codegen the commands into the union and implement engine-side, or render those three tiles inert. Confidence: HIGH — Verification: read (union line, panels.ts, capabilities.ts).

F-002 [S3]
Where: ui/src/screens/player/customize/CustomizePanels.tsx:172-179
Code:
```
        // Errors (feature-not-exposed etc.) leave tile state untouched —
        // honest no-change, no fabricated success.
      } catch {
        // Transport failure — same honest no-change policy.
      }
```
What is wrong: both the error-result path and the transport-failure path are silent — no toast, no aria-live, no visual cue. "Honest no-change" is honest to the state machine but dishonest to the user, who sees a click with no effect and no reason. Confidence: HIGH — Verification: read.

F-003 [S3]
Where: ui/src/screens/player/customize/capabilities.ts:13-15 (dependency of this surface)
Code:
```
// The listing query below therefore has NO product consumer inside the
// customize surface anymore; it stays exported (frozen barrel API —
// removals are not additive)
```
What is wrong: self-admitted dead code kept for an API freeze; the seam this surface's header still advertises ("capability-gated in capabilities.ts", panels.ts:9-10) no longer exists. A header that describes retired machinery. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: ui/src/screens/player/customize/CustomizePanels.tsx:115-119
Code:
```
      (key: CustomizeTile["labelKey"] | MessageKey): string =>
      m(key as MessageKey) ??
      a17Keys[key as keyof typeof a17Keys] ??
```
What is wrong: double `??` chain ends in `?? key` (raw key fallback reaches users if all catalogs miss) — same class as OnboardingScreen F-004; acceptable but undocumented. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useCustomizeMsg | 82-94 | REAL | - | i18n fallback chain |
| CustomizePanels | 107-519 | SHALLOW | S2 | real dispatch for 7 cmds; 3 void dispatches (F-001); silent errors (F-002) |
| applyCompletion | 128-148 | REAL | - | engine-confirmed state only |
| dispatch | 158-184 | REAL (7 cmds) / DEAD (3 cmds) | S2 | cast bridge F-001 |
| event subscription effect | 188-204 | REAL | - | shuffle-set + cycle events |
| moveTile/addToBand/removeFromBand | 224-250 | REAL | - | layout reducers |
| onTileKeyDown | 252-270 | REAL | - | keyboard drag equivalents |
| closePanel/onDialogClose | 272-289 | REAL | - | ✕/Escape; scrim never closes |
| focus-keeper effect | 314-337 | REAL | - | h07 defect 2 recovery |
| CustomizeBand | 625-842 | REAL | - | decorative preview chrome, correctly aria-hidden |
| useBandGeometry | 531-575 | REAL | - | measures real strip; jsdom fallback |
| bandVariant | 599-607 | REAL | - | pure frame-variant derivation |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 42 it() blocks in CustomizePanels.test.tsx | YES for layout/drag/a11y | behavioral, mock-transport asserts on dispatched requests |
| No test dispatches video.record / video.toggle-pip / video.set-tone-mapping | — | rg over the test file finds zero mentions — the F-001 void dispatches are untested (nothing would catch the engine never knowing these commands) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "engine-bound tiles dispatch their commands honestly" | file header L33-34 | SHALLOW | true for 7; 3 dispatch non-existent commands (F-001) |
| "capability-gated in capabilities.ts — hidden until the engine lists the command" | panels.ts:9-11 | FALSE | gating helpers deleted (capabilities.ts:3-6); box renders full catalog always |
| "Save persists the arrangement (doc 08 §4 UI-owned layout state...)" | header L29-30 | REAL | persistToolbarLayout → localStorage `lumen.player.toolbar.v1` |

## 7. Dead / duplicated / unreachable code
- `useCommandCapabilities` (capabilities.ts) — no product consumer inside this surface (self-admitted, kept for barrel freeze).
- Three tile dispatch targets that cannot succeed on the current wire.

## 8. Dependencies & flags
Shared Dialog/Icon/IconButton/Button primitives, cycleState.ts (shared with the real bar), generated wire types. No feature gates. No env vars.

## 9. Open questions for the next agent
- Does the Rust dispatcher reply a typed `unknown-command` error for `video.record` (vs. rejecting the IPC call)? Determines whether F-001's swallowed branch is the error path or the catch path. Check apps/desktop dispatch + crates/command catalog.
- toolbarItems.tsx (the REAL bar) — same question: does it render the record/PiP tiles? (toolbarItems.tsx:173 dispatches `media.add-bookmark` with a hard-coded `requestId: 2` — collision-prone; audit in its own lane.)
