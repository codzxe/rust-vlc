# AUDIT: crates/_template/tests/smoke.rs
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-23 |
| Lines / functions | 8 / 1 test (`template_marker_is_stable`) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test of the `template` crate (publish=false scaffolding/canary member, in no internal_deps list). Runs under `cargo test --workspace`: baseline cargo-test-workspace.txt:6186 `test template_marker_is_stable ... ok`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read of crates/_template/src/lib.rs (asserted constant verified); baseline citation cargo-test-workspace.txt:6173-6197 (PACKAGE template, exit_code=0) |

## 1. One-paragraph verdict
An honest, minimal smoke test that does exactly what its doc header says: proves the template crate builds inside the workspace and its public surface is callable, by asserting `template::template_marker()` returns the documented marker string. The asserted value matches the implementation literal in crates/_template/src/lib.rs (`"lumen-crate-template"`, which carries a sanctioned `// lumen-allow-literal:` marker for the C1 literal gate), and the test ran green in the orchestrator baseline. It is deliberately a placeholder — the header says "Replace with real tests when copying the template" — and it does not pretend to be more.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:6186: `test template_marker_is_stable ... ok` (PACKAGE template exit_code=0) — the test runs and passes in the workspace suite.
- Read-verified: `template_marker()` in crates/_template/src/lib.rs returns `"lumen-crate-template"` — the assertion's expected value matches the real constant.

## 3. Findings
None. What was checked and how:
- Tautology risk (checklist 6.4): the test compares a function returning a hard-coded literal against the same literal — on its own that proves nothing about behavior, but the test's stated purpose is build/linkage smoke ("proves the template builds and its public surface works inside the workspace"), which is exactly what a passing compile+call proves; the name (`template_marker_is_stable`) does not promise more. Mutation reasoning: deleting `template_marker()` from lib.rs, changing the marker string, or breaking the crate's compile all fail this test.
- Placement/visibility: standard `tests/smoke.rs` integration test, auto-discovered by cargo; verified wired via the baseline run.
- Literal-gate interaction: the marker literal in lib.rs carries `// lumen-allow-literal: crate-template placeholder value...` — consistent with the lint-literals policy (baseline lint-literals failure is in apps/cli/src/host.rs, not here).
- Instruction honesty: header says to replace this file when instantiating the template; consistent with lib.rs's step-3 instruction ("Replace `template_marker()` and `tests/smoke.rs`").

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `template_marker_is_stable` | 5-8 | REAL | — | Asserts the marker constant; passes in baseline |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| template_marker_is_stable | YES (for its stated scope) | Fails on compile break, missing/renamed fn, or marker-string change; it does not claim to test behavior beyond the smoke surface |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "proves the template builds and its public surface works inside the workspace" | smoke.rs:1-3 | REAL | Baseline run: compiled + passed in cargo test --workspace |
| "Replace with real tests when copying the template" | smoke.rs:3 | REAL (instruction) | Consistent with lib.rs copy-instructions; no other crate references template_marker (rg) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None — plain #[test] on the `template` crate; no dev-dependencies, no features.

## 9. Open questions for the next agent
None.
