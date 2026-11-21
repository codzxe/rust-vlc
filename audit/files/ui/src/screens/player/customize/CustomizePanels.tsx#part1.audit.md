# AUDIT: ui/src/screens/player/customize/CustomizePanels.tsx#part1
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 421 (1–421 of 843) / 11 (useCustomizeMsg, CustomizePanelsProps, CustomizePanels [component: applyCompletion, dispatch, onEvent effect, layout state, moveTile, addToBand, removeFromBand, onTileKeyDown, closePanel, onDialogClose, focus-keeper effect, box JSX through tile onDrop-start]) |
| Chunk boundary | part1 = 1–421 (assignments.tsv; part2 = 422–843 owned by W15-A05 — the CustomizeBand edit preview + drag-into-band logic). The cut falls INSIDE the tile button's `onDrop` handler (415–426): the handler starts in my range and completes at 426; I audited it as a unit (read through 489 for context) and own its findings, flagged for A05's awareness. Also read for context (owned by others): panels.ts (the 31-tile catalog — not in any w15 list), toolbarItems.tsx (A01), capabilities.ts, toolbarLayout.ts. |
| File verdict | REAL (with one S2 dead-command seam) |
| Reachable from shipped entry point? | YES (app menu row "customize-player" dispatches `lumen:open-customize` — ControlBar.tsx:1037; PlayerScreen hosts CustomizePanels on that event; `/player` route → PlayerScreen). UI bundle builds (baseline #22); product caveat: no desktop binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -n '"video.record"\|"video.toggle-pip"\|"video.set-tone-mapping"' crates apps --type rust` → **0 matches** (the three commands exist in NO engine); `rg` same ids in `src/generated/commands.ts:24` (the CommandRequest union) → absent (cast bridge needed); `rg -n 'take-screenshot' crates` → present in 4 Rust files (screenshot tile is real); Read panels.ts (31 tiles: 15 engine-bound, 18 uiTile inert — counted from the catalog), toolbarItems.tsx (bar-side rendering incl. disabled-with-reason at 287-299 and the same cast at 110-113), capabilities.ts (gating RETIRED, header documents it), toolbarLayout.ts (`lumen.player.toolbar.v1` localStorage + `lumen:toolbar-changed` broadcast); `rg -n '"customize.title"\|"customize.save"' src/i18n/messages.ts` (661-668, 2139+: a17 keys registered → useCustomizeMsg fallbacks dead); `rg -n 'it\(' CustomizePanels.test.tsx` (42 tests incl. :180 "renders the full 31 against the demo listing — Record/Snapshot/PiP/HDR/Zoom included", :233 "UI-owned tiles never dispatch", :413 cancel/save persistence, :637/:664 focus keeper); Read demo-transport.ts:386-394 (unknown commands → `feature-not-exposed` "command not simulated"). Baseline: ui-vitest.log.txt 136/136 files, 1705 passed. |

## 1. One-paragraph verdict
The customize surface is the most honestly-instrumented dead-button exhibit in my assignment: its 31-tile catalog splits into 18 UI-owned palette items (inert by design, pinned by test :233), 10 tiles dispatching real wire commands that exist in both the generated catalog and the Rust engine (verified for take-screenshot), and **3 tiles (Record, PiP, HDR) that dispatch commands existing nowhere** — `video.record`/`video.toggle-pip`/`video.set-tone-mapping` have zero matches across `crates/` and `apps/`, are absent from the UI's own wire union (sent via a double `as unknown as CommandRequest` cast at 161-165), and their errors are swallowed with no disabled state and no user feedback (171-175) — dead buttons rendered as actionable, in the box AND in the real bar (toolbarItems.tsx renders the same tiles as active IconButtons), directly contradicting this file's header claim "never a dead control, never a fabricated one" (39). Second-order: the box renders the same inert tiles the bar renders disabled-with-reason (inconsistent honesty treatment), panels.ts's header still claims capability gating that capabilities.ts documents as retired, and the lane-seam resolver is dead in the now-standard way. Everything else is genuinely wired: seek/shuffle/repeat/rate/zoom dispatch real commands with engine-confirmed state (never optimistic — the applyCompletion/event mapping is real and test-pinned), DnD + keyboard equivalents + persistence are complete and tested (42 tests).

## 2. What actually works (verified)
— verified by test :217 ("seek tiles dispatch relative playback.seek targets (±10 s / ±15 s)") + generated/commands.ts:24 (`{ args: { target: SeekTarget }; cmd: "playback.seek" }` in the wire union) + baseline vitest 136/136 — the 4 seek tiles dispatch a real command with a real wire shape.
— verified by `rg 'take-screenshot' crates` → 4 hits (catalog/command.rs, catalog/ids.rs, playback-engine/snapshot.rs, catalog/event.rs) — the Snapshot tile's command exists in the engine, unlike F-001's trio.
— verified by test :242-299 (shuffle flips on engine report; repeat cycles engine-confirmed modes; playback-speed cycles from `playback.rate-changed` events) — the never-optimistic contract is behavioral, not aspirational.
— verified by test :413-495 (Cancel restores, Save persists through the `lumen.player.toolbar.v1` localStorage seam, broadcast fires, parse heals junk/dup/omissions) — UI-owned layout state genuinely persists (contrast: W11's settings-never-persist).
— verified by test :637/:664 — the h07-defect-2 focus keeper (Save-disables-under-pointer blur fix) executes and is pinned.

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/player/customize/CustomizePanels.tsx:158-165 (dispatch), 171-175 (swallow); data at panels.ts:130-161; bar-side twin at toolbarItems.tsx:110-113
Code:
```ts
      // Forward-declared commands (PiP, HDR) are not yet in the 1.D4 wire
      // union — the cast is the bridge until `cargo xtask codegen` regrows
      // the catalog (recorded in ui/NEXT-STEPS.md).
      const request = {
        cmd: tile.command,
        args,
        requestId: nextRequestId.current,
      } as unknown as CommandRequest;
      try {
        const response = await wire.send(request);
        if ("completed" in response.result) {
          applyCompletion(response.result.completed);
        }
        // Errors (feature-not-exposed etc.) leave tile state untouched —
        // honest no-change, no fabricated success.
      } catch {
        // Transport failure — same honest no-change policy.
      }
```
What is wrong: The Record, PiP and HDR tiles dispatch `video.record`, `video.toggle-pip`, `video.set-tone-mapping` — commands that (a) are absent from the UI's own generated wire union (necessitating the `as unknown as CommandRequest` double cast) and (b) have ZERO matches in the entire Rust engine (`rg` over `crates/` + `apps/` → nothing). In every lane the dispatch fails: the demo transport answers `feature-not-exposed` ("command not simulated", demo-transport.ts:386-394); a real shell would answer unknown-command. The error is swallowed at two levels (error-result ignored at 171-173, transport rejection at 173-175) and the tile renders as a fully actionable button — no disabled state, no reason label — so a user clicking Record/PiP/HDR (in the box, or placed in the real bar via toolbarItems.tsx:251-265 which renders the same tiles active) gets zero feedback, forever. This is the W8 `audio.set-eq` pattern: a UI affordance awaiting a backend that does not exist.
Why it matters: The file header claims "never a dead control, never a fabricated one" (36-39) — these ARE dead controls, shipped active-looking in the product surface. The forward-declaration is honestly commented and recorded in NEXT-STEPS.md, but the user-facing behavior is a silent no-op button, and the double cast deletes the type system's protection for every future tile (any typo'd command would compile).
What correct looks like: Render forward-declared tiles disabled-with-reason (the exact pattern toolbarItems.tsx:287-299 already uses for surface-less UI tiles) until the engine lists the command — doc 05 §3's own rule; or implement the three engine commands. The cast bridge belongs behind a single typed seam that fails loudly at compile time when codegen regrows.
Confidence: HIGH — Verification: read + rg (engine grep 0 matches; wire union read; demo transport read). Runtime behavior UNVERIFIED (no GUI binary — W14-A01), but both lanes' failure modes are static certainties.
### F-002 [S3]
Where: ui/src/screens/player/customize/CustomizePanels.tsx:428-440 (vs toolbarItems.tsx:287-299)
Code:
```ts
                    onClick={() => {
                      if (tile.control.kind === "action") {
                        void dispatch(tile, tile.control.args);
                      } else if (tile.control.kind === "toggle-action") {
                        void dispatch(tile, tile.control.argsFor(engineBools[tile.id] ?? null));
                      } else if (tile.control.kind === "cycle") {
                        void dispatch(
                          tile,
                          cycleArgs(tile.control, cycleSelections[tile.id] ?? null),
                        );
                      }
                      // kind "ui": the palette item is inert by design.
                    }}
```
What is wrong: In the box, the 18 UI-owned tiles (Quit, Favorite, VLC Settings, Go Home, the Name/Properties family, Replay, Chapters, Bookmark, Playlists…) render as ordinary enabled `<button>`s whose click does nothing (the comment's "inert by design"), with only a hover `title` drop-hint (387-389). The SAME tiles placed in the real bar render disabled-with-reason ("— unavailable", toolbarItems.tsx:290-299, "never theater"). Two surfaces of one catalog, two opposite honesty treatments.
Why it matters: A user who clicks Quit or Favorite in the box gets a silent no-op on an enabled-looking control — the exact theater the bar's treatment exists to avoid. The disabled-with-reason grammar is this codebase's documented standard (doc 08 §4.4); the box's palette exemption is asserted (test :233 pins "inert by design") but not defensible against its own sibling surface.
What correct looks like: Box tiles with no click action render `aria-disabled` with the reason in the title (still draggable — draggability and clickability are separate concerns), or the click performs the place-in-band action so the button is never a no-op.
Confidence: HIGH — Verification: read both surfaces + the pinning test.
### F-003 [S3]
Where: ui/src/screens/player/customize/panels.ts:4-8 (cross-file; contradicts capabilities.ts:3-9 and this file's 36-39)
Code:
```ts
//   * engine-bound tiles name the command they dispatch (capability-gated
//     in capabilities.ts — hidden until the engine lists the command, per
//     doc 05 §3 "no dead controls");
//   * UI-owned toolbar items (command: null) always render — they are
```
What is wrong: panels.ts's header claims engine-bound tiles are "capability-gated … hidden until the engine lists the command". FALSE at HEAD: capabilities.ts:3-9 documents the gating helpers (tileVisible/visibleTiles) were RETIRED by the h07-defect-4 ruling and the box renders the FULL catalog against ANY listing (this file's header 36-39 states the same; tests :180/:196 pin "any listing" / "the empty scaffold listing"). The record tile's own entry comment (panels.ts:131-132) repeats the stale claim ("hidden until an engine lists it").
Why it matters: The one file that defines the catalog states a gating policy that no longer exists — a maintainer reading panels.ts alone would believe dead commands are hidden (they are not; see F-001), which is precisely how dead controls survive reviews.
What correct looks like: panels.ts header rewritten to the capabilities.ts truth (catalog always renders; engine truth governs dispatch only).
Confidence: HIGH — Verification: read all three files; panels.ts appears unassigned in wave 15 (flagged for the orchestrator).
### F-004 [S4]
Where: ui/src/screens/player/customize/CustomizePanels.tsx:91-101
Code:
```ts
function useCustomizeMsg() {
  const m = useMsg();
  return useCallback(
    (key: CustomizeTile["labelKey"] | MessageKey): string =>
      m(key as MessageKey) ??
      a17Keys[key as keyof typeof a17Keys] ??
      gfixAKeys[key as keyof typeof gfixAKeys] ??
      key,
    [m],
  );
}
```
What is wrong: The same post-registration dead seam as ControlBar/ShortcutsScreen (17 repo copies): the a17/gfixA keys are already registered in messages.ts (661-668 union, 2139+ catalog), and useMsg is total (messages.ts:3016), so both `??` fallbacks can never fire. The trailing `?? key` (render the raw msgid) is a fourth dead layer. toolbarItems.tsx:63-74 compounds it with a FIVE-step variant.
Why it matters: Systemic dead indirection with stale "pre-registration" comments (86-89) — the cleanup debt is now duplicated per-file with diverging chain lengths.
What correct looks like: Direct `m(...)` calls (or one shared resolver while genuinely unregistered).
Confidence: HIGH — Verification: read messages.ts registration + totality.

## 4. Per-function table (my range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useCustomizeMsg` | 91-101 | REAL | S4 | F-004 dead fallback chain. |
| `CustomizePanelsProps` | 103-114 | REAL | — | transport override + onClose seam (`lumen:close-panel` event fallback). |
| `CustomizePanels.applyCompletion` | 138-152 | REAL | — | Bool arm finds the shuffle tile by command (O(n) over 31, fine); cycle arm delegates to shared cycleState. Engine truth only. |
| `CustomizePanels.dispatch` | 154-178 | REAL | S2 | F-001: double cast + swallowed errors for commands with no backend; the happy path (real commands) is correct. |
| engine-event effect | 184-203 | REAL | — | shuffle-set → engineBools; everything else → cycleState mapper; subscribe/unsubscribe clean. |
| layout/saved state + dirty | 206-211 | REAL | — | Two independent `loadToolbarLayout()` initializers (double parse, benign); dirty via structural equality. |
| `moveTile` | 226-238 | REAL | — | Splice reorder; insert-at-target semantic; no index bugs traced (A onto B → [B,A]; A onto C → [B,C,A]). |
| `addToBand`/`removeFromBand` | 241-252 | REAL | — | Idempotent guards. |
| `onTileKeyDown` | 255-272 | REAL | — | Delete/Backspace removes from band; Alt+←/→ reorders — keyboard parity for drag, preventDefault correct. |
| `closePanel`/`onDialogClose` | 274-289 | REAL | — | Escape closes via ✕-equivalent; scrim deliberately inert (frame grammar); event seam `lumen:close-panel`. |
| focus keeper effect | 299-315 | REAL | — | document focusout → timeout 0 → pull BODY focus back to the dialog; h07 defect 2 belt-and-suspenders; pinned by test :664. |
| box JSX + tile button (through onDrop) | 319-426(+context to 449) | REAL | S3/S2 | role=list/listitem; drag ghost = styled well (setDragImage optional-chained for jsdom); dataTransfer types; aria-pressed only for toggle kind, engine-confirmed; F-002 inert click; F-001 dispatch on click; onDrop: band-id → removeFromBand, else reorder — the 415-426 handler is mine despite the chunk cut (see boundary note). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :131/:180/:196 "renders the full 31 … any listing" | YES | Tile-count + order assertions against both a demo listing and an empty MockTransport listing — pins the always-render ruling (and makes F-001's dead tiles visibly deliberate). |
| :217 seek dispatches | YES | Asserts dispatched cmd + relative deltaMs args. |
| :233 "UI-owned tiles never dispatch" | YES | Pins the inertness F-002 critiques — the test enshrines the design, it does not evaluate it. |
| :242/:263/:284 shuffle/repeat/rate engine-confirmed cycling | YES | Drives FakeTransport completions/events and asserts the tile follows (not optimistically). |
| :317-:413 DnD/keyboard/persistence frames | YES | Drag simulations assert DOM wells/order; localStorage seam asserted incl. the broadcast. |
| :637/:664 focus keeper | YES | Body-focus death and recovery exercised. |
| Gap: NO test covers the record/pip/hdr dispatch outcome (feature-not-exposed → no-change) | — | The one behavior F-001 indicts is unpinned; a regression that made these tiles throw would pass the suite. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "engine-bound tiles dispatch their commands honestly … never a dead control, never a fabricated one" | CustomizePanels.tsx:36-39 header | FALSE for 3 of 15 engine-bound tiles | F-001: video.record/toggle-pip/set-tone-mapping exist in no engine; silent no-op click. |
| "the box renders the frame's FULL static catalog regardless of the listing (the h07-defect-4 ruling)" | CustomizePanels.tsx:37-39 | REAL | Tests :180/:196 pin it; capabilities.ts confirms the ruling. |
| "capability-gated in capabilities.ts — hidden until the engine lists the command" | panels.ts:4-8 | FALSE (stale) | F-003: gating retired (capabilities.ts:3-9). |
| "Save persists the arrangement … the localStorage seam the controls lane can consume via toolbarLayout.ts" | CustomizePanels.tsx:32-34 | REAL | toolbarLayout.ts:20 `lumen.player.toolbar.v1`; test :413/:483. |
| "cycle tiles dispatch the next option from ENGINE-CONFIRMED state only — never optimistic" | CustomizePanels.tsx:40-43 | REAL | cycleState mapping + tests :263/:284/:809-:848. |

## 7. Dead / duplicated / unreachable code
Dead: the `?? a17Keys ?? gfixAKeys ?? key` fallback chain (F-004); useCommandCapabilities has no product consumer (capabilities.ts:10-16, self-documented frozen API — noted, not mine). Duplicated: the cast-bridge dispatch is copy-pasted between CustomizePanels.tsx:154-178 and toolbarItems.tsx:107-126 (the second copy even hardcodes `requestId: 1`); the label-resolver seam ×2 with different chain lengths. Unreachable: F-001's three commands are unreachable by construction (no backend anywhere).

## 8. Dependencies & flags
Consumes: CUSTOMIZE_TILES/cycleArgs/tileById (panels.ts — unassigned in w15), cycleState (shared with the bar), toolbarLayout (localStorage + event seam), Dialog/Icon/IconButton/Button, i18n a17/gfixA keys (registered). No feature flags; transport injectable for tests (used heavily). The `as unknown as CommandRequest` cast is the file's only type-system breach.

## 9. Open questions for the next agent
- A05 (part2): the band preview's drop handling (422+) shares F-001's catalog — do the preview wells also render forward-declared tiles as active? (toolbarItems.tsx — A01 — already shown to.)
- Orchestrator: panels.ts and capabilities.ts appear UNASSIGNED in wave 15 — F-001's data and F-003's false header live there; assign or absorb.
- Does ui/NEXT-STEPS.md's codegen plan actually include the three commands? If not, the "bridge" comment is a promise with no owner (same family as the token-lane handoffs in player.css).
