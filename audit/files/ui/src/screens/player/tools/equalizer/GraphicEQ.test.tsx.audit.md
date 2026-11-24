# AUDIT: ui/src/screens/player/tools/equalizer/GraphicEQ.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A10 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 235 / 15 (3 describes) |
| File verdict | REAL (baseline-verified green; a11y-level contracts + state-seam proofs) |
| Reachable from shipped entry point? | YES — GraphicEQ.tsx (261 LOC) is mounted by EqualizerTool.tsx (player audio tools) and settings/areas/effects.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline ui-test.txt:89; rg consumers |

## 1. Verdict
Behavioral suite for the equalizer panel: bar↔panel morph with aria-expanded/aria-controls/
hidden wiring, 11 vertical faders with exact frame labels, ±20 dB range + tabular
aria-valuetext, controlled/uncontrolled/defaultOpen state semantics, keyboard commits with
0.1 dB lattice arithmetic, and a shadow-write regression pin (P3-3). Baseline: 15/15 pass
(ui-test.txt:89, 368ms).

## 2. What actually works (verified)
- Baseline 15 tests green (cite above).
- Mutation reasoning: any fader count/label/orientation change fails (:75-99); hidden-body
  wiring failure fails (:17-27); controlled-wins-over-defaultOpen failure fails (:54-71);
  0.1 dB step regression fails via exact payload match (:177-181, 190-193); optimistic
  shadow-write regression fails (:196-233 — rerender without host echo must keep 5.6).
- The monochrome test replaced a known-vacuous null-check with two real pins
  ([data-accent] namespace empty + exactly 3 children per column) — a fixed tautology, and
  the fix comment cites the incident (:130-143).

## 3. Findings
None. What was checked and how: aria attribute wiring (expanded/controls/hidden), role/label
contracts for 11 sliders, chip text counts (1× "12.0 dB", 10× "0.0 dB"), zero-tick class on
exactly the 10 band faders, controlled/default/onChange seams, keyboard commit payloads with
DEFAULT_EQUALIZER_STATE spread equality, source surface (GraphicEQ.tsx 261 LOC + presets.ts
exports DEFAULT_EQUALIZER_STATE), baseline run line. No tautologies, no mocks of the component
under test, no order dependencies.

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| collapsed bar wiring | YES | hidden + role=group + no slider in tree |
| expand/collapse cycle | YES | slider count 11 ↔ null |
| controlled open | YES | onOpenChange(true) + rerender state |
| frame bands/labels | YES | exact 10 labels + aria-orientation |
| chips/valuetext/range | YES | exact text + min/max attrs |
| 0 dB tick | YES | class count 10, preamp col null |
| monochrome | YES | children length exactly 3 |
| controlled state | YES | chip texts 6.0/3.0 |
| commit payloads | YES | toHaveBeenLastCalledWith full-state equality |
| no shadow write | YES | aria-valuenow stays 5.6 after unechoed commit |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "11 fader columns (Preamp + 10 bands)" per EQ.svg | :2-5 | REAL | slider count + labels asserted |
| "0 dB tick class on band faders only" | :5-6 | REAL | class-count test :119-128 |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
i18n lane keys imported (w26.f05) — real localization keys, not literals; presets.ts constants.

## 8. Open questions for the next agent
None blocking.
