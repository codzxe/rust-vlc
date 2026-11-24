# AUDIT: ui/src/components/form/SegmentedControl.tsx (compact, 85 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 85 / 1 (SegmentedControl) |
| File verdict | REAL (correct radiogroup semantics except the no-match keyboard hole) |
| Reachable from shipped entry point? | YES — ThemeModePicker.tsx:83 renders it (interface.theme row); rg shows the settings screens as consumers; test file colocated |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg tabIndex RadioGroup.tsx SwatchGroup.tsx` -> same pattern :84/:94; read of full file + useRadiogroupKeyboard import check |

## 1. One-paragraph verdict
A real, properly-semantics segmented control: `role="radiogroup"` /
`role="radio"` buttons, shared roving-keyboard hook (the w26.k01
deduplication verified in the states.md audit), arrow keys with wrap,
disabled-option support, honest `aria-invalid` error grammar. The defect
is shared with its two siblings: roving tabindex is computed as
`checked ? 0 : -1`, so when the controlled value matches NO option —
exactly the "honest unknown" state ThemeModePicker deliberately produces
(`display = ""`) — every segment gets tabIndex -1 and the whole control
becomes unreachable by Tab. Keyboard users lose the theme picker whenever
a stale/unknown value persists.

## 2. What actually works (verified)
— Keyboard model delegated to the shared hook (verified import :3, hook
file exists and is unit-tested per states.md claim 3).
— Click path: `if (!checked) onChange(option.value)` — re-clicking the
  active segment is a no-op, no spurious change events.
— SegmentedControl.test.tsx colocated (in vitest baseline, 1705 passed).

## 3. Findings
F-001 [S3]
Where: ui/src/components/form/SegmentedControl.tsx:71-73
Code:
            aria-checked={checked}
            // roving tabindex: only the selected segment is tabbable
            tabIndex={checked ? 0 : -1}
What is wrong: with a value that matches no option (unknown persisted
value — the documented "renders no active cell rather than guessing"
posture of ThemeModePicker.tsx:68-71), zero segments are tabbable: the
group is skipped entirely by keyboard traversal. Same pattern in
RadioGroup.tsx:84 and SwatchGroup.tsx:94 — a family-wide hole. The ARIA
authoring pattern requires the first non-disabled option tabbable when
nothing is checked. What correct looks like:
`tabIndex={checked || (valueMatchesNothing && firstEnabled) ? 0 : -1}`.
Confidence: HIGH — Verification: read + rg (all three files).

## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| SegmentedControl | 36-85 | REAL | S3 keyboard hole (F-001) |

## 5. Test honesty
Colocated SegmentedControl.test.tsx covers selection, arrows, disabled,
invalid (asserted by the states.md enforcement claim, baseline green).
No test covers the no-match value case — the exact hole F-001 describes;
mutation: setting all tabIndex to -1 unconditionally would pass today's
suite for the no-match case.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantics mirror RadioGroup ... so enum settings can swap without rewiring" | :32-34 | REAL | same roles/hook/tabIndex pattern (verified in all three) |

## 7. Dead / duplicated / unreachable code
None in-file (deduplication already done via the shared hook — verified).

## 8. Dependencies & flags
useRadiogroupKeyboard (shared), form.css tokens. No flags.

## 9. Open questions for the next agent
- useRadiogroupKeyboard.ts auditor: does ArrowKey handling focus-move work
  when nothing is checked (focus starts outside the group)? If arrows
  also assume a checked anchor, the hole is deeper than tabindex.
