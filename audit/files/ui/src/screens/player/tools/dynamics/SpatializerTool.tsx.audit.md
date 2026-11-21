# AUDIT: ui/src/screens/player/tools/dynamics/SpatializerTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 80 / 2 (SPATIALIZER_ROWS, SpatializerTool) |
| File verdict | REAL (honest staged-inert tool card; no backend of any kind — disclosed) |
| Reachable from shipped entry point? | YES in the web UI (ToolsDialog.tsx:37 imports + :128 mounts `<SpatializerTool />` on the audio tab; ToolsDialogHost mounts at PlayerScreen.tsx:1203; opened from ControlBar.tsx:702). NO desktop binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read DynamicsCard.tsx (the shared staged shell: data-staged-tool/data-pending-command markers, role="note" staged line, local-only slider state, memoized row adapter); Grep `spatializer\|equalizer\|compressor\|stereo-widener` in generated/commands.ts → **no matches — `audio.set-spatializer` is not in the wire catalog at all** (a genuinely forward-declared id, unlike CropTool's existing-but-unserved `video.set-crop`); rg -i spatializer over crates/ + apps/ → zero matches (no engine stage either — DynamicsCard's header claim "no compressor/spatializer/stereo-widener engine stage or catalog command exists" is TRUE); Read SpatializerTool.test.tsx:26-115 (11 tests); tick-math verification below; Grep mounts (ToolsDialog :128). Baseline: ui-vitest 136 files / 1705 passed. |

## 1. One-paragraph verdict
A clean 80-line configuration of the shared DynamicsCard: five frame rows (Size/Width/Wet/Dry/Damp) with disclosed staged display ranges, per-row degree ticks whose math checks out (Size: default 8 on a 0..11 scale with 10 ticks at i/11 — tick 8; Width at the max end; Wet/Dry/Damp defaults 4/5/5 on the 1..9 marks of a 0..10 scale — exactly the header's claim, and the test's 46-tick + 4-covered assertions pin it), one-decimal formatting, keyboard step and Home-clamp behavior pinned by tests. The staging is the honest pattern done fully: local state only, visible "staged" note, data-pending-command naming a command that does not exist anywhere in the protocol — and unlike the CropTool case, the header's "no catalog command exists" claim is TRUE here (verified: zero matches in generated/commands.ts and zero engine-side spatializer code). Nothing in this file misstates anything.

## 2. What actually works (verified)
— verified by SpatializerTool.test.tsx:26-115 (collapsed/expanded shell + aria wiring; five rows with labels, sliders, plain "8.0"/"10.0"/"4.0"/"5.0"×2 values; 46 ticks with 4 covered defaults; aria-valuemin/max/valuetext "4.0"; ArrowRight commit 5→5.5; Home clamp→"0.0"; staged markers + role="note") + baseline ui-vitest (136 files, 1705 passed).
— verified by arithmetic on the row defs: ticks render at i/(n+1) of the track (ToolSliderRow contract) — Size 0..11 with ticks=10 puts marks at 1..10/11 and initial 8 sits exactly on mark 8; Wet/Dry/Damp 0..10 with ticks=9 puts marks at 1..9/10 and initials 4/5/5 sit on marks; Width initial 10 = max end (past the last mark, honestly noted in the test).
— verified by Grep: zero spatializer references in crates/ + apps/ and zero in the generated wire union — the header and the pendingCommand marker describe reality.

## 3. Findings
None. Checked and clean: row-def/range/tick arithmetic (above), disclosure markers (rendered by DynamicsCard:91-92 + note :138-140), no dead code, no duplicated literals (labels via i18n keys w26.c2), no engine/transport imports, props passthrough correct ({...props} forwards defaultOpen/id).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SPATIALIZER_ROWS` | 15-66 | REAL | — | Five module-level row defs; ranges disclosed as staged display ranges (dynamics/types.ts note); format toFixed(1) matches frame "plain values". |
| `SpatializerTool` | 69-79 | REAL | — | Thin wrapper: tool id, name key, pendingCommand "audio.set-spatializer" (nonexistent id — accurately forward-declared), rows, props spread. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :26/:36/:50 shell + aria | YES | DOM/attribute assertions. |
| :59 five rows, values, 5 sliders | YES | Role/text assertions incl. the duplicated "5.0" count. |
| :72 46 ticks, 4 covered | YES | Pins the tick arithmetic exactly. |
| :81 valuetext format | YES | aria-valuetext "4.0". |
| :91/:97 keyboard commit + clamp | YES | Interaction assertions. |
| :107 staged markers | YES | data-staged-tool/data-pending-command/role=note. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Tick counts are the frame's data-driven degree marks: Size 10, the other rows 9" | SpatializerTool.tsx:6-7 | REAL | Row defs + test :74 (46 = 10+4×9). |
| "each default sitting exactly on a tick (Size 8 of 11 steps, Width at the max end, Wet/Dry/Damp on the 1..9 marks…)" | SpatializerTool.tsx:7-9 | REAL | Arithmetic verified (§2); test's 4-covered count consistent (Width uncovered). |
| "Ranges are staged display ranges (types.ts note)" | SpatializerTool.tsx:9 | REAL | dynamics/types.ts carries the note; no engine ranges exist to mirror. |
| (inherited shell claim) "no compressor/spatializer/stereo-widener engine stage or catalog command exists" | DynamicsCard.tsx:16-18 | REAL | rg crates/+apps/ = 0; commands.ts = 0. |

## 7. Dead / duplicated / unreachable code
None. Product-dead-by-design: the five slider values (no consumer, staged, disclosed).

## 8. Dependencies & flags
DynamicsCard + dynamics types + i18n keys (w26.c2 lane via useDynamicsMsg). No transport, no engine imports, no flags. The named command does not exist in the wire union (forward declaration, disclosed).

## 9. Open questions for the next agent
- When an audio-effects command layer lands, the spatializer's five parameters need an engine-side range reconciliation (current 0..11 Size etc. are display-only calibrations) — the same CropSpec-style mismatch flagged in my CropTool audit §9.
