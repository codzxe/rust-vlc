# AUDIT: ui/src/screens/player/customize/CustomizePanels.tsx#part2 (lines 422–842)
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 421 (chunk 422–842 of 843) / 1 component + 1 hook + 2 helpers + 11 render branches |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle: PlayerScreen.tsx mounts CustomizePanels per-open on the `lumen:open-customize` window event (PlayerScreen.tsx:68-72); the real ControlBar consumes its output via toolbarItems.tsx:135 `useToolbarLayout()`. NO shipped binary mounts it (W14: desktop-tauri E0425). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | sed read of Surface.tsx focus trap (:139-193) + toolbarLayout.ts (full) + toolbarItems.tsx consumers; rg slot-modifier CSS/test-hook census; test-file read (CustomizePanels.test.tsx frame-variant suites); customize.css band-positioning check. |

## 1. One-paragraph verdict
Part 2 is the customize surface's footer, the live control-band edit overlay, and its geometry/variant machinery — and it is genuinely wired end-to-end: Save persists the arrangement to localStorage AND broadcasts `lumen:toolbar-changed`, which the real toolbar's `useToolbarLayout()` (toolbarItems.tsx:135) re-reads live — verified consumer chain; engine tiles dispatch real catalog commands with engine-confirmed state only (never optimistic), and the frame-variant state machine (bandVariant + SLOT_FLEX) matches its six-frame documentation exactly (f2 866/551, f3 729/551, f4 563/551, f5 526/499 — all cross-checked against the frame-map comments). The one real defect: the placed band wells are interactive buttons rendered OUTSIDE the Dialog's focus trap (CustomizeBand is a sibling of `<Dialog>`, :497), so while the modal is open Tab can never reach them — the trap takes over Tab entirely and cycles only the dialog's descendants (Surface.tsx:164-190) — yet the module doc (:612-614) and a test name (CustomizePanels.test.tsx:383) both call this click path "the keyboard-safe path back into the box". The actual keyboard path is the box tile's Delete/Backspace (part1's onTileKeyDown). Everything else verified clean, including the jsdom 0×0 geometry fallback and the decorative preview chrome being aria-hidden.

## 2. What actually works (verified)
- Save wiring: `setSaved(layout); persistToolbarLayout(layout)` (:472-473) → toolbarLayout.ts:79-90 writes `lumen.player.toolbar.v1` and dispatches `lumen:toolbar-changed` → useToolbarLayout (toolbarLayout.ts:117-129) re-reads on that event AND cross-tab `storage` → consumed by the real bar (toolbarItems.tsx:135). The comment at :470-471 is accurate.
- Variant machine: `bandVariant` (:586-593) maps dirty/placed/dragging to f0-f5 exactly as documented (:566-581); SLOT_FLEX values match the frame-measured comments digit-for-digit (:599-606).
- Geometry hook: `useBandGeometry` (:523-560) measures `.app-shell__strip` / `.lumen-player__controls`, guards ResizeObserver absence and jsdom 0×0 rects (CSS fallback at customize.css:295-298 `position:fixed; bottom:0`), cleans up observer + listener.
- Honest preview chrome: every fixed well/cluster/slot is `aria-hidden="true"` (:691, :705, :712, :741, :749, :794, :815, :837) — decorative mock, real controls stay underneath; the hardcoded preview timecodes "00:02:10"/"00:25:40" (:692/:697/:795/:800) live only inside that aria-hidden chrome and are disclosed as frame-pinned.
- Focus recovery on Save (:474-479): synchronous `closeRef.current?.focus()` before the state update disables the button — order is correct in React's batching model; belt-and-suspenders focusout keeper lives in part1.
- Tests (CustomizePanels.test.tsx:495-575) pin the f0-f5 compositions structurally — slot counts, seek-row promotion, DOM order of the f3 flat well (`compareDocumentPosition`) — behavioral, would fail on regression. Baseline vitest passed them (INDEX.md row 23).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/customize/CustomizePanels.tsx:608-614 (claim), :765-786 (buttons), :497-509 (mount outside the dialog)
Code:
```
 * are the preview chrome of controls this lane does not own (aria-hidden);
 * placed wells are real buttons (click = remove — the keyboard-safe path
 * back into the box). The seek row (frames -1..-6) rides ABOVE the real
```
What is wrong: The placed wells ARE real buttons (with proper aria-labels, :772) but they are rendered by `<CustomizeBand>` (:497), a sibling AFTER `</Dialog>` (:486) — outside the Dialog's focus trap. The trap "takes over Tab entirely (preventDefault + explicit move)" and cycles only `node.querySelectorAll(FOCUSABLE_SELECTOR)` — the dialog node's descendants (Surface.tsx:164-190). While the customize modal is open (the only time these buttons exist — the whole fragment returns null when closed, :317), Tab can NEVER focus them. The comment (and the test name at CustomizePanels.test.tsx:383, "clicking a placed well removes it (the keyboard-safe path)") misattribute keyboard safety to a control keyboard users cannot reach.
Why it matters: A keyboard-only user who places a tile and wants it gone must discover the box tile's Delete/Backspace path (part1, :255-272) — the documented affordance is a false lead. Screen-reader browse mode can reach the buttons (they are not aria-hidden, correctly so since they act), but sequential focus navigation cannot — an inconsistent a11y story on an otherwise carefully trapped surface.
What correct looks like: Either render the interactive placed wells inside the Dialog's trapped tree (visually positioned over the band via portal-with-trap or trap extension), or fix the comment/test name to point at the box-tile Delete path as THE keyboard path.
Confidence: HIGH — Verification: read Surface.tsx trap (Tab handler cycles dialog descendants only) + read the mount structure; no DOM assertion covers Tab reachability of band wells (tests use fireEvent.click, which bypasses focus).

