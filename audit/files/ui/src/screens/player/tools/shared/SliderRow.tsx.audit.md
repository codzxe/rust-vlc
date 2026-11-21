# AUDIT: ui/src/screens/player/tools/shared/SliderRow.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 144 / 2 (SliderRowInner, memo wrapper) + reviewed the geometry chain it depends on: Slider.tsx paint vars, slider.css thumb/fill rules, dynamics.css tool scope |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: PlayerScreen → ControlBar (app menu "customize-player" / tools buttons) → `openToolsDialog` event (ToolsDialog.tsx:63) → ToolsDialog mounts CompressorTool/SpatializerTool/StereoWidenerTool (dynamics) + EssentialTool/ColorsTool (video) → ToolSliderRow/VideoSliderRow → SliderRow; tests for all five card hosts green in baseline ui-vitest.log.txt) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read full file + Slider.tsx + dynamics.css:156-249 + slider.css:45-108 · `rg -n "SliderRow" ui/src` (both lane wrappers + their card hosts) · `rg -n "client.send\|dispatch\|useClient\|cmd:" ui/src/screens/player/tools` (ZERO engine sends in the whole tools tree — the "no engine yet" comment is literal) · `rg -n "covered\|staged" tools/` (staged markers + the covered-tick test pins) · geometry arithmetic on the 328px track (see F-001) · baseline ui-vitest.log.txt (1705 passed incl. EssentialTool/CompressorTool/SpatializerTool/StereoWidenerTool/ColorsTool tests) |

## 1. One-paragraph verdict
The consolidation story checks out: one shared row, two thin lane wrappers, and the previously-drifted video copy's 1.7px tick error is fixed by construction (both lanes now render this file's fill-domain `calc(2px + (100% - 4px)·p)` formula — regression-pinned by EssentialTool.test.tsx:84-115 down to jsdom's canonical calc serialization). The row is honest about what it is: fully-controlled local state, i18n through the lane's resolver, memoized so a drag on one row does not re-render its siblings, and the entire tools tree it serves carries visible staged markers (`data-staged-tool` + a `role="note"` line in every card) disclosing that no engine command exists — so the "dead sliders" hostile question is answered DISCLOSED-dead, not disguised-dead. The single defect is a cosmetic geometry mis-anchor: the covered-tick band compares the tick's fill-domain percent against `percent` symmetrically (±1.5%), but the thumb's actual span is [fill_edge−8px, fill_edge] — shifted one half-thumb-width left of that anchor — so a tick landing under the thumb's left half (t ∈ [p−2.47%, p−1.5%)) is NOT hidden and, because the tick overlay sits at z-index 2 ABOVE the slider (z-index 1, dynamics.css:170,228), it paints OVER the thumb, inverting the file's own documented frame z-order ("thumb paints last").

