# AUDIT: ui/src/components/form/NumberField.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 90 / 9 |
| File verdict | REAL — stepper/clamp/ARIA contracts + two named regressions (native-bound parity, numeric-string no-op) |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:139 `✓ …NumberField.test.tsx (9 tests) 78ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
Compact and honest: steppers assert exact onChange targets (value string +
valueAsNumber), bounds-disable pins "no fabricated out-of-range values", the
j01-1 regression pins min/max/step reaching the NATIVE input (keyboard parity
with pointer steppers) plus the no-fabricated-constraints negative, the
j01-11 regression pins numeric-string values stepping while empty text
no-ops, error state wires aria-invalid↔describedby id resolution, disabled
inertness, and one DOM snapshot. Mock hygiene is correct —
`onChange.mockClear()` between sub-assertions within each test, fresh
vi.fn() per test (contrast Dropdown.test.tsx's shared mock).

## 2. What actually works (verified)
— Baseline ui-test.txt:139: 9 tests green in 78 ms.
— Zero-findings checklist (spec §12): checked every `it` for a negative or
boundary assertion (bounds-disable L26-29; no-attrs negative L45-48;
no-fabricated-step L61-64; disabled no-fire L82-83); checked regression names
match bodies (j01-1 native attrs asserted L37-39; j01-11 string/empty cases
L55-63); checked snapshot usage (single golden, L86-89); no mocks of the unit
under test; no tautologies.

## 3. Findings
None (see §2 checklist).

## 4. Per-function table
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| render + steppers | 6-29 | REAL | exact targets; bounds-disable |
| native parity regressions | 31-64 | REAL | j01-1 ± cases; j01-11 string/empty |
| error + disabled + snapshot | 66-89 | REAL | id resolution; inertness |

## 5. Test honesty
All 9 fail on their named regression (§2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no fabricated out-of-range values" | file L25 | REAL | L27 disabled-at-bound |
| "no fabricated constraints" | file L42 | REAL | L45-46 attribute-absence assertions |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
RTL + vitest; real NumberField; correct per-test mock hygiene.

## 9. Open questions for the next agent
- None.
