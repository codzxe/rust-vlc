# AUDIT: ui/src/components/form/Checkbox.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 39 / 5 |
| File verdict | REAL — small honest primitive tests (39 lines; one-line-plus audits acceptable at this size) |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:174 `✓ …Checkbox.test.tsx (5 tests) 50ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
Five honest primitive tests: labelled native checkbox that fires onChange
exactly once on click, controlled checked state, invalid → aria-invalid,
disabled → native disabled attribute (with the in-file note that fireEvent
bypasses the browser gate so the CONTRACT is asserted), and ref forwarding
plus one DOM snapshot. Fresh per-test mocks. Nothing fake.

## 2. What actually works (verified)
— Baseline ui-test.txt:174: 5 tests green in 50 ms.
— Zero-findings checklist (spec §12): checked each `it` for negative/boundary
coverage (unchecked default L12; disabled contract L30); checked the
disabled-test's honest note (L29-30 — fireEvent bypasses browser gating,
contract asserted instead); checked mock hygiene (per-test vi.fn(), none
shared); checked snapshot usage (single golden L37).

## 3. Findings
None (see §2 checklist).

## 4. Per-function table
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| toggles | 7-15 | REAL | native type + one fire |
| controlled | 17-20 | REAL | |
| invalid | 22-25 | REAL | aria-invalid |
| disabled | 27-31 | REAL | contract-asserted (note in-file) |
| ref + snapshot | 33-38 | REAL | |

## 5. Test honesty
All 5 fail on their named regression (unchecked default, onChange count,
aria wiring, disabled attribute, ref instance).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "fireEvent bypasses the browser's disabled gate, so assert the contract" | file L29 | REAL | toBeDisabled assertion |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
RTL + vitest; real Checkbox; no network/env.

## 9. Open questions for the next agent
- None.
