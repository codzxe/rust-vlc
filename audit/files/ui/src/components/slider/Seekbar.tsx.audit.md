# AUDIT: ui/src/components/slider/Seekbar.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 141 / 1 exported component (+2 types) — interaction engine borrowed from interaction.ts (read in full for this audit) |
| File verdict | REAL (truthful-pending scrub bar, correct ARIA, guarded math; 12 honest tests) |
| Reachable from shipped entry point? | NO binary at HEAD (ui-wide). SPA-internal: components barrel → ControlBar SeekCluster (ControlBar.tsx:384-403) → PlayerScreen → /player. Also miniplayer lanes. Executed under vitest (baseline run #23). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read complete + interaction.ts complete (clamp/quantize/valueFromPointer/useSliderInteraction/toPercent — the interaction contract this component depends on) + formatTime.ts presence; Seekbar.test.tsx inventory (12 tests); toPercent range-guard verified (inert duration=0 path returns 0 — no NaN style); consumers: `rg -l "Seekbar" ui/src -g '!*.test.*'`. |

## 1. One-paragraph verdict
The most honest component audited this wave: fill ALWAYS renders the engine position (never the request), the knob renders preview→pending→truth in strict order, buffer ranges come only from engine data with duration-guarded math (the j03 P3-13 Infinity fix is real and tested), the inert no-duration bar disables truthfully, and keyboard seeking (arrows/shift-coarse/PageUp-Down/Home/End with quantize+clamp) lives in a shared, well-factored interaction engine. The two defects are API-level footguns, not behavior: `{...rest}` spreads AFTER the interaction handlers so a caller-supplied onKeyDown would silently kill keyboard seeking, and the primitive cannot enforce an accessible name (role=slider relies on callers remembering aria-label via rest).

## 2. What actually works (verified)
— Truthful-pending contract verified in code and tests: knob = dragValue ?? pending ?? value (:85), fill = value (:88), data-pending suppressed while dragging (:107); tests "pending: knob locks at the requested target…" (:107) and "pending resolves to engine truth…" (:116) pin it.
— Math guards: toPercent returns 0 for range ≤ 0 (interaction.ts:194-197) — inert bar renders 0% fill/thumb, no NaN; buffer widths clamped to `100 - start` (:123).
— Keyboard: interaction.ts:151-190 — preventDefault before the no-op check, live-then-commit call order (documented contract), quantize onto the step lattice.
— ARIA: valuemin/max/now/text on the role=slider div; max guarded for duration ≤ 0 (:101); custom formatter supported (test :149).
— 12/12 tests passed in baseline run #23.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/slider/Seekbar.tsx:114-115
Code:
```
      {...handlers}
      {...rest}
```
What is wrong: `rest` (arbitrary HTMLAttributes) spreads AFTER the interaction handlers — any caller passing `onKeyDown`, `onPointerDown`, `onPointerMove`, `onPointerUp`, or `onPointerCancel` silently REPLACES the slider's entire interaction engine (keyboard seek dies with one innocuous `onKeyDown={trackSomething}`).
Why it matters: Latent (current callers — ControlBar SeekCluster, miniplayer lanes — pass only aria/data props, verified), but the override is silent and the type system permits it.
What correct looks like: Spread `...rest` first and `...handlers` last, or compose caller handlers with the engine's (call both).
Confidence: HIGH — Verification: read + rg of all call sites' props.

### F-002 [S4]
Where: ui/src/components/slider/Seekbar.tsx:98-99
Code:
```
      // aria-label comes from the caller (i18n `useMsg`, doc 07 §6.5) via rest
      // props — no user-visible literals in primitives.
```
What is wrong: The primitive renders `role="slider"` with no built-in accessible name and no runtime dev-warning when a caller forgets `aria-label` — an unnamed slider is an APG violation invisible to every gate (l10n lint checks literals, not missing labels).
Why it matters: Current callers all pass aria-label (ControlBar.tsx:392-396 verified), so no live defect; one forgetful future caller ships an unnamed slider.
What correct looks like: A required-ish `ariaLabel` prop (or a dev-mode warning when absent), keeping the i18n-at-callsite rule.
Confidence: HIGH — Verification: read + call-site props verified.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SeekbarBufferRange` (type) | 12-15 | REAL | — | Engine-data contract. |
| `SeekbarProps` (type) | 17-41 | REAL | F-002 | value/duration/buffered/pending/onCommit/onChangeLive/formatTime documented per doc 08 §5.2. |
| `Seekbar` (forwardRef) | 57-139 | REAL | F-001 | inert = disabled ∥ duration≤0; step = max(duration/1000, 0.001) (undocumented keyboard granularity — a 2h movie seeks 7.2s/keypress, 72s with shift; no `step` prop to tune it); knob/fill split; buffer map with clamps; handlers+rest spread order (F-001). |
| (borrowed) `useSliderInteraction` | interaction.ts:64-180 | REAL | — | Pointer capture feature-detected; cancel aborts without commit; commit = emitted ?? value; keyboard live-then-commit. |
| (borrowed) `toPercent`/`clamp`/`quantize` | interaction.ts:25-36, 194-198 | REAL | — | Range-guarded; lattice snap. |
| `Seekbar.displayName` | 141 | REAL | — | ForwardRef hygiene. |

## 5. Test honesty (Seekbar.test.tsx, 12 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "scrub: knob follows the preview while the fill stays at engine truth" (:75) | YES | Asserts the two CSS vars diverge during drag — the core honesty invariant. |
| "commit fires once with the target seconds; live fired during the drag" (:89) | YES | Event-count assertions. |
| "w26.k03: no buffer nodes on a duration-less bar" (:61) | YES | Pins the Infinity fix. |
| "no duration (no media): inert and truthfully disabled, no events" (:137) | YES | Inert contract. |
| "keyboard: arrows seek by a sub-second step and commit" (:125) | YES | Drives keydown, asserts commit value. |
No tautologies; mutation of the knob/fill split would fail :75 and :107.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Fill: always the engine-reported position — never the request" | :87 | REAL | Code :88 + tests :75/:107. |
| "the component never invents buffer progress" | :9-10 | REAL | Buffer rendered only from `buffered` prop (:121). |
| "Hit area stays ≥16px tall (slider.css)" | :48 | UNVERIFIED | slider.css is A01's file — not checked here. |
| doc 08 §5.2.2 pending lock semantics | :50-55 | REAL | data-pending + knob lock; tests :107/:116. |

## 7. Dead / duplicated / unreachable code
None. All exports consumed (components barrel re-export; ControlBar + miniplayer call sites verified).

## 8. Dependencies & flags
interaction.ts (shared with Slider/Volume/EQSlider — single interaction engine, no duplication), formatTime.ts, slider.css (A01). No feature flags.

## 9. Open questions for the next agent
- slider.css owner (A01): confirm the ≥16px hit area and the pending (accent-strong) knob styling exist — the component's contract depends on them.
- Should `step` be a prop? (2h movie → 7.2s/keypress is coarse for fine seeking; VLC uses ~10s with finer modifiers.)
