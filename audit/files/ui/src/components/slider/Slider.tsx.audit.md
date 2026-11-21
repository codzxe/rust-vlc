# AUDIT: ui/src/components/slider/Slider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 115 / 1 component (Slider, forwardRef) + props interface; logic delegated to `useSliderInteraction`/`toPercent` (interaction.ts — asset-group g003, another agent) |
| File verdict | REAL (thin, correct, and backed by the deepest test file in my sample: 17 tests incl. CSS-parsing pixel-math geometry proofs) |
| Reachable from shipped entry point? | YES as web-UI source: 7 production consumers (SettingWidgets.tsx, areas/subtitles.tsx, tools/video/VideoSliderRow.tsx, tools/shared/SliderRow.tsx, tools/dynamics/ToolSliderRow.tsx, EQSlider.tsx, Volume.tsx) — settings/player routes. Real shell: no GUI binary (W14). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n '190,205p' interaction.ts` (toPercent returns 0-100 NUMBER; slider.css divides by 100 in calc — React stringifies custom properties, contract coherent); `grep -n "lumen-slider-fill" slider.css` (:80/:104 `calc((100% - 4px) * var(--lumen-slider-fill, 0) / 100)`); `grep -rln "<Slider" src --include=*.tsx` (7 consumers); `grep -n "it(" Slider.test.tsx` (17) + `sed -n '221,330p'` (the pixel-math tests read and EVALUATE the real CSS calc formulas); baseline #23 |

## 1. One-paragraph verdict
A 115-line fully-controlled slider that does exactly what its truthfulness contract says: `shown = dragValue ?? value` means the thumb never optimistically jumps — after commit it renders what the caller passes back (:11-15, :78-79), ARIA is complete (role/valuemin/max/now/text/orientation/disabled), disabled is genuinely inert (tabIndex -1, :89), and the CSS-variable seam (`--lumen-slider-fill/--lumen-slider-thumb` as 0-100 numbers consumed by `/100` calc) is coherent across the TS/CSS boundary. The component's real strength is its test file: 13 behavioral tests (scrub-vs-commit call order, step quantization, clamping, full keyboard model incl. Shift-coarse and vertical ArrowUp-increases, disabled inertness) plus a CSS-contract test and three PIXEL-MATH geometry tests that parse slider.css's calc() expressions and prove the fill/thumb flush identity at five probed positions — the only tests in my 27 files that evaluate CSS formulas rather than compare strings. One footgun: spread order lets callers silently clobber the interaction handlers.

## 2. What actually works (verified)
— verified by Slider.test.tsx:64-120 (baseline 1705-pass) → onChangeLive/onCommit ordering, quantize, clamp, keyboard semantics all asserted with real event sequences.
— verified by Slider.test.tsx:291-322 → geometry proofs parse the live CSS (`cssBody`/`decl` helpers) and compute thumb-vs-fill flush at W=320 for p ∈ {0,25,50,75,100} — a reverted formula fails arithmetic, not a string diff.
— verified by read of interaction.ts:26-38 + slider.css:80/:104 → the number-in-custom-property contract holds (React stringifies custom props; CSS divides by 100).
— verified by consumer grep → 7 production mounts across settings, subtitles area, and the player tools.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/slider/Slider.tsx:103-104
Code:
```
      {...handlers}
      {...rest}
```
What is wrong: `rest` (arbitrary HTMLAttributes) is spread AFTER the interaction handlers, so any consumer passing `onPointerDown`, `onPointerUp`, `onKeyDown` or `onKeyUp` through the props interface REPLACES the drag/keyboard engine wholesale — silent breakage with type-system blessing. The props docs (:10-41) do not mention the override hazard.
Why it matters: analytics/telemetry wrappers are the classic accidental culprit; the failure is invisible in review (a legit-looking prop) and total (slider stops responding).
What correct looks like: either compose (`{...handlers, ...rest, onKeyDown: (e) => { handlers.onKeyDown?.(e); rest.onKeyDown?.(e); }}` — at least for the four engine handlers), or a comment on the interface warning that these four props are load-bearing.
Confidence: HIGH — Verification: read; React spread semantics; no current consumer overrides (spot-check of the 7 mounts shows none pass these handlers — hazard is latent, hence S4).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SliderProps` (interface) | 10-41 | REAL | — | every prop documented incl. truthfulness semantics |
| `Slider` | 51-112 | REAL | S4 | controlled render :79; percent custom props :97-102; ARIA complete :85-95; F-001 spread order |
| `Slider.displayName` | 114 | REAL | — | forwardRef hygiene |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 13 behavioral tests | YES | real pointer/keyboard event sequences through the shared engine; call-order assertions (live-before-commit) |
| css contract test (:221) | YES | comment-stripped source scanned for raw colors + 8 required selectors — reads the real file |
| 3 pixel-math geometry tests (:291,:322,:348) | YES | evaluates calc() formulas numerically at 5 positions; the header documents these tests exist BECAUSE a formula bug survived three fix waves undetected — the right lesson, applied |
| (jsdom caveat) | partial | geometry is proven from the CSS source, not from rendered layout — still stronger than every other sheet in this wave |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "after a commit it renders exactly what the caller passes back — never an optimistic jump (doc 08 §5)" | Slider.tsx:11-15 | REAL | `shown = dragValue ?? value` :79 + test :64 |
| "Disabled: aria-disabled + token opacity, fully inert (no pointer, no keyboard, no focus)" | Slider.tsx:25-29 | REAL | tabIndex -1 :89 + test :170 |
| "Keyboard: arrows ±step (Shift = coarse ×10), PageUp/PageDown coarse, Home/End min/max" | Slider.tsx:48-49 | REAL | tests :139/:149 |
| "ArrowUp always increases" (vertical) | Slider.tsx:23 | REAL | test :198 |

## 7. Dead / duplicated / unreachable code
None. The component delegates all logic to interaction.ts (shared with Seekbar — one engine, two components).

## 8. Dependencies & flags
interaction.ts (clamp/quantize/valueFromPointer/useSliderInteraction/toPercent), slider.css. No flags.

## 9. Open questions for the next agent
- interaction.ts (asset-group g003) carries two doc typos worth fixing while that agent is in there: `clamp \`value\` into \`in, max]`` (:20, missing `[m`) and the same in toPercent's doc (:193); also confirm pointer-capture feature detection holds for WebKitGTK (documented jsdom guard only mentions jsdom).
- The pixel-math test pattern (parse calc(), evaluate at sizes) is the single best CSS-verification idea in this repo — clone it to menu.css (row-pitch formula) and list.css (spacer/row geometry).
