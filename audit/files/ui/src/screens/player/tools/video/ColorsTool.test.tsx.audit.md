# AUDIT: ui/src/screens/player/tools/video/ColorsTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A01) |
| Date | 2026-11-24 |
| Lines / functions | 244 / 16 tests + 1 helper (expand) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:77 (16 tests, 480ms); subject mounted via tools/video/harness.tsx + settings/areas/effects.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg "set-colors" (UI-only: no crate command), sed over types.ts + ColorsTool.tsx quotes, it-count 16 == baseline |

## 1. One-paragraph verdict
A precise frame-pinning suite: collapsed-by-default card + aria wiring, the eleven frame rows
(hex inputs with swatch honesty on invalid input, staged slider ranges with exact valuetext,
five OFF toggles, separators' cadence classes), keyboard lattice symmetry regression (the
pre-fix asymmetric quantization would fail it), the Gradient sub-dropdown, and an explicit
"honest staging" test asserting the data-pending-command marker. 16/16 pass in the baseline.
The material fact it honestly encodes: ColorsTool's controls are wired to NOTHING —
`video.set-colors` is a forward-declared marker; no such command exists in any crate. The suite
is real about a component that is a labeled mock.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:77 `✓ src/screens/player/tools/video/
ColorsTool.test.tsx (16 tests) 480ms` (1705-passed run); 16 it() blocks on disk match.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/video/ColorsTool.tsx:224 + types.ts:44-51 (test :236-243)
Code (types.ts, verbatim):
   * Forward-declared catalog command the rows await (the effects-area
   * data-pending-command marker; e.g. "video.set-colors"). No such command
   * exists yet — the marker keeps the controls honestly staged.
Code (test, verbatim):
    expect(card).toHaveAttribute("data-staged-tool", "colors");
    expect(card).toHaveAttribute("data-pending-command", "video.set-colors");
What is wrong: the shipped card renders interactive hex inputs, sliders, switches and a
dropdown whose state never leaves the component — rg "set-colors" across the repo hits ONLY
ui files (no crates/ command, no transport wiring; ColorsTool.tsx has no useClient/send).
Why it matters: a user opening Video tools gets a fully interactive Colors card that affects
no playback; the visible "staged" note (role="note", test :241-242) is the mitigation, and the
code comments declare the gap, so this is honestly-labeled dead-end UI rather than fraud —
but it is UI wired to nothing (checklist 6.5). What correct looks like: land the
video-effects command layer (types.ts:61-66's own plan: "Engine ranges replace these when the
video effects command layer lands").
Confidence: HIGH — Verification: read (rg + sed).

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| ships collapsed (18) | REAL | — | aria-expanded/controls + hidden body |
| expand/collapse roundtrip (28) | REAL | — | |
| defaultOpen (43) | REAL | — | |
| two hex rows (52) | REAL | — | #FF0000 + swatch style |
| three staged sliders (67) | REAL | — | 21.6/22%/35% — staged calibration documented |
| sepia-only ticks (86) | REAL | — | negative assertions on other rows |
| five toggles OFF (98) | REAL | — | |
| five separators (114) | REAL | — | cadence modifier classes pinned |
| hex typing → swatch (131) | REAL | — | |
| invalid hex honest (141) | REAL | — | no fill until valid |
| toggle flips (154) | REAL | — | |
| arrow step readout (161) | REAL | — | 0.05 step keeps "22 %" |
| lattice symmetry (172) | REAL | — | j11 P2-2 regression, ±0.05 undo |
| gradient dropdown (201) | REAL | — | 3 options, selection roundtrip |
| no escape markers in jsdom (214) | REAL | — | degradation pinned; live flip tested elsewhere |
| honest staging (236) | REAL | S3 | F-001's subject fact, asserted honestly |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 16 | YES | every assertion binds to rendered DOM (roles, aria values, styles, classes); no mock of the subject anywhere |
The suite does NOT claim engine effect — the last test pins the staged markers instead, which
is the honest posture for a no-command tool. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "local-state interactions, and the honest staged markers" | header 4-5 | REAL | tests 130-232 + 236-243 |
| "No such command exists yet" | types.ts:48-49 | REAL | rg set-colors → UI files only |

## 7. Dead / duplicated / unreachable code
None in the test. Subject's dead-end nature covered by F-001.

## 8. Dependencies & flags
In-repo (ColorsTool). No transport dependency at all — consistent with the staged pattern.

## 9. Open questions for the next agent
When video.set-colors lands, this suite needs dispatch assertions (it has none today —
correctly, since there is nothing to dispatch). Track whether the effects-area staged set
(CropTool/HdrTool/EqualizerTool siblings) ever got its command layer.
