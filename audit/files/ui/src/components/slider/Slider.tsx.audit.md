# AUDIT: ui/src/components/slider/Slider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 114 / 1 (Slider component; interaction delegated to ./interaction) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumers: Volume.tsx/Seekbar.tsx (control bar -> PlayerScreen -> main.tsx), SettingWidgets.tsx (settings screens), subtitles.tsx; via components/index.ts |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln` consumers (3 production + index); Slider.test.tsx case scan (10+ behavioral cases); interaction.ts toPercent locate (L194) |

## 1. One-paragraph verdict
A real, fully-controlled slider primitive with honest truthfulness semantics:
the rendered value is the caller's committed value (`dragValue ?? value` — no
optimistic jump after commit, doc 08 §5), live-drag preview is visual-only,
ARIA is complete (valuemin/max/now/valuetext/orientation/disabled), disabled
is fully inert (tabIndex -1). Pointer/keyboard math lives in ./interaction.ts
(198 LOC, another agent's scope if assigned) — this component is the correct
thin shell over it. One cosmetic defect: no aria-valuetext fallback.

## 2. What actually works (verified)
— Code read: controlled value contract (L79 `const shown = dragValue ?? value`),
CSS-var driven fill/thumb (L97-102), handler spread (L103). — Tests
(Slider.test.tsx): call-order assertion for onChangeLive/onCommit (L64),
step-lattice quantization (L98), clamp (L108), Shift-coarse keyboard (L139)
— behavioral and mutation-sensitive. Not run here (no node_modules; baseline
vitest 1705 passed cited).

## 3. Findings
F-001 [S4]
Where: ui/src/components/slider/Slider.tsx:93
Code:
```
      aria-valuetext={formatValue?.(shown)}
```
What is wrong: when no formatValue is passed, aria-valuetext is undefined and
screen readers announce the raw number — fine for percent sliders, but the
component cannot be distinguished from a "percent" slider without a value
semantics; minor a11y polish gap (consumers all pass formatValue or accept
raw numbers, so blast radius is small).
Confidence: HIGH — Verification: read + consumer scan.

Zero S0-S3 findings. Checked and how: fake seams (none — all state flows
caller->component); dead controls (disabled path fully inert, L89);
swallowed input (clamping/quantizing verified in tests); units (percent math
delegated to toPercent, L194 interaction.ts); reachability (3 production
consumer files).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Slider (forwardRef) | 51-112 | REAL | S4 (F-001) | thin shell over useSliderInteraction; ARIA-complete |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "scrubs: onChangeLive fires continuously, onCommit only on release (call order)" | YES | ordered spy assertions (L64) |
| "quantizes pointer positions to the step lattice" | YES | exact value assertions (L98) |
| "clamps positions outside the track into [min, max]" | YES | L108 |
| keyboard coarse/step cases | YES | L120, L139 |
Not run here; baseline cited.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "after a commit it renders exactly what the caller passes back — never an optimistic jump" | Slider.tsx:12-14 | REAL | L79 controlled `shown`; test L64 pins order |
| "Disabled: ... fully inert (no pointer, no keyboard, no focus)" | Slider.tsx:25-28 | REAL-UNPROVEN | tabIndex -1 (L89) + aria-disabled read; pointer/keyboard gating lives in interaction.ts (not my file) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
react forwardRef, ./slider.css, ./interaction (useSliderInteraction, toPercent).
No feature gates; no backend dependency (presentational primitive — hosts wire
onCommit).

## 9. Open questions for the next agent
- interaction.ts (198 LOC) owns the pointer/keyboard math — audit it for
  step-rounding drift and vertical-orientation correctness; the tests above
  suggest it is solid but the file itself is unverified here.
