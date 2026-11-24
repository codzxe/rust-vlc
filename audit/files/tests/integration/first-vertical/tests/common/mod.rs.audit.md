# AUDIT: tests/integration/first-vertical/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines / functions | 22 / 1 (assert_green) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test helper module |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; rg usage (class_b.rs, evidence.rs scenarios) |

## 1. One-paragraph verdict
The single helper `assert_green(run)` iterates a ScenarioRun's machine checks, asserting each `check.passed` with the scenario id + claim + detail in the failure message, and separately asserts `!run.checks.is_empty()` — the anti-vacuity guard (a scenario that asserts nothing fails the test). This is the honesty backbone for every scenario test in the suite (used by class_b.rs:20-50 and the other first-vertical test files). No logic beyond the two assertions; nothing to break.
## 2. What actually works (verified)
Read in full; consumers verified (class_b.rs imports at L11). The design puts assertions in test files (lib itself is panic-free by workspace lint) — the doc comment L1-4 states exactly that and the code matches it.
## 3. Findings
None. Checked: tautology (none — the checks come from ScenarioRun), mock-of-subject (none — the run is produced by the real harness), ignore-gating (none).
