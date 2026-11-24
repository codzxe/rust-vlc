# AUDIT: ui/src/components/form/__snapshots__/RadioGroup.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 67 / 1 snapshot (RadioGroup DOM snapshot 1) |
| File verdict | REAL (generated test artifact) |
| Reachable from shipped entry point? | N/A — vitest snapshot consumed by RadioGroup.test.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; cross-check against RadioGroup.tsx grammar (form.css audit verified producers) |

## 1. One-paragraph verdict
One vitest DOM snapshot pinning the RadioGroup anatomy: fieldset+radiogroup, aria-label, per-option native radio inputs with correct roving tabindex (checked option tabindex=0, others -1), label/for wiring, Light/Dark/System fixture values. Honest snapshot: it pins structure (roles, ids, tabindex, checked) — would fail if the roving tabindex or label association regressed. No baked-in engine data, no fabricated state.

## 2. What actually works (verified)
— verified by reading: the snapshot matches the RadioGroup component grammar audited via form.css/Surface family; baseline vitest 1705 passed includes snapshot checks (1 skipped elsewhere).

## 3. Findings
None. Note (not a finding): `id="_r_9_"` is a jsdom useId artifact — stable within the test env; snapshot brittleness on React id-format changes is a known accepted cost of DOM snapshots.

## 4. Per-function table
N/A (artifact).

## 5. Test honesty
Snapshot test: fails on any DOM structure change (roving tabindex, checked, for/id, class names) — genuine regression pin. It does NOT test behavior (keyboard cycling is covered by RadioGroup's other tests per form.css audit trail).

## 6. Doc claims touching this file
None.

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Vitest snapshot v1 format.

## 9. Open questions for the next agent
- None.
