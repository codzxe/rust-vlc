# AUDIT: tests/integration/first-vertical/tests/journey.rs
| Field | Value |
|---|---|
| Auditor | W16-A04 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / functions | 71 / 7 (all #[test]) |
| File verdict | REAL-UNPROVEN (honest thin drivers; zero execution evidence anywhere) |
| Reachable from shipped entry point? | NO — integration test binary; would run via `cargo test --workspace` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Read full file; Read tests/common/mod.rs; baseline log inspection |

## 1. Verdict
Seven thin test drivers over the six journey scenarios plus a determinism test. The file itself
contains no logic — correctness rests on `assert_green` (tests/common/mod.rs), which is honest:
it fails on any failed check AND on an empty check list. However there is NO recorded execution of
this binary at all: the orchestrator baseline aborted the first-vertical package at the failing
evidence.rs binary (fail-fast), so journey.rs never ran there; sandbox has no cargo.

## 2. What actually works (verified)
- tests/common/mod.rs `assert_green` (read in full):
    assert!(check.passed, "scenario `{}` — check failed: {} — {}", run.id, check.claim, check.detail);
    assert!(!run.checks.is_empty(), "scenario `{}` produced no checks (a scenario must assert its AC)", run.id);
  — a scenario returning zero checks FAILS the test; a regression in any check FAILS it.
- Determinism test (:61-71) compares two fresh run-to-end transcripts byte-for-byte in-process —
  a real double-run, not a golden read.
- Scenario fns exist and are exported: first_vertical::scenarios::{scenario_open … scenario_run_to_end}
  (src/lib.rs:74-78; scenarios module compiled — package built in baseline :2724-2725).

## 3. Findings
F-001 [S3]
Where: audit/runs/baseline/cargo-test-workspace.txt:2761-2795 (context) / this binary
Code:
test result: FAILED. 1 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.54s
error: test failed, to rerun pass `-p first-vertical --test evidence`
### PACKAGE first-vertical exit_code=101
What is wrong: cargo's per-package fail-fast stopped after evidence.rs; the journey.rs test
binary was never executed in the only recorded workspace run (no `journey_` string exists in
the baseline log). The 7 tests have zero pass evidence in this repo's records.
Why it matters: "journey green" cannot be claimed from any artifact.
What correct looks like: a run that reaches this binary (fix evidence/ drift or --no-fail-fast).
Confidence: HIGH — Verification: read (rg 'journey_' over the baseline log: no matches).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| journey_open | 14-17 | REAL-UNPROVEN | — | scenario_open + assert_green |
| journey_play | 22-25 | REAL-UNPROVEN | — | scenario_play |
| journey_pause | 29-32 | REAL-UNPROVEN | — | scenario_pause |
| journey_seek | 36-39 | REAL-UNPROVEN | — | scenario_seek |
| journey_stop | 43-47 | REAL-UNPROVEN | — | scenario_stop |
| journey_run_to_end | 52-55 | REAL-UNPROVEN | — | scenario_run_to_end |
| journey_transcripts_are_deterministic | 60-71 | REAL-UNPROVEN | — | double fresh run, byte-eq assert |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all journey_* | YES (by construction) | assert_green fails on any failed/missing check — verified in common/mod.rs |
| journey_transcripts_are_deterministic | YES | assert_eq on two independent runs' bytes |

Caveat: the checks each scenario installs live in src/scenarios.rs (not this file, not my
assignment) — the file is only as strong as those check lists.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| journey rows = doc 10 §6.4 Phase-2 | journey.rs:1-3 | UNVERIFIED | doc 10 not in my assignment; ids match ledger rows |
| lib.rs:33-37 "normal gate only READS" evidence | lib.rs | REAL | evidence.rs reads, regen is #[ignore] (:191) |

## 7. Dead / duplicated / unreachable code
None. `mod common;` resolves (tests/common/mod.rs exists).

## 8. Dependencies & flags
Depends on crate `first_vertical` (path dep) — nothing gated behind features; no unsafe; no ignores.

## 9. Open questions for the next agent
Did any CI run (ci.yml:234 cargo test --workspace) actually execute this binary before the
evidence drift was introduced? No run record is committed anywhere I could find — UNVERIFIED.
