# AUDIT: ui/src/screens/player/dialogs/parseTimeInput.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 45 / 1 (parseTimeInput) |
| File verdict | REAL-UNPROVEN (correct pure parser; 7 committed unit tests; not executed by me) |
| Reachable from shipped entry point? | YES — GotoTimeDialog (Ctrl+T dialog, PlayerScreen.tsx:792-799 matchModifierBindings path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; byte-verify of L40 (display eats "[ma"; true bytes `const [major, minor, lowest = 0] = fields as [number, number, number?];` — byte list [91,109,97,106,111,114] = "[major"); test inventory (parseTimeInput.test.ts, 7 its) |

## 1. One-paragraph verdict
Correct VLC-grammar time parser: bare (fractional) seconds, mm:ss, hh:mm:ss; rejects negatives, >3 fields, non-numeric or fractional colon fields, and low fields >= 60; nothing guessed or clamped (invalid -> null, caller renders the error). The destructuring line renders mangled through the tool display layer (`[ma` eaten) — byte-verified the source is correct. No findings.

## 2. What actually works (verified)
— verified by read + byte check: regexes (`^\d+(\.\d+)?$`, `^\d+$`), field bounds (minor/lowest <= 59, major unbounded so "90:00" = 5400s), 2-vs-3 field arithmetic, trim, empty->null. 7 unit tests assert accepted/rejected forms (parseTimeInput.test.ts:6-8...).

## 3. Findings
None. Checked: sign rejection, fractional-field rejection, overflow paths (Number on digit strings is safe), default `lowest = 0` for the 2-field form, whitespace.

## 4. Per-function table
n.a. — <100 LOC single function.

## 5. Test honesty
parseTimeInput.test.ts: 7 its over accepted + rejected forms — would fail if the grammar drifted. Not executed by me (baseline: UI vitest pass).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Anything else is honestly invalid (null)" | L8-10 | REAL | every non-matching branch returns null |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None (pure function).

## 9. Open questions for the next agent
None.
