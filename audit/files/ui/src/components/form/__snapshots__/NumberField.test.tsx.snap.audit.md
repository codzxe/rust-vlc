# AUDIT: ui/src/components/form/__snapshots__/NumberField.test.tsx.snap (generated test artifact — short audit per spec §12)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines | 71 (1 snapshot: `NumberField > DOM snapshot 1`) |
| File verdict | REAL (committed vitest snapshot pinning the field's base DOM) |
| Reachable from shipped entry point? | N/A (test artifact; the component it pins — NumberField — ships via settings rows) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read of full file; cross-checked against states.md claims and NumberField.test.tsx (colocated) |

## 1. Blunt verdict
Honest generated snapshot: it pins exactly what the states.md doc claims —
native `<input type="number">` carrying min/step (`min="0" step="1"`,
:20-22), pointer steppers as `aria-hidden` + `tabindex="-1"` buttons
(:29-32) with SVG +/- glyphs, label wired via `for`/`id` pair
(`for="_r_8_"` / `id="_r_8_"`, :9/:18 — the React useId value states.md
:130-131 warns is order-stable but snapshot-embedded). This is a real
structural test: any DOM drift (a removed stepper, a broken label link, a
lost inputmode) fails the suite. The one caveat: snapshots pin STRUCTURE,
not behavior — a component whose steppers dispatch nothing would still
pass this snapshot (the behavior tests in NumberField.test.tsx carry that
load).

## 2. Findings
F-001 [S4]
Where: ui/src/components/form/__snapshots__/NumberField.test.tsx.snap:9,18
Code:
    for="_r_8_"
...
      id="_r_8_"
What is wrong: the snapshot embeds a React useId value (`_r_8_`) — stable
only per test-file order, as states.md itself records. Any test reorder or
React id-format change (React 19 changed the `_r_` prefix format once
already) forces a re-pin of every field snapshot at once. Known, accepted,
documented debt — recorded, not new. Confidence: HIGH — Verification: read.

## 3. Cross-references
- states.md claim 13 ("DOM snapshots embed React useId values — stable per
  file order") — CONFIRMED by this artifact.
- states.md claim 2 (min/max/step forwarded to the native input) —
  consistent with :20-22 (min/step present; max absent because the test
  props omit it — correct pass-through, not a defect).
