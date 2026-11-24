# AUDIT: ui/src/screens/convert/convertQueue.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 196 / 5 describes, 11 its |
| File verdict | REAL (pure-model pinning; 13 tests green in the committed baseline) |
| Reachable from shipped entry point? | NO (vitest suite; model under test is shipped UI code) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. One-paragraph verdict
High-quality pure-model tests: the convert-queue reducer's lifecycle (enqueue/
start/progress/complete/cancel/fail/block), totality (unknown ids and illegal
transitions are identity no-ops — asserted by reference equality `toBe(queue)`),
progress clamping, the 8-job bound with canEnqueue flip, cancellable's exact
domain, the engine-lane probe against real catalog namespaces (honestly pinned
as empty today: "absence is never proof"), and the formatters. Everything
asserted matches the implementation (MAX_CONVERT_QUEUE_JOBS = 8 verified in
convertQueue.ts:55).

## 2. What actually works (verified)
- Baseline (ui-test.txt:102): `✓ src/screens/convert/convertQueue.test.ts
  (13 tests) 8ms` — green.
- `rg '"(convert|record)\.' crates/commands/src/catalog/ids.rs` -> no matches —
  the "returns empty for the REAL catalog namespaces" test is factually right
  today (no convert.*/record.* engine commands exist).

## 3. Findings
F-001 [S4] Where: convertQueue.test.ts:155 comment "the REAL 64-command engine
catalog" vs crates/commands/src/catalog/ids.rs (~100 dotted ids counted by
`rg -c '"[a-z]\.'` = 100).
What is wrong: the comment's command count is stale (64 vs ~100 now) — comment
only; the assertion itself uses the namespace array, not the count.
Confidence: HIGH — read + counted.

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| convertQueueReduce — lifecycle (6 its) | 32-119 | REAL | monotonic ids, honest cancel partial (0.4 kept), fail/block detail, totality via toBe identity |
| the bound (2 its) | 121-142 | REAL | 8-bound + cancellable domain |
| laneCommandsOf (3 its) | 144-177 | REAL | prefix probe; empty-for-today pinned honestly |
| formatProgressLabel | 179-187 | REAL | clamped percents incl. 0.999→100% rounding |
| sourceBasename | 189-196 | REAL | both separators + trailing-slash empty |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 11 | YES | exact state/progress/detail assertions; identity-reference no-op proofs; bound flip |
## 6. Doc claims: "engine-lane probe against REAL catalog shapes" — verified against ids.rs (no convert/record rows). REAL.
## 7. Dead code: none.
## 8. Dependencies: vitest + the pure module only.
## 9. Open questions: none.
