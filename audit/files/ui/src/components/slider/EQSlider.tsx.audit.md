# AUDIT: ui/src/components/slider/EQSlider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 46 / 1 type (EQSliderProps) + EQSlider (forwardRef) + displayName |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web UI): production consumers GraphicEQ.tsx:37,238 (the equalizer tool's vertical fader panel), ToolSliderRow.tsx / SliderRow.tsx (dynamics + shared tool rows, horizontal); barrel index.ts exports it. The equalizer tool lives in the player tools dialog. Behaviorally tested (EQSlider.test.tsx, 11 tests, baseline vitest row 23). Ceiling: no GUI binary at HEAD (W14). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | Read of EQSlider.tsx + EQSlider.test.tsx (181 lines) + slider.css EQ sections (:168-210 horizontal, :373-405 vertical) + interaction.ts (PageUp/PageDown :161-164) + Slider.tsx props; rg census EQSlider over ui/src (9 files: 3 production consumers + barrel + css/states.md/tests); vertical scrub math recomputed (y=300/328 → −16.58 → quantize → −17 ✓). |

## 1. One-paragraph verdict
A transparent 46-line wrapper that earns a zero-finding verdict the hard way: every docstring claim traces to verifiable fact. The frame anatomy (16px rx6 track on the #090909 surface well, flat #371F06 accent-tinted fill 12px rx4 inset 2, 8×24 rx4 accent-strong handle; vertical = 24×8) matches slider.css's .lumen-eq-slider rules declaration-for-declaration; the "interaction contract is the shared slider engine" claim is implemented in interaction.ts (arrows/Shift×10/PageUp/PageDown/Home/End all present, :161-164); the "value chip rendered by CONSUMERS" claim is pinned by a test asserting the slider renders no text of its own (textContent === "" — test :47); and the wrapper's spread hygiene is correct (orientation/className destructured out, rest cannot clobber them). The companion test suite is the strongest in my wave: two pixel-math tests parse the CSS source and prove the fill/handle flush identity at p ∈ {0,20,50,80,100} for BOTH axes with 1e-9 tolerance — the kind of test that fails on any geometry regression (it was written to pin a live-measured 2.8px overhang bug, w26.k03). No defects found; exactly what was checked is listed below.

## 2. What actually works (verified)
- Wrapper transparency: `Omit<SliderProps, "orientation"> & { orientation?: … }` re-exports the base contract additively (default "horizontal" — callers unchanged); orientation/className destructured before `{...rest}` so consumers cannot accidentally override the class composition.
- Frame anatomy claims ↔ slider.css: horizontal track `height:16px`/rx6/surface (:173-177), fill `top:2px bottom:2px left:2px` + `(100%−4px)·p` + accent-tinted + radius-inset rx4 (:183-191), thumb 8×24 radius-inset accent-strong with the L8 flush travel (:193-210); vertical mirror at :373-405 (24px hit width, 16px track, fill anchored bottom inset 2, thumb left calc(50%−12px), height 8 — frame 24×8).
- Keyboard contract claims: PageUp/PageDown coarse EXISTS in the shared engine (interaction.ts:161-164) — the docstring's contract is real, not aspirational; the EQ test suite pins arrows/Shift×10/Home/End (:72-84) and the vertical Y-axis scrub with exact lattice math (:108-130, recomputed independently: (328−300)/328·40−20 = −16.58 → quantize → −17 ✓).
- Live-then-commit: test :51-70 pins the call order `["live:5","live:15","commit:15"]` + data-dragging lifecycle — doc 08 §5's contract is behaviorally enforced.
- Chip-outside claim: test :47 (`slider.textContent === ""`) — the primitive renders no copy; consumers (GraphicEQ) own the chip.
- ARIA surface: role=slider + aria-orientation/min/max/now/text + tabindex roving (tests :24-37, :99-106); aria-label flows from callers via rest (the wrapper adds no literals — i18n clean).
- Disabled semantics: aria-disabled + tabindex −1 + pointer/keyboard inert (test :86-97).
- Consumers real: GraphicEQ.tsx (vertical fader panel), ToolSliderRow/SliderRow (horizontal rows) — census over ui/src; barrel export present.

## 3. Findings
None. Checklist coverage for the zero-finding claim:
- Fakeness: no stubs/constants/ignored params — the wrapper forwards everything (verified by reading every line).
- Correctness: spread order audited (no clobberable props); clamp/quantize live in the engine (interaction.ts read); vertical math recomputed by hand (−17/+17 ✓).
- Performance: no state/effects in the wrapper — zero render overhead beyond one className join.
- Tests: 11/11 would fail on regression (pixel-math tests mutate-proof the CSS geometry; the vacuous-test class from the dynamics harness does NOT apply here — every test asserts).
- Hygiene: displayName + named forwardRef fn; no literals; no dead exports (3 production consumers); no duplication (anatomy lives once in slider.css, documented in states.md :39).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| EQSliderProps | 20-28 | REAL | — | Omit+re-add additive pattern; orientation documented with default |
| EQSlider (forwardRef) | 30-44 | REAL | — | Transparent passthrough; className composition; no own logic |
| displayName | 46 | REAL | — | — |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "exposes horizontal slider ARIA over the band range" :24 | YES | Full ARIA attr set pinned |
| "formats aria-valuetext through formatValue" :34 | YES | formatValue seam |
| "renders the pill anatomy … chip stays outside" :39 | YES | Class hooks + track/fill/thumb nodes + textContent "" + CSS var --lumen-slider-fill=50 |
| "scrubs live and commits once on release" :51 | YES | Exact call-order array |
| "keyboard: arrows step, Shift coarse, Home/End" :72 | YES | Value assertions per key |
| "disabled means disabled" :86 | YES | aria-disabled + tabindex −1 + both input modes inert |
| "orientation=vertical renders the A2 fader axis" :99 | YES | Both classes + aria-orientation |
| "vertical faders scrub on the Y axis" :108 | YES | Lattice-snapped values (math recomputed) |
| pixel-math flush tests :135, :166 | YES | Parse the CSS source, assert the L8 travel grammar + flush identity at 5 sample points, 1e-9 tolerance — any geometry edit that breaks flush fails |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "16px rx6 track on the surface well (#090909), FLAT accent-tinted fill (#371F06, 12px rx4 inset 2), 8×24 rx4 accent-strong handle" | :8-11 | REAL | slider.css:168-210 matches every number; hexes == token values |
| "horizontal in the tool rows, orientation=vertical in the A2 fader panel (24×8 handle)" | :10-11 | REAL | Consumers census (ToolSliderRow/SliderRow horizontal; GraphicEQ vertical); CSS :373-405 (thumb height 8) |
| "The VALUE CHIP … is rendered by CONSUMERS, outside this slider" | :13-14 | REAL | Test :47 textContent ""; GraphicEQ :223 owns the chip |
| "Interaction contract is the shared slider engine: arrows ±step (Shift ×10), PageUp/PageDown coarse, Home/End bounds, live-then-commit, fully controlled" | :16-18 | REAL | interaction.ts:161-164 + Slider.tsx:48-49 + tests :51-84 |
| "SVG-measured (w26.a20 §A1, EQ.svg / Audio - Equalizer.svg …)" | :7-8 | REAL-UNPROVEN | The measurement provenance is lane-log prose; the RESULT is pinned by the pixel-math tests (geometry verified against the CSS, not the SVG) |
## 7. Dead / duplicated / unreachable code
- None. The wrapper adds the one thing the base Slider lacks (the EQ class scope + additive orientation); anatomy lives once.
## 8. Dependencies & flags
Slider (the shared engine), slider.css (.lumen-eq-slider scope), barrel index.ts. No feature flags. Note the engine-level files (Slider.tsx, interaction.ts) belong to other agents — this audit covers the wrapper + its CSS scope + its tests only; the flush-identity pins make the wrapper's geometry contract self-defending.
## 9. Open questions for the next agent
- GraphicEQ.tsx (equalizer tool, unassigned to me): the W8 pin (audio.set-eq awaits a nonexistent backend) lands there — the slider commits values into whatever seam the tool wires; my audit proves the primitive's contract, not the tool's backend reach.
- If slider.css is ever refactored to shorthand `inset:` in the horizontal scope, the vertical pixel-math test's `inset` regex (`/^auto\s+…$/`) is the fragile edge — it parses exactly the current notation.
