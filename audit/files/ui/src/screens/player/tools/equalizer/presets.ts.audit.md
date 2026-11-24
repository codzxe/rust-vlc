# AUDIT: ui/src/screens/player/tools/equalizer/presets.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 94 / 0 functions — 6 exported constants (NEUTRAL_GAINS, EQ_PRESETS, DEFAULT_PREAMP_DB, DEFAULT_EQUALIZER_STATE, EQ_GAIN_MIN/MAX/STEP_DB) |
| File verdict | REAL (data) feeding a control that commands nothing |
| Reachable from shipped entry point? | YES — EqualizerTool -> ToolsDialog -> ControlBar "Show Advanced Settings" / Ctrl+E |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read; consumer trace via ToolsDialog.tsx audit (rg: no sends in tools/) |

## 1. One-paragraph verdict
Clean static data table: the 8 frame-scoped VLC/Winamp preset shapes (one-decimal dB values, documented derivation, [VERIFY] markers against a not-yet-existing engine command card), the 0.1 dB lattice fix (j10 P2-1) with its asymmetry rationale. The data is real; its only consumer is the EqualizerTool, which is fully interactive local state with zero engine dispatch (see ToolsDialog.tsx audit F-001) — a curated preset table for a control that cannot apply a preset anywhere.

## 2. What actually works (verified)
— verified by import trace: presets feed EqualizerTool (EqualizerTool.test.tsx asserts preset application to local state). Baseline vitest 1705 passed.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/equalizer/presets.ts:7-8
Code:
```
// numeric preset data — the values below are the canonical shapes VLC ships
// [VERIFY against the engine lane once an equalizer command card exists].
```
What is wrong: the [VERIFY] is honest but open-ended — the equalizer command card does not exist (audio.set-eq absent from the generated catalog), so these dB shapes can never be checked against engine semantics, and the entire table is dead data behind an inert control. Confidence: HIGH — Verification: read + generated catalog inspection (ToolsDialog audit).

## 4. Per-function table (constants)
| Constant | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NEUTRAL_GAINS / EQ_PRESETS | 28-64 | REAL (data) | S4 | 8 presets, frame row order |
| DEFAULT_PREAMP_DB / DEFAULT_EQUALIZER_STATE | 67-74 | REAL | — | frame-measured 12.0 dB |
| EQ_GAIN_MIN/MAX/STEP_DB | 82-94 | REAL | — | ±20 dB, 0.1 lattice (j10 fix) |

## 5. Test honesty
EqualizerTool tests assert preset selection updates local gains — would fail if the table broke; they cannot fail on "preset does not reach the engine" (no engine path exists).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the values below are the canonical shapes VLC ships [VERIFY ...]" (L7-8) | comment | REAL-UNPROVEN (disclosed) | VLC parity unverifiable here; engine card absent |

## 7. Dead / duplicated / unreachable code
None within the file; dead-by-consumer (ToolsDialog F-001).

## 8. Dependencies & flags
Types only (EqMessageKey, EqualizerState). No flags.

## 9. Open questions for the next agent
- If the feature matrix claims EQ presets shipped on any platform, this data + the inert tool is the contradiction to cite.