## 4. Per-function table (chunk 422–842)
| Function/branch | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Tile onClick dispatch branches | 428-440 | REAL | — | action/toggle/cycle dispatch via wire; kind "ui" inert by design (disclosed :439) — click no-op on ui tiles is a documented palette affordance (drag/reorder are the interactions) |
| Tile well/label render | 442-449 | REAL | — | data-in-toolbar/dragging/drop-target hooks consumed by customize.css |
| Footer Cancel | 455-463 | REAL | — | setLayout(saved) reverts session edits; label from central catalog (open.cancel) |
| Footer Save | 464-484 | REAL | — | Persist + broadcast verified (see §2); disabled-while-clean matches frame; focus recovery order correct |
| CustomizeBand invocation | 497-509 | REAL | S3 | F-001: mounted outside the Dialog trap |
| BandGeometry / useBandGeometry | 514-560 | REAL | — | jsdom 0×0 guard; ResizeObserver guard; cleanup correct; no re-query if band absent at mount (acceptable: strip exists in app shell) |
| BandVariant / bandVariant | 583-593 | REAL | — | Pure; maps exactly to documented f0-f5 |
| SLOT_FLEX | 601-606 | REAL | — | Frame ratios verified vs comments |
| CustomizeBand — seekrow | 683-699 | REAL | — | aria-hidden; frame-pinned timecodes disclosed |
| f3 flat well / f4-f5 topbarrun / f5 ghost | 701-732 | REAL | — | All aria-hidden decorative; ghost mirrors setDragImage well |
| Split slots | 737-743, 805-810 | REAL | — | `--left/--right` modifiers have NO css rules — they are test DOM hooks (test :540-541/:562); geometry via inline flexGrow/flexBasis over base `.lumen-customize-band__slot` (customize.css:442 flex 1 1 auto) |
| Transport cluster | 745-759 | REAL | — | aria-hidden preview; data-centered for f2+ |
| Placed wells | 761-786 | REAL | S3 | F-001; real buttons w/ aria-label template replace |
| Trailing slots (f0 live preview / f1 empty / split) | 788-810 | REAL | — | f0 carries the in-slot preview; all aria-hidden |
| Right cluster + f5 far-right well | 812-838 | REAL | — | aria-hidden; data-shifted for f5 (51px frame shift) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| f0-f5 variant suites (:495-575) | YES | Structural pins (slot counts, seekrow presence, DOM order via compareDocumentPosition) — real regressions break them |
| "clicking a placed well removes it (the keyboard-safe path); Delete key too" (:383) | YES for behavior, NO for the keyboard claim | fireEvent.click bypasses focus — cannot detect Tab-unreachability (F-001); the Delete-key half genuinely exercises the box-tile path |
| "Cancel restores…; Save persists (localStorage seam)" (:413), "reorder + Save persists" (:444) | YES | Asserts storage contents + dirty transitions |
| "persistToolbarLayout broadcasts the lumen:toolbar-changed seam" (:483) | YES | Asserts the event fired (spy) |
| dispatch-spy suites (:216-297) | YES | Transport stub records commands; engine-confirmed flip via emitted events asserted |
| "UI-owned tiles never dispatch (inert palette items by design)" (:233) | YES | Honest no-dispatch pin |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Save persists the arrangement (…the localStorage seam the controls lane can consume via toolbarLayout.ts)" | module header :32-34 (part1, chunk cross-ref) | REAL | toolbarLayout.ts:79-90; consumer toolbarItems.tsx:135 |
| "cycle tiles … dispatch the next option from ENGINE-CONFIRMED state only — never optimistic" | :40-41 (part1) | REAL | applyCompletion only on completion; events fold via applyCycleEvent (part1 :184-203); tests :263-297 |
| "placed wells are real buttons (click = remove — the keyboard-safe path…)" | :612-614 | FALSE (keyboard attribution) | F-001 — outside the focus trap |
| "the overlay extends 54px above the measured strip" | :615-617 | cross-ref | customize.css (not audited by me) |
| Band geometry "null → CSS fallback" | :514, :536-539 | REAL | customize.css:295-298 fixed/bottom fallback |

## 7. Dead / duplicated / unreachable code
- `lumen-customize-band__slot--left/--right` (no CSS rules) — intentional test DOM hooks, not dead (test :540-541/:562 query them); noted so a future CSS sweep does not flag them as orphans.
- Nothing else: every band well class maps to a customize.css rule; every preview chrome branch is frame-documented.

## 8. Dependencies & flags
client transport (useClient/CommandTransport), cycleState.ts (shared with the real bar), panels.ts (CUSTOMIZE_TILES catalog), toolbarLayout.ts (localStorage + CustomEvent seam), components/surface Dialog (focus trap — the F-001 boundary), Icon. The forward-declared-command cast (`as unknown as CommandRequest`, part1 :161-165) is shared with toolbarItems.tsx:111 (same pattern comment) — part1/A09 territory.

## 9. Open questions for the next agent
- Part1 (W15-A09) owns the dispatch cast bridge (:161-165) and onTileKeyDown (the REAL keyboard path) — F-001's fix likely touches their chunk; cross-count once.
- Surface.tsx (A01's file this wave) owns the trap — my F-001 cites it as the mechanism, not as a defect of Surface.tsx.
- The `?:` on `setDragImage` (:402, :778) is jsdom-safe — fine; if the visual harness ever needs the native ghost, note it suppresses it.
