//! The shared test-side helpers: turning a scenario's machine-evaluated
//! checks into test verdicts with the failing claim + detail in the
//! assertion message (the lib itself never panics — the workspace
//! lint bans; the test files are where assertions live).

use first_vertical::harness::ScenarioRun;

/// Assert every check of a run held, naming the first failure.
pub fn assert_green(run: &ScenarioRun) {
    for check in &run.checks {
        assert!(
            check.passed,
            "scenario `{}` — check failed: {} — {}",
            run.id, check.claim, check.detail
        );
    }
    assert!(
        !run.checks.is_empty(),
        "scenario `{}` produced no checks (a scenario must assert its AC)",
        run.id
    );
}
