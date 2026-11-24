# AUDIT: ui/src/screens/player/tools/equalizer/bands.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A07) |
| Date | 2026-11-24 |
| Lines / functions | 35 / 1 export (EQ_BANDS) + 1 type (EqBandSpec) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — EQ_BANDS → GraphicEQ.tsx:38,190 → effects.tsx:357 `<GraphicEQ/>` (areas/index.tsx:256 audio area → /settings/:area, App.tsx:108) AND GraphicEQ via EqualizerTool.tsx:146 → ToolsDialog.tsx:122 → PlayerScreen.tsx:1203 `<ToolsDialogHost/>` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (EQ_BANDS consumers, label keys, Eq3Band, audio.set-eq, ToolsDialog chain) -> hits cited; sed -n '24,35p' -> quote verified; python3 gains-length count -> 6x10 entries; baseline ui-test.txt:56,89 |

## 1. One-paragraph verdict
A real, consumed data table: 10 band specs (stable id + i18n key), every labelKey resolving in messages.ts:2451-2460 to the frame-verbatim "60 Hz"…"16 KHz" labels, rendered as the fader columns of the shipped GraphicEQ (settings effects embed + tools dialog). The structural risk is a latent 10-vs-3 mismatch: the UI table carries VLC's 10-band set while the engine's only EQ is `Eq3Band` (3 filters, crates/audio-engine/src/effects/eq.rs:194) and no `audio.set-eq` command exists yet — the band-to-engine mapping the table implies has no implementable target today.

## 2. What actually works (verified)
— verified by `rg -n "screen.eq.band" ui/src/i18n/messages.ts` -> keys at :971-980, values at :2451-2460 (all 10 resolve).
— verified by baseline `ui-test.txt:89` `✓ GraphicEQ.test.tsx (15 tests)` and `:56` `✓ EqualizerTool.test.tsx (18 tests)` — bands render as 10 fader columns (GraphicEQ.test.tsx:104 `toHaveLength(10)`).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/equalizer/bands.ts:24-35 vs crates/audio-engine/src/effects/eq.rs:194
Code: (verbatim, sed-verified)
```
/** VLC's classic 10-band set, low → high (frame row order). */
export const EQ_BANDS: readonly EqBandSpec[] = [
  { id: "hz60", labelKey: "screen.eq.band.60" },
```
What is wrong: the UI's band count (10) has no engine counterpart — the only shipped EQ is `Eq3Band` ("composes preamp → low shelf → mid peak → high shelf", eq.rs:7,194); the awaited `audio.set-eq` command (EqualizerTool.tsx:49) does not exist. Why it matters: when the command lands, 10 UI gains must map onto 3 engine filters — a lossy projection nobody has specified; the disclosure lives in effects.tsx:7, not in this file's "VLC's classic 10-band set" header, which implies parity with a real backend. What correct looks like: the mapping spec (or an engine 10-band EQ) documented at the seam before wiring. Confidence: HIGH — Verification: read (both sides verified; command absence verified by rg `audio\.set-eq` over ui/src+crates -> staged markers only).
F-002 [S4]
Where: ui/src/screens/player/tools/equalizer/bands.ts:24 (contract with presets.ts:28 / GraphicEQ.tsx:193)
What is wrong: nothing enforces `gains.length === EQ_BANDS.length` at the type level (`readonly number[]`), and GraphicEQ.tsx:193 `state.gains[index] ?? 0` silently substitutes 0 for a short gains array — drift would render a mislabeled fader, not an error. Why it matters: the band-order contract between three files (bands/presets/GraphicEQ) is convention-only. What correct looks like: `[number, ...]` tuple or a dev assert. Confidence: HIGH — Verification: read (python3 count: all 6 preset arrays + NEUTRAL_GAINS are exactly 10 — currently consistent).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| EqBandSpec (interface) | 16-21 | REAL | — | id + labelKey only; no frequency field (Hz never leaves the UI) |
| EQ_BANDS (const) | 24-35 | REAL | S3/S4 | 10 entries, ids `hz60`…`khz16`, all keys resolve |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| GraphicEQ.test.tsx:104,122 / EqualizerTool.test.tsx:80,93 (`toHaveLength(10)`) | YES for count | counts rendered chips/zero-marks; deleting/reordering a band entry changes the rendered count or order |
| Band label correctness | WEAK | tests assert count and specific texts via eq() mocks; no test pins id→key pairing per entry (a swapped `hz170`/`hz310` labelKey would still pass count checks unless a text query covers each label — spot-check GraphicEQ.test.tsx for per-band text) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "VLC's classic 10-band set" (60…16k) | bands.ts:1 | REAL-UNPROVEN | matches VLC Qt GUI band set from knowledge; no executable VLC here — UNVERIFIED by execution |
| "frame-verbatim labels … `Audio - Equalizer.svg` rows 2-11" | bands.ts:2-3 | REAL-UNPROVEN | messages.ts:2451-2460 values match the listed labels; frame SVG itself not diffed (no renderer) |
| "the dead `accent` field … never read by any consumer — … gone" | bands.ts:10-11 | REAL | rg `accent` in bands.ts + consumers -> only the header mention; EqBandSpec carries no accent field |

## 7. Dead / duplicated / unreachable code
None — EQ_BANDS has two live production consumers (GraphicEQ, presets order-contract) plus index.ts:11 re-export.

## 8. Dependencies & flags
Imports only `EqMessageKey` type from ./useEqMsg. No feature gates. No runtime dependencies.

## 9. Open questions for the next agent
When `audio.set-eq` lands: does the engine grow 10 bands, or does the UI downsample to Eq3Band? Neither file specifies — the mapping is undefined (see F-001).
