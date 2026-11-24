# AUDIT: ui/src/screens/player/dialogs/parseTimeInput.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A09 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 54 / 7 (2 describes) |
| File verdict | REAL (baseline-verified green; exhaustive grammar + honest-refusal pins) |
| Reachable from shipped entry point? | YES — parseTimeInput.ts drives the Go-to-Time dialog (GotoTimeDialog.tsx consumer, rg verified) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline ui-test.txt:170; rg consumers |

## 1. Verdict
Pure-parser grammar pinned field by field: bare seconds (incl. fractional, whitespace
tolerance), mm:ss with VLC-parity minutes>59, hh:mm:ss, and the refusal set — empty/whitespace,
signed input, non-numeric, fractional low fields, out-of-range 0..59 fields, 4+ fields —
each honest null, never a guess. Baseline green: ui-test.txt:170 `✓ …(7 tests) 5ms`.

## 2. What actually works (verified)
- Baseline 7/7 (cite above).
- Mutation reasoning: any accepted-form regression (e.g. dropping fractional seconds, or
  rejecting 90:00) fails the literal tables; any guessed-value regression (returning 0/-1/
  NaN instead of null) fails the null table — the refusals are exact `toBeNull` pins.
- "1:30.5" refused while "90.5" accepted pins the fractional rule to the low field only —
  a subtle boundary most suites miss.

## 3. Findings
None. Zero-findings list: 7 tests read (all literal input→output tables, no mocks, no DOM,
no tautologies); consumer verified real (GotoTimeDialog); boundary coverage inspected for
off-by-one shapes (0:59 ok / 1:60 null; 3 fields ok / 4 fields null).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bare seconds ×4 | YES | literal equalities |
| mm:ss ×4 | YES | incl. 90:00 → 5400 |
| hh:mm:ss ×4 | YES | 10:11:12 → 36672 arithmetic |
| refusals ×4 tables | YES | exact nulls |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the accepted grammar, pinned field by field; everything else is honestly null" | :1-2 | REAL | both tables exist |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
None beyond vitest + the pure function.

## 8. Open questions for the next agent
None blocking.
