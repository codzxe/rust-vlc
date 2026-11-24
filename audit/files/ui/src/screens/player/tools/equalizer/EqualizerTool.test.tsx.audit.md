# AUDIT: ui/src/screens/player/tools/equalizer/EqualizerTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 282 / 18 it() tests, 4 describe blocks |
| File verdict | REAL (18/18 pass in baseline; multiple genuine regression pins) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 56 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full file read; rg count 18 == baseline 18; sed verification (exit 0) |

## 1. One-paragraph verdict
Honest component test for the EQ tool: 11 vertical faders with verbatim frame labels, ±20 dB
ranges, 8 presets with pinned label order, real preset-application math asserted on the
rendered chips (-9.6/16.0/16.4 dB counts), the onChange wire seam asserted with full payload
arrays, and three genuine regression pins (0.1 dB lattice asymmetry from fractional preset
gains; float-dust-free one-decimal values; last-row selection after the scrollport fix).
Notably it asserts the tool's own honesty markers: `data-pending-command="audio.set-eq"` +
a "staged" note — the EQ is an UNBOUND seam in the UI and the test pins that disclosure.

## 2. What actually works (verified)
- Baseline: ui-test.txt line 56 `✓ src/screens/player/tools/equalizer/EqualizerTool.test.tsx (18 tests) 569ms`; my `^\s+it\(` count = 18, exact.
- Lattice regression (verified read, lines 217-236): ArrowUp from Dance's 5.6 -> 5.7, two
  ArrowDowns -> 5.5, Shift+ArrowUp -> 6.5 — asymmetric-step bug (j10 P2-1) would FAIL here.
- Seam payload (lines 194-206): `expect(onChange).toHaveBeenLastCalledWith({ preamp: 12, gains: [0.1, 0, ...], preset: "none" });` — exact array, would fail on any gain/rounding drift.

## 3. Findings
F-001 [S4]
Where: lines 135-141 (verified read)
Code: `expect(listbox.children).toHaveLength(EQ_PRESETS.length);` …
`expect(markers).toEqual(EQ_PRESETS.map((preset) => preset.id));`
What is wrong: count + marker assertions derive their oracle from the component's own
EQ_PRESETS data (golden-from-code-under-test). A presets.ts change passes these lines
automatically. Mitigated: the LABEL order array is hard-pinned verbatim one line below.
Confidence: HIGH — read.
F-002 [S4]
Where: line 199 `const classical = EQ_PRESETS.find((preset) => preset.id === "classical");` …
`gains: classical?.gains,`
What is wrong: the expected seam payload's gains array is sourced from the component data —
if the classical curve changes, the expectation changes with it. The preamp/preset-id parts
are literal. Partially source-derived oracle. Confidence: HIGH — read.

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| describe collapse | 17-56 | REAL | — | aria-expanded, a11y absence of sliders |
| describe fader columns | 58-91 | REAL | — | verbatim labels, ±20 dB, monochrome markers |
| describe preset panel | 93-206 | REAL | S4 | label pin strong; count/markers source-derived (F-001/2) |
| describe fader commits | 208-282 | REAL | — | lattice + float-dust + staged-seam markers |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| keyboard steps stay ON the fader lattice | YES | exact 5.6→5.7→5.5→6.5 chain |
| the LAST preset selects and applies | YES | chip counts per band value (3×-9.6 etc.) |
| a preset pick fires onChange with the full state | YES (partially) | exact payload; gains source-derived (F-002) |
| self-stages like its sibling tools | YES | asserts the honest unbound-seam markers |
| the listbox's children are exactly the 8 options | MIXED | labels pinned; count/markers derive from source (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 18 tests pass | baseline ui-test.txt:56 | REAL | vitest line |
| EQ is a staged/unbound seam (`data-pending-command="audio.set-eq"`) | the test itself | REAL | asserted at lines 268-271 — the UI honestly labels itself unwired |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
vi for the onChange spy; i18n keys imported for OPEN/CLOSE (not hardcoded strings — good
locale-truth). No router needed.

## 9. Open questions
- Is `audio.set-eq` ever served by the CLI host? The staged marker suggests no — the
  W-next domain audit should trace the command's served set (apps/cli host.rs).
