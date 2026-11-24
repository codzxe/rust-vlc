# AUDIT: ui/src/components/slider/index.ts
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A05) |
| Date | 2026-11-23 |
| Lines / functions | 9 / 0 (barrel) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — main.tsx -> App.tsx -> PlayerScreen tools/dialogs: SliderRow.tsx:34 (Slider), GraphicEQ.tsx:37 (EQSlider), GotoTimeDialog.tsx:38 / BookmarksDialog.tsx:45 (formatSeekTime) all import this barrel |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | test -f on 5 targets (all OK); rg export declarations (5 components + 6 types resolve); rg barrel consumers (exit 0) |

## 1. One-paragraph verdict
Clean barrel: 5 value re-exports + 6 type re-exports, all resolving to real
declarations — Slider.tsx:51, Seekbar.tsx:57, formatTime.ts:8 (formatSeekTime,
executed in its own audit this run), Volume.tsx:27, EQSlider.tsx:30; types at
Slider.tsx:8/:10, Seekbar.tsx:12/:17, Volume.tsx:15, EQSlider.tsx:20. Four
screen-level consumers import through the barrel. `interaction.ts` exists in
the directory but is deliberately internal (consumed by Slider.tsx/Seekbar.tsx
only — not exported, correct layering).

## 2. What actually works (verified)
— verified by `rg -n "export (function|const|type|interface) ..." on the 5
  target files` -> all 11 re-exported symbols exist at cited lines.
— verified by `rg -n "components/slider\"" src` -> 4 consumer files +
  components/index.ts:4 `export * from "./slider"`.
— Baseline ui-test.txt: Slider/Seekbar/Volume/EQSlider test files all inside
  the 136-file passing run.

## 3. Findings
None. Checked: dangling re-exports (none), name mismatch (none), circular
imports (none — file imports nothing), dead barrel (none), internal module
leak (none — interaction.ts correctly stays unexported).

## 4. Per-function table
Not applicable — barrel, < 100 LOC, zero functions.

## 5. Test honesty
No tests in this file; all five re-exported components carry sibling test
files, all executed in baseline.

## 6. Doc claims touching this file
None — no comments in the file.

## 7. Dead / duplicated / unreachable code
None. (VolumeSlider's direct consumer was not re-verified this run — it is
exported and the Volume.tsx test exists; UNVERIFIED which screen imports it,
but the components/index.ts wildcard re-export keeps it reachable.)

## 8. Dependencies & flags
None. Plain re-export module.

## 9. Open questions for the next agent
None.
