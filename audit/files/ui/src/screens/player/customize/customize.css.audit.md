# AUDIT: ui/src/screens/player/customize/customize.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 626 / 66 rule blocks (CSS "functions" = selector groups) |
| File verdict | REAL-UNPROVEN (styles shipped + wired to live components; zero rendered verification of any of the ~60 frame-measured values) |
| Reachable from shipped entry point? | YES as web-UI source: `src/main.tsx → App.tsx:4 → PlayerScreen.tsx:109 import { CustomizePanels } from "./customize"` and `PlayerScreen → ControlBar.tsx → toolbarItems.tsx:60 import "./customize.css"`; `CustomizePanels.tsx:78` also imports it. CAVEAT: no GUI binary exists (W14: tauri host crate does not compile at HEAD; baseline #2 `build-desktop-tauri.log.txt` exit 101) and the UI mounts no `<canvas>` (W0) — reachability is source-level within the web bundle that `npm run build` compiles (baseline #22 OK), not a shipped desktop app. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "customize.css" src/` (importers: toolbarItems.tsx:60, CustomizePanels.tsx:78); `rg -ho "lumen-customize[a-z_-]*|lumen-toolbar-items[a-z_-]*" CustomizePanels.tsx toolbarItems.tsx | sort -u` (every class in this sheet is produced by TSX; conversely TSX also emits `__slot--left/--right` which have NO rules here — dead classnames, see F-004); `rg -n "lumen-scrim|lumen-dialog" src/components/surface/Surface.tsx` (scrim:236 → dialog child:245, so the `:has(> .lumen-dialog …)` hook matches); `rg -n "z-index" src/components/surface/surface.css` (scrim z20; band z21 wins); `rg -o "var(--[a-z-]*" customize.css … against src/tokens/generated/tokens.css` (all 24 external tokens defined); `rg -n "persistToolbarLayout" toolbarLayout.ts` (Save → localStorage TOOLBAR_STORAGE_KEY, UI-local only); `find e2e -type f` + `rg -n customize e2e/` (no e2e spec covers the modal); baseline #22/#23 (ui build OK, vitest 1705 pass) |

## 1. One-paragraph verdict
This is the best kind of CSS in the tree — every hard-coded pixel carries a frame citation, raw hexes are quarantined behind stylelint-disable with justification, the 4-4 dashed-well paint is factored into one gradient rule shared by three surfaces, and the layering trick (band z21 above scrim z20 so drag-drops reach it) is correct against surface.css:35. Every selector is produced by CustomizePanels.tsx/toolbarItems.tsx, and all 24 semantic tokens it consumes exist in generated tokens.css. But the sheet's entire premise — "pinned 1:1 per the g-wave Lead ruling" across ~60 measured values (94px header, 27×27 wells, 1048×655, 7.9% buffered bar, 2.15vh lift) — is enforced by nothing: no visual baseline covers the customize modal (the visual harness's own manifest defers in-panel states to "e2e dialogs" specs that do not exist), and the vitest suite runs in jsdom where CSS is never applied. The 1:1 claims are prose, not guarantees.

