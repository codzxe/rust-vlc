# AUDIT: ui/src/screens/player/tools/equalizer/index.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A07) |
| Date | 2026-11-24 |
| Lines / functions | 22 / 0 (barrel: 17 exports — 10 value, 7 type) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — ToolsDialog.tsx:36,122 (EqualizerTool) and effects.tsx:59-67 (GraphicEQ, useEqMsg, DEFAULT_EQUALIZER_STATE, EQ_PRESETS, NEUTRAL_GAINS + 2 types) both import this barrel; both mount surfaces are shipped (player dialog + /settings/audio) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg export-existence (all 17 verified at sources) + per-symbol external-consumer scan -> 8 live / 9 dead-via-barrel; sed quotes verified; baseline ui-test.txt:56,89,108 |

## 1. One-paragraph verdict
A real barrel with a genuinely used core: the two components and the preset/default-state payload are imported through it by two shipped surfaces (tools dialog, settings effects area), and all 17 exports resolve to real declarations. The defect is the same dead-surface disease as the sibling barrels: 9 of 17 exports (both Props types, EQ_BANDS, EqBandSpec, DEFAULT_PREAMP_DB, the three gain-bound constants, EqPresetSpec) have zero importers via the barrel — they are consumed only by deep imports inside the module.

## 2. What actually works (verified)
— verified by rg: every export exists (EqualizerTool.tsx:52,61, GraphicEQ.tsx:37,65, useEqMsg.ts:24,35, bands.ts:16, presets.ts:18,28,31,67,70,82-94).
— verified by rg import scan: effects.tsx:59-67 (7 symbols) + ToolsDialog.tsx:36 (EqualizerTool) — both production mounts.
— verified by baseline ui-test.txt:56,89,108 — EqualizerTool (18), GraphicEQ (15), effects tests pass through this barrel's exports.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/equalizer/index.ts:6,8,11-12,15-16,22
Code: (verbatim, excerpt)
```
export type { EqualizerToolProps, EqualizerState } from "./EqualizerTool";
...
export { EQ_BANDS } from "./bands";
export type { EqBandSpec } from "./bands";
```
What is wrong: 9 of 17 exports are dead via the barrel — EqualizerToolProps, GraphicEQProps, EQ_BANDS, EqBandSpec, DEFAULT_PREAMP_DB, EQ_GAIN_MAX_DB, EQ_GAIN_MIN_DB, EQ_GAIN_STEP_DB, EqPresetSpec appear (per exhaustive rg) only inside tools/equalizer/ (deep imports) and this barrel. Why it matters: the public surface advertises an API nobody consumes; the gain bounds and band table are exactly the values a future `audio.set-eq` wire-up would need, so the barrel pre-ships an integration surface for a command that does not exist (EqualizerTool.tsx:49 PENDING_COMMAND). What correct looks like: trim to the consumed 8, re-expose the rest when the engine command lands. Confidence: HIGH — Verification: read.

## 4. Per-function table
Not applicable (barrel). Export trace: 8 live (EqualizerTool, GraphicEQ, useEqMsg, EqualizerState, EqMessageKey, DEFAULT_EQUALIZER_STATE, EQ_PRESETS, NEUTRAL_GAINS — all consumed by effects.tsx/ToolsDialog.tsx), 9 dead-via-barrel (F-001).

## 5. Test honesty
No tests in this barrel. Consumer tests (EqualizerTool.test.tsx 18, GraphicEQ.test.tsx 15, effects.test.tsx — baseline :56,:89,:108) import deep or via the barrel; the barrel's importability for its live half is exercised by effects tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the GraphicEQ vertical-fader panel (EQ.svg), the EqualizerTool that composes it … for the tools dialog" | index.ts:1-3 | REAL | ToolsDialog.tsx:122 mounts EqualizerTool; effects.tsx:357 mounts GraphicEQ |
| "the band + preset tables, and the default state payload for the engine wire-up" | index.ts:3-4 | REAL (payload) / DORMANT (band table, bounds) | DEFAULT_EQUALIZER_STATE consumed by effects.tsx:62; EQ_BANDS/gain bounds have no consumer (F-001); "engine wire-up" target `audio.set-eq` does not exist (see bands.ts audit) |

## 7. Dead / duplicated / unreachable code
F-001's nine exports. No duplication. Internal-only sharing (GraphicEQ↔presets↔bands) bypasses the barrel via deep imports.

## 8. Dependencies & flags
No imports — pure re-export. No feature gates.

## 9. Open questions for the next agent
Same as bands.ts: when `audio.set-eq` lands, do EQ_GAIN_* bounds and EQ_BANDS become the wire contract (justifying F-001's surface), or does the engine grow its own constants? Undefined today.
