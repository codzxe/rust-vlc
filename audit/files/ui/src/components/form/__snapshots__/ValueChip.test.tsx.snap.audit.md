# AUDIT: ui/src/components/form/__snapshots__/ValueChip.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A03) |
| Date | 2026-11-23 |
| Lines / functions | 9 / 1 snapshot export |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (test artifact exercising shipped code) — ValueChip renders the EQ band readouts via GraphicEQ (PlayerScreen tools dialog) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sed ValueChip.test.tsx:1-40 (exit 0); sed ValueChip.tsx:15-24; grep baseline ui-test.txt:176 (4 tests passed) |

## 1. One-paragraph verdict
Committed vitest golden for ValueChip's "DOM snapshot" test (ValueChip.test.tsx:
20-22, `<ValueChip value="0.0 dB" />`). The golden matches the real component
exactly: ValueChip.tsx:19-21 renders `<span className={["lumen-valuechip",
className]...}>` — element type, base class, and text content all agree.
Executed and current in the baseline run (ui-test.txt:176 — ValueChip.test.tsx
4 tests ✓, so the golden matched a fresh render). Unlike the Button golden,
this one is proportionate: ValueChip's whole prop surface is value + className,
and className composition has its own non-snapshot test (:15-17).

## 2. What actually works (verified)
— verified by `sed -n '20,22p' ValueChip.test.tsx` + `sed -n '17,22p'
  ValueChip.tsx` -> the golden is byte-consistent with the render output.
— verified by `grep -n "ValueChip.test.tsx" baseline/ui-test.txt` -> line 176:
  `✓ src/components/form/ValueChip.test.tsx (4 tests) 38ms`.

## 3. Findings
None. Checked: golden-vs-component consistency (element, class, children),
obsolescence (none — baseline passed), tautology risk (none — the golden is
generated from the real component, and the sibling tests assert accessible
content and CSS skin independently).

## 4. Per-function table
Not applicable — single snapshot export, < 100 LOC.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ValueChip (EQ value readout) > DOM snapshot 1 | YES | Changing the element type, base class, or children rendering fails the golden |
| Siblings (:11-17, :27-33) | YES | Text-content, class-composition, and form.css skin regex assertions — independent of the golden |

## 6. Doc claims touching this file
None.

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Vitest snapshot v1 format. No feature gates.

## 9. Open questions for the next agent
None.
