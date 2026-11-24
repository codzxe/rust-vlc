# AUDIT: crates/testkit/tests/alloc_count_e2e.rs
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 97 / 1 #[global_allocator] + 1 test (5 scenarios) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES from the test lane: `cargo test -p testkit` (baseline ran it green) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read; rg baseline |

## 1. One-paragraph verdict
An instrument-proof test: it installs the counting allocator as the
process allocator and proves the four properties every zero-alloc suite
relies on, including the FAILURE path — scenario 4 builds a deliberately
dirty window inside `catch_unwind` and asserts that `assert_zero` panics
(a test that verifies the test instrument can fail). Ran green in the
committed baseline (audit/runs/baseline/cargo-test-workspace.txt:6237 `test
instrument_end_to_end ... ok`; :6244 also shows the module's doctest
compiling). The one-#[test] design rationale (process-global allocator +
sibling-print interference) is documented in the header — a real
determinism argument, not laziness.

## 2. What actually works (verified)
— Baseline :6237 green.
— Mutation reasoning: if the allocator stopped counting → scenario 1's
`delta.alloc_calls >= 1` fails; if `assert_zero` stopped failing → scenario
4's `caught.is_err()` fails; if the guard leaked state → scenario 5's clean
window fails. Every property has a failing observable.
— Scenario 2's anti-elision (`assert!(acc > 0, "the compiler may not elide
the loop body")`, :66) guards the silent window; any hidden allocation in
it would fail `guard.assert_zero` itself.

## 3. Findings
None. (Checked: catch_unwind safety — the closure is unwind-safe (Vec +
guard are plain values); the panic's own formatting allocations inside the
dirty window are acknowledged and harmless, :78-80.)

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `#[global_allocator] ALLOC` | 29-31 | REAL | — | process-wide instrument install |
| instrument_end_to_end | 35-97 | REAL | — | 5 scenarios: see/drop, silent-zero, snapshots, dirty-fails-loudly, no-stickiness |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| instrument_end_to_end | YES — 5 independent failure observables | incl. the negative path (must panic on dirty window) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the measurement tool itself is measured and proven, not trusted" | :25-27 | REAL | baseline green + failure-path scenario |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
testkit only (crate under test). No features.

## 9. Open questions for the next agent
None.
