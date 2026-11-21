# AUDIT: ui/src/screens/player/tools/dynamics/ToolSliderRow.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 44 / 1 (ToolSliderRow) |
| File verdict | REAL |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (DynamicsCard renders it for every dynamics tool row; ToolsDialog mounts the tools). ENGINE: NO (staged family — see my DynamicsCard F-001). SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1:0 S2: 0 S3: 0 S4: 1 |
| Commands run | sed shared/SliderRow.tsx:1-20 (the one-row claim); rg "../shared" imports — BOTH ToolSliderRow and VideoSliderRow compose it (dedup completed); read wrapper body |

## 1. One-paragraph verdict
A 44-line frozen-API wrapper: resolves the row's label through the dynamics
i18n seam and delegates everything to the shared SliderRow — exactly the
composition the w26.k10 dedup mandated, and the cross-lane duplication is
genuinely gone (VideoSliderRow imports the same shared row, verified). The
one finding is a stale comment: the header still says the video lane's
drifted copy "is gone as soon as it migrates (k11)" — the migration has
already happened.

## 2. What actually works (verified)
— verified by rg: shared/SliderRow.tsx exists and is imported by BOTH
ToolSliderRow.tsx:23 and VideoSliderRow.tsx:23 — one row implementation,
two lane wrappers (the documented end-state).
— verified by baseline ui-vitest.log.txt (1705 passed): the dynamics tool
tests exercise this row through DynamicsCard (row rendering, values,
aria).
— verified by read: props are fully forwarded (row/value/onChange/
resolveLabel) with no behavior added — a pure adapter; the
"no engine yet" honesty note rides the onChange doc (line 32).

## 3. Findings
### F-001 [S4] Stale dedup comment: "is gone as soon as it migrates (k11)" — the migration already landed
Where: ui/src/screens/player/tools/dynamics/ToolSliderRow.tsx:11-13
Code:
```
 * seam. The video lane's VideoSliderRow composes the same shared row — the
 * 1:1 copy that had already drifted (j11 P2-1) is gone as soon as it
 * migrates (k11).
```
What is wrong: VideoSliderRow.tsx:23 already imports the shared SliderRow —
the k11 migration is complete; the comment describes a pending state that
no longer exists.
Why it matters: Readers auditing duplication are sent looking for a copy
that is gone; wave-log references (j11/k11) rot in place.
What correct looks like: "…is gone (migrated in k11)."
Confidence: HIGH — Verification: rg import proof.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ToolSliderRow | 37-44 | REAL | S4 (comment only) | pure adapter; no logic to break; fully controlled value flow |

## 5. Test honesty
No dedicated test file; covered through the dynamics tool suites
(CompressorTool/SpatializerTool/StereoWidenerTool tests render rows via
DynamicsCard → this wrapper → shared SliderRow). Mutation check: break the
resolveLabel forwarding → labels render as raw keys → tool tests' label
assertions fail. Adequate for an adapter.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the row itself now lives in ONE place — tools/shared/SliderRow" | header 6-8 | REAL | both lanes import it (rg) |
| "the 1:1 copy … is gone as soon as it migrates (k11)" | header 11-13 | STALE | F-001: migration landed (VideoSliderRow.tsx:23) |
| "the slider is the standard components/slider Slider (mission pin: not EQSlider)" | header 19-20 | REAL | shared/SliderRow composes components/slider (header cross-ref; Slider import verified in shared module) |

## 7. Dead / duplicated / unreachable code
None — this file is the anti-duplication artifact itself.

## 8. Dependencies & flags
shared/SliderRow, ./copy, ./types. No client import. No feature flags.

## 9. Open questions for the next agent
- shared/SliderRow.tsx (unassigned to me): the tick-paint domain fix
  (w26.i07) lives there — its own audit should pin the 1.7px-error
  regression test.
