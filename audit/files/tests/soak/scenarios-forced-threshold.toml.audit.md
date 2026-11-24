# AUDIT: tests/soak/scenarios-forced-threshold.toml
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | test-infra config, 70 lines, 2 scenarios |
| File verdict | REAL (intentionally-failing enforcement self-proof; recorded run exists) |
| Reachable from shipped entry point? | NO — on-demand profile; never selected by CI/nightly/sandbox (header lines 10-12) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | file read; schema-contract test located (soak_scenarios/main.rs:259-318); baseline cite |

## 1. One-paragraph verdict
This is the anti-ritual artifact done right: an intentionally-failing soak profile that
proves the runner's budget enforcement BITES (a breached budget must yield a FAIL verdict
and non-zero exit). The header documents exactly which two paths trip (growth-budget via
rss_growth_budget_mb = 0.01 with live/VmRSS budgets at 0; slope-ceiling via
leak_ceiling_mb_per_h = 0.0001) and honestly discloses that a negative measured slope PASSES
(the deterministic trip is the growth path). A recorded run exists:
docs/soak/raw/scenarios-forced-threshold.md — release build, 2026-11-10T19:23:46Z, verdict
"FAIL — 2 violation(s)" (open-close-churn: live +82 B over 0 B budget; VmRSS +0.25 MB over
0.01 MB; seek-storm PASS on a −6.951 MB/h slope) — exactly as designed. The manifest is
schema-validated on every test run: soak_scenarios/main.rs:300-308 includes it in
`shipped_manifests_validate_against_the_schema_contract`, which PASSED in the committed
baseline (cargo-test-workspace.txt soak_scenarios block, 32 passed).

## 2. What actually works (verified)
- Baseline: cargo-test-workspace.txt (soak_scenarios section) `test shipped_manifests_validate_against_the_schema_contract ... ok`.
- Recorded enforcement proof: docs/soak/raw/scenarios-forced-threshold.md `## Verdict` ->
  `FAIL — 2 violation(s):` with the two growth violations (verified read).
- Runner wrapper honors exit codes: scripts/soak-scenarios.sh:35 "Exit status: the cargo
  test exit status — non-zero when any configured" budget is breached.
- Profile arithmetic: open-close-churn 1M cycles @ ~9.2k cycles/s ≈ 1.1 min (comment) with
  duration_min = 5.0 cap — consistent.

## 3. Findings
F-001 [S4] The recorded enforcement run is 15 days stale (2026-11-10, captured on an older
checkout path `/wt/w24b/`); no current-tree re-run is committed. The enforcement path itself
is re-proven only on demand (the profile is excluded from every lane). Risk: the self-proof
could silently stop failing (e.g. budget logic refactor) and nothing in the gate would
notice. What correct looks like: run the forced profile in the nightly lane.
Confidence: HIGH — Verification: read (dates + lane lists).

## 4. Config key inventory (vs schema keys asserted in main.rs:271-281)
schema=1, defaults{sample_ms 1000, warmup_fraction 0.15, min_cycles 3, max_skip_rate 0.25,
leak_ceiling_mb_per_h 50, target_mb_per_h 5, assert_target false, rss_growth_budget_mb 0.01,
fd_growth_budget 0, live_growth_budget_bytes 0}, scenario[open-close-churn: cycles 1M,
duration 5 min, 2 fixtures], scenario[seek-storm: duration 1 min, advance_polls 1,
leak_ceiling 0.0001, 2 fixtures]. All keys are loader-enforced (manifest::DEFAULTS_KEYS/
SCENARIO_KEYS asserted against the schema — verified read).

## 5. Open questions
- Does the 8.C1 report's enforcement-proof section re-run this on demand? (docs/soak/
  soak-report-20261110.md — W-next.)