## 2. What actually works (verified)
— verified by `rg -ho "lumen-customize[a-z_-]*" src/screens/player/customize/*.tsx | sort -u` vs the sheet's selectors → all 40 distinct `lumen-customize*`/`lumen-toolbar-items*` rules in this sheet have producing JSX sites (e.g. `__well--27/--23/--60` at CustomizePanels.tsx:713-718, `[data-shifted]` at :815, `[data-drop-active]` at :655).
— verified by `rg -n "lumen-scrim|lumen-dialog" src/components/surface/Surface.tsx` → `.lumen-scrim` (:236) directly contains `.lumen-dialog ${className}` (:245) and CustomizePanels passes `className="lumen-customize__surface"` (:326), so the `:has(> …)` scrim-neutralization at customize.css:54 matches a real DOM shape.
— verified by token cross-check → all `var(--…)` references resolve in `src/tokens/generated/tokens.css:31-96` (e.g. `--color-surface-deep: #0e0e0e` matches the frame comment's "#0E0E0E").
— verified by baseline #22/#23 (`audit/runs/baseline/INDEX.md`: `npm run build` OK, `npm test` 1705 passed/1 skipped) → the sheet compiles into the bundle and CustomizePanels.test.tsx passes; class-name contract holds.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/player/customize/customize.css:1-29 (and the ~60 `frame:` citations throughout, e.g. 100, 132-136, 366-374, 473, 487)
Code:
```
 * MODAL ANATOMY (frame `Player - customize.svg`, measured @1920 and pinned
 * 1:1 per the g-wave Lead ruling):
```
What is wrong: the sheet's core claim — dimensions "pinned 1:1" to nine Figma frames — has zero automated enforcement. `e2e/visual/manifest.ts:12-14` explicitly excludes the customize modal from the screenshot-diff grid ("in-panel states, not routes — they belong to … e2e dialogs"), and `find e2e -type f` shows no dialog spec exists (only smoke/a11y/theme + visual baselines of routes). `CustomizePanels.test.tsx` asserts class names and behavior only; jsdom never applies CSS, so `npm test` (baseline #23, 1705 pass) proves nothing about a single one of these 60 values.
Why it matters: the Lead ruling "pinned 1:1" is a fidelity guarantee; any regression (a 94px header drifting to 84, a 27px well resized) passes every committed gate silently, and the guarantee rests entirely on the honesty of comments.
What correct looks like: a playwright dialog-state spec (open the modal via the `lumen:open-customize` seam PlayerScreen.tsx:158 exposes) with a committed baseline per frame state, or at minimum a unit test asserting the computed 94px/27px/45px values in a real-browser lane.
Confidence: HIGH — Verification: read (manifest.ts, e2e tree, CustomizePanels.test.tsx grep for `getComputedStyle|toMatchSnapshot` → no hits) + baseline #23 cited.

### F-002 [S3]
Where: ui/src/screens/player/customize/customize.css:302 (4.3vh lift) and 133 (grid gap math)
Code:
```
  margin-block-end: 4.3vh; /* frame: 23.25/1080 above center */
```
What is wrong: the lift is computed for the *default flex-centering* case and silently breaks whenever the Dialog primitive's centering changes or the viewport is short enough to hit `max-height: 86vh` (:70) — at 86vh cap the modal is clipped from the top by design while the 4.3vh push-down compensation still applies, so the "slightly high of center" invariant (23.25px @1080) holds only in the unconstrained case. Same class of issue at :133: `gap: 53px 4px` with the comment "row pitch 120 (tile 67 + 53)" — tile is 27px well + 13px gap + ≥27px 2-line label ≈ 67px, so the pitch depends on the label never wrapping to 3 lines; `-webkit-line-clamp: 2` (:214) enforces that, but any localization that makes labels 2-line at 12px in a narrower locale breaks the 120px pitch the frames measure. No guard, no test.
Why it matters: frame-measured geometry that depends on runtime content (localized labels, viewport caps) is not "pinned"; the deviation is invisible to every committed gate (F-001).
What correct looks like: fixed row heights (`grid-auto-rows`) instead of content-derived pitch, and a min-viewport guard documented for the lift.
Confidence: MEDIUM — Verification: read + line-clamp at :204-214 traced; no rendered check possible in this sandbox (UNVERIFIED rendering behavior).

### F-003 [S4]
Where: ui/src/screens/player/customize/customize.css:54-56
Code:
```
.lumen-scrim:has(> .lumen-dialog.lumen-customize__surface) {
  background: transparent;
}
```
What is wrong: reaching across the component boundary with `:has()` (the sheet's own comment, :50-53, admits "The shared Dialog primitive renders `.lumen-scrim` with no className seam of its own") — a structural coupling to Surface.tsx's DOM (`lumen-scrim` > `lumen-dialog`). If Surface ever inserts a wrapper (e.g. a portal focus-trap div) the selector silently stops matching and the scrim dims the player contrary to every customize frame. `:has()` support is fine for the target webviews; the fragility is the undocumented DOM contract.
Why it matters: silent-failure coupling between sheets; the failure mode contradicts a Lead-level design ruling (undimmed player behind the modal).
What correct looks like: a `data-`/className seam on the scrim owned by Surface.tsx (e.g. `scrimTone="none"` prop), or a comment in Surface.tsx marking the DOM shape as load-bearing for customize.css.
Confidence: HIGH — Verification: read Surface.tsx:236-245; selector matches today.

### F-004 [S4]
Where: ui/src/screens/player/customize/customize.css:175-177 vs CustomizePanels.tsx:740/807 (produced classnames with no rules)
Code:
```
.lumen-customize__tile-well,
.lumen-customize-band__well,
.lumen-customize-band__slot {
```
What is wrong: the reverse direction — CustomizePanels.tsx emits `lumen-customize-band__slot--left` / `--right` class names (grep over both TSX producers shows them) that have no corresponding rules in this sheet; they exist only to be flex-grow containers via inline `style={{ flexGrow: leftFlex … }}` (:740, :807). Harmless but the modifier names promise styling that lives in TSX inline styles — a naming seam that will rot.
Why it matters: dead classname surface invites future confusion ("where is --left styled?").
What correct looks like: either CSS rules owning the split-slot geometry or drop the modifier classes.
Confidence: HIGH — Verification: `rg "--slot--left|--slot--right"` → TSX only, zero CSS hits in this sheet.

## 4. Per-function table (selector groups; CSS has no functions — every rule block listed)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-customize, .lumen-customize-band` (well vars) | 36-41 | REAL | — | `--customize-well-ink` hex carries stylelint-disable + justification; light-mode remap at 622-625 overrides correctly (later sheet position + equal specificity → source order wins, verified ordering) |
| `.lumen-scrim:has(> …__surface)` | 54-56 | REAL | S4 | F-003 cross-boundary coupling |
| `.lumen-scrim .lumen-dialog…__surface` | 65-80 | REAL | — | 1048×655/92vw caps, blur 25px matches token `--blur-backdrop: 25px`; 4.3vh lift → F-002 |
| `.lumen-customize` | 82-89 | REAL | — | flex column shell |
| `__header` | 93-103 | REAL | — | 94px frame-pinned, unverified (F-001) |
| `__title` | 105-110 | REAL | — | tokens only |
| `__instruction` | 112-117 | REAL | — | 12px raw with citation; token deviation documented (#6B6B6B→#828282 WCAG ruling) |
| `__close` | 119-124 | REAL | — | absolute 11px offset; hit target sized by IconButton |
| `__box` | 128-138 | REAL | — | 9-col grid, scrollable; pitch math → F-002 |
| `__tile-slot` | 140-143 | REAL | — | thin wrapper |
| `__tile` | 145-158 | REAL | — | button reset, cursor grab |
| `__tile:focus-visible` / `:active` | 160-167 | REAL | — | a11y focus ring present |
| `__tile-well, band__well, band__slot` (dash paint) | 175-189 | REAL | — | 4-4 gradient dash factored once; correct per-edge sizes/positions |
| `__tile-well` | 191-199 | REAL | — | 27×27 frame-pinned |
| `__tile-label` | 203-215 | REAL | — | 2-line clamp |
| `__tile:hover` ×2 | 217-223 | REAL | — | hover re-ink via var override |
| `__tile[data-dragging]` ×2 | 232-239 | REAL | — | vacate grammar; data attr produced at CustomizePanels.tsx:380 |
| `__tile[data-drop-target]` | 242-246 | REAL | — | attr produced at :381 |
| `__footer` | 250-259 | REAL | — | space-between |
| `__cancel, __save` | 264-268 | REAL | — | shared Button components |
| `__cancel` | 270-274 | REAL | — | frame colors via tokens |
| `__footer …__save:disabled` | 280-288 | REAL | — | specificity claim "(0,3,0) out-ranks button.css `.lumen-button:disabled`" — verified: 2 classes + element-context = (0,3,0)? Actually `.lumen-customize__footer .lumen-customize__save:disabled` = 3 classes + 1 pseudo = (0,4,0); claim understates; harmless |
| `.lumen-customize-band` | 295-305 | REAL | — | fixed overlay; fallback 45px = controls.css:40 `min-height: 44px` + 1px border — consistent; runtime inline override at CustomizePanels.tsx:658-669 |
| `band__seekrow` | 312-324 | REAL | — | 54px promoted row, frame-pinned |
| `band__time` | 329-337 | REAL | — | tabular-nums |
| `band__row` / `__cluster` | 339-356 | REAL | — | layout only |
| `band__well` / `--tall` | 360-375 | REAL | — | 29×27/29×31 frame-pinned |
| `button.band__well` (+focus/hover) | 379-392 | REAL | — | interactive wells; focus-visible present |
| `band__well--play` (+::before, .lumen-icon) | 396-414 | REAL | — | 48×28 pill inset 2.5 in 53×33 |
| `band__well--wide`, `__volume-track` (+::after) | 417-438 | REAL | — | static 40% thumb — decorative preview, aria-hidden in TSX |
| `band__slot` | 442-448 | REAL | — | drop target |
| `band__seek`, `--row`, `__seek-buffered`, `__seek-played` | 453-488 | REAL | — | static preview values (7.9%/8px) documented as frame 0 stills |
| `band[data-drop-active]` ×2 | 492-500 | REAL | — | attr at TSX:655 |
| `band[data-dragging]` | 503-505 | REAL | — | attr at TSX:654 |
| `button.band__well[data-dragging]` | 509-511 | REAL | — | vacate grammar |
| `band__well--flat` | 518-522 | REAL | — | produced at TSX:705/719 |
| `band__topbarrun` (+first-child) | 527-536 | REAL | — | 1px pitch run |
| `band__well--27/--23/--60` | 538-548 | REAL | — | produced at TSX:713-718 |
| `band__well-mark` | 552-557 | REAL | — | 2×12 divider |
| `band__well--low` | 561-565 | REAL | — | produced at TSX:837 |
| `band__ghost` | 571-573 | REAL | — | produced at TSX:729 |
| `band__right` (+[data-shifted]) | 577-586 | REAL | — | shifted at TSX:815 |
| `.lumen-toolbar-items` / `__spacer` / `__divider` | 593-614 | REAL | — | consumer half; used in toolbarItems.tsx (live bar) |
| `:root[data-theme="light"]` remap | 622-625 | REAL | — | correct cascade position (documented "kept at the sheet's end") |

## 5. Test honesty (tests exist for the component, not this sheet)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| CustomizePanels.test.tsx queries (`.lumen-customize-band__well--flat` at :560, `--low` at :609, well counts at :583) | NO for any styling breakage | jsdom does not apply CSS; these assert DOM classnames only. They DO fail if classnames drift from the TSX — a naming contract test, not a style test |
| e2e/visual (route grid) | NO for this sheet | manifest.ts:12-14 excludes in-panel dialogs; no baseline image exists for the customize modal |
| baseline `npm test` (1705 pass) | NO for this sheet | same jsdom limitation |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "pinned 1:1 per the g-wave Lead ruling" (issue #62 fix) | customize.css:2-5 comment; CustomizePanels.tsx:1-18 header | REAL-UNPROVEN | values present in code; zero rendered verification exists (F-001) |
| "Semantic tokens only (doc 08 §3.2); raw values appear solely for frame-measured dimensions/colors with no token" | customize.css:27-29 | REAL | verified: only raw values are frame-cited px/2 hexes behind stylelint-disable (:38-39); everything else is var() |
| "It sits ABOVE the Dialog scrim (z21) so drops reach it" | customize.css:24-25 vs surface.css:35 (z20) | REAL | layering verified by read |

## 7. Dead / duplicated / unreachable code
No dead selectors in this sheet (all 40 classnames produced — verified by two-way grep, see Commands). Reverse dead code: TSX emits `__slot--left/--right` with no rules (F-004). No duplication: the dash paint is a single shared rule.

## 8. Dependencies & flags
- Imports nothing; consumed via Vite CSS bundling from two TSX importers.
- Tokens: 24 distinct `var(--…)` all defined in `src/tokens/generated/tokens.css` (generated by theme-tokens crate — the crate's generated CSS is in-scope for this wave).
- `stylelint-disable` comments imply a stylelint config; `npm run lint:css` (package.json:55) was NOT in the baseline run set (INDEX.md #21-24) — the disables are unexercised in committed evidence.

## 9. Open questions for the next agent
- CustomizePanels.test.tsx and toolbarItems.tsx belong to other agents: does any test cover the `persistToolbarLayout` localStorage seam end-to-end (Save → reload → restored band)? W11 recorded "settings never persist" — toolbar layout DOES persist to localStorage (toolbarLayout.ts:79-82); the contradiction (or scope difference) is worth pinning.
- Does the visual harness ever grow the "e2e dialogs" lane manifest.ts promises? If yes, F-001 downgrades to S4.