## 2. What actually works (verified)
— The deduplication is real: ToolSliderRow.tsx:43 and VideoSliderRow.tsx:43 are both one-line compositions of this row (`<SliderRow row={row} value={value} onChange={onChange} resolveLabel={m} />`), so the pre-k10 drift the header describes cannot recur.
— The tick domain formula is frame-verified and regression-pinned: EssentialTool.test.tsx:99-108 asserts `left: calc(2px + 0.1 * (100% - 4px))` for gamma tick 1 and explicitly rejects the pre-fix raw-percent form — the would-fail-if-broken bar is met.
— The Slider wiring is correct: both `onChangeLive` and `onCommit` drive the same sink (:99-100), so the covered computation's `percent` (from `value`) stays in sync with the painted thumb (`dragValue ?? value` in Slider.tsx:79) during drags; `aria-label` + `formatValue` give every row a name and valuetext (pinned per-card, e.g. EssentialTool.test.tsx:117-130).
— `React.memo` claim is honest and load-bearing: DynamicsCard.tsx:38 documents stabilizing the onChange closures precisely so this memo works.
— The dead-engine state is disclosed in the UI itself: every host card carries `data-staged-tool` + `data-pending-command` and renders a visible staged note (DynamicsCard.tsx:91,138-139; VideoCard.tsx:68,100-101; HdrTool.tsx:63,74-75), test-pinned in all five host test files — the repo's honest-staging pattern, applied consistently.
— Degenerate-range guard: `range > 0` (:83) keeps min==max rows from dividing by zero (percent → 0).

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/player/tools/shared/SliderRow.tsx:36-39, 119 (against dynamics.css:191-206, 225-249)
Code:
```
/** Thumb half-width in track-percent (8px thumb on a 328px track ≈ ±1.2%,
 * padded to 1.5%) — the tick under the thumb is covered, per the frames'
 * z-order (thumb paints last). */
const COVERED_TICK_BAND = 1.5;
```
```
              const covered = Math.abs(left - percent) <= COVERED_TICK_BAND;
```
What is wrong: the band is anchored on the wrong point. The comment assumes the thumb is CENTERED at `percent` p, but the CSS places the thumb's RIGHT edge flush with the fill edge: `left: calc((100% - 4px)·p/100 - 6px)`, width 8 (dynamics.css:191-206) → the thumb spans [fill_edge−8, fill_edge], i.e. its center sits at p−0.62% on the 324px domain, not at p. The tick (2px wide, left edge at 2px+(100%−4px)·t/100) is FULLY under the thumb for t ∈ [p−2.47%, p−0.62%], but the symmetric code band hides only t ∈ [p−1.5%, p+1.5%]. Two consequences: (a) ticks at t ∈ [p−2.47%, p−1.5%) sit fully under the thumb's left half, are NOT marked covered, and — because the tick overlay is z-index 2 over the slider's z-index 1 (dynamics.css:170,228) — paint OVER the thumb, inverting the documented z-order ("thumb paints last"); (b) ticks at t ∈ (p, p+1.5%] are hidden while actually sitting right of the thumb — benign, and in fact what makes the "default sits exactly on its tick" case look right.
Why it matters: purely cosmetic and transient (visible only while the thumb rests or passes in the ~3px window left of a tick), but it contradicts the file's own stated frame contract, and the test that pins covering (SpatializerTool.test.tsx:77-80, "Each default sits exactly on its tick — covered by the thumb") pins exactly the benign t==p case, so the inverted window is untested and would not fail any test today.
What correct looks like: anchor the band on the thumb center: `const covered = Math.abs(left - (percent - 0.6)) <= COVERED_TICK_BAND;` (or compute against the thumb's left edge in percent units) — one-line fix; add a test with the value parked one half-thumb-width left of a tick.
Confidence: HIGH on the geometry (arithmetic from the quoted CSS rules, both elements verified to share the same width domain: `.dynamics-row__slider{position:relative}` + `.lumen-slider__track{left:0;right:0}`); the visual artifact is UNVERIFIED (no runnable browser here).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SliderRowInner | 75-137 | REAL | S4 | degenerate-range guard; ticks `i/(n+1)` on the fill domain (frame-verified); covered band F-001 mis-anchor; aria-label + valuetext via row.format |
| SliderRow (memo) | 144 | REAL | — | memo correct given host-stabilized props (DynamicsCard.tsx:38); `as typeof` preserves the generic signature |
| — dependency: Slider primitive | components/slider/Slider.tsx | REAL | — | fully controlled; paint vars `--lumen-slider-fill/thumb` = toPercent; verified against slider.css + dynamics.css rules |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| EssentialTool.test.tsx:43 rows in frame order, 8 sliders | YES | data-row order assertions |
| :65 per-row tick counts | YES | exact counts per frame row |
| :84 tick left on the 2px-inset fill domain | YES | asserts the calc string + rejects the pre-fix raw-percent form (the drift regression) |
| :117 staged ranges + valuetext | YES | aria attributes incl. w26.i08 calibration values |
| :163 staged markers (all five host cards) | YES | data-staged-tool + note text |
| SpatializerTool.test.tsx:77 defaults covered | PARTIAL | pins tick--covered presence at t==p only — the F-001 window (thumb left half) is untested |
| CompressorTool/StereoWidenerTool/ColorsTool suites | YES | same pattern per card (ranges, local commits, staged markers) |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped). Tests mock no engine — correctly, because no engine path exists in the tools tree (verified by grep).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ToolSliderRow and VideoSliderRow were 1:1 copies that had ALREADY drifted … this module is the ONE row" | header :1-5 | REAL | both wrappers are one-line compositions (:43 in each) |
| "the tick under the thumb is covered, per the frames' z-order (thumb paints last)" | :37-38 | FALSE in the window t∈[p−2.47%, p−1.5%) | F-001: uncovered ticks paint over the thumb (overlay z2 > slider z1) |
| "Live + commit sink (both drive the same local state — no engine yet)" | :68 | REAL | rg: zero sends/dispatches in ui/src/screens/player/tools; visible staged notes in every host card |
| "with a stable row def + value + sink, a drag on one row no longer re-renders the other rows" | :28-30 | REAL-UNPROVEN | memo is correct and hosts stabilize closures (DynamicsCard.tsx:38); no render-count test exists for the cross-row claim |
| "the default value sitting exactly on one [tick]" | :57-58 | REAL | SpatializerTool.test.tsx:77-80 (covered class present at defaults) |

## 7. Dead / duplicated / unreachable code
- None. The module exists precisely to remove duplication and does.

## 8. Dependencies & flags
- components/slider (Slider), React memo. Class contract on dynamics.css (`dynamics-row*`, `dynamics-slider*`) — the "class names are the cross-lane contract" claim (:18-20) verified: video.css composes onto the same dynamics classes (per VideoCard.tsx:15-20 and the shared staged-note rule).
- No engine dependency, no feature flags — the entire tools tree is local-state-only by design, disclosed via staged markers.

## 9. Open questions for the next agent
- dynamics.css and video.css themselves are not in my file set — the F-001 fix touches both this file and possibly the overlay z-order; whoever audits dynamics.css should re-derive the band arithmetic.
- The cross-row memo claim (no re-render of sibling rows) has no test — a render-count test would pin it.
