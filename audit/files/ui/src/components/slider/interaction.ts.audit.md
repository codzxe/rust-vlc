# AUDIT: ui/src/components/slider/interaction.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 198 / 6 (clamp, quantize, valueFromPointer, useSliderInteraction, toPercent + inner callbacks) |
| File verdict | REAL-UNPROVEN (sound interaction engine; exercised by committed tests; not executed by me) |
| Reachable from shipped entry point? | YES — Slider.tsx:67 and Seekbar.tsx:73 call useSliderInteraction; Seekbar is the player transport (PlayerScreen/MiniPlayer/NowPlayingCard), Slider drives settings rows + EQ tools |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; grep consumers; test-file inventory (Slider.test.tsx 17 its, Seekbar.test.tsx 12 its) |

## 1. One-paragraph verdict
Clean, correct interaction engine: live-vs-committed semantics exactly as documented (dragValue is a local preview, onCommit fires on release/keyboard with the last emitted value, callers render engine truth — no optimistic jump), pointer math guards zero-size rects (`rect.width || 1`), quantize/clamp compose safely for both orientations, pointer capture is feature-detected with try/catch for jsdom, pointercancel aborts without commit, keyboard set covers arrows/PageUp-Down/Home/End with shift-coarse and a no-change guard. No findings. (Note: the source comments read "[min, max]" but display as "in, max]" through the tool pipeline — the `[m`/`[h` two-char sequences are eaten by the output layer; byte-verified this is a display artifact, not source corruption.)

## 2. What actually works (verified)
— verified by read + grep consumer chain + committed tests: Slider.test.tsx:98 "quantizes pointer positions to the step lattice", :108 clamping test (17 its), Seekbar.test.tsx (12 its). Baseline: UI vitest pass.

## 3. Findings
None. Checked: clamp bounds, quantize lattice (round + step guard), zero-rect division, orientation mapping (horizontal clientX/width, vertical bottom-origin), drag-state races (draggingRef gate), commit-on-cancel avoidance, keyboard preventDefault + equality guard, toPercent range guard (range <= 0 -> 0), emitted-dedupe ref. All sound.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| clamp | 17-18 | REAL-UNPROVEN | — | min/max bound |
| quantize | 21-25 | REAL-UNPROVEN | — | step<=0/NaN no-op; lattice round |
| valueFromPointer | 28-46 | REAL-UNPROVEN | — | rect guard, both orientations, quantize+clamp |
| useSliderInteraction | 64-171 | REAL-UNPROVEN | — | the engine; callbacks memoized with correct deps; jsdom-safe capture |
| toPercent | 174-179 | REAL-UNPROVEN | — | range guard |

## 5. Test honesty
Slider.test.tsx / Seekbar.test.tsx assert quantization, clamping, live/commit ordering — would fail if the math broke. Not executed by me (no node_modules; baseline cites pass).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`onCommit` fires on pointer release and on keyboard set — the value callers act on ... never an optimistic jump" | header L11-15 | REAL | endDrag commits emitted-or-current; component renders caller `value` |
| "Pointer capture is feature-detected (jsdom has no active-pointer table)" | L61-62 | REAL | try/catch around setPointerCapture (L93-97) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
React hooks only. No feature flags.

## 9. Open questions for the next agent
None.
