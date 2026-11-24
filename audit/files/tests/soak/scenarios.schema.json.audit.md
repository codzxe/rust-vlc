# AUDIT: tests/soak/scenarios.schema.json
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 6,565 bytes / 2b08d8ddca5f25f193923dac6861eca6c4ae5677cc92ae72fcca4eebc0aaab15 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — soak test config schema |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; rg consumers; baseline check |

## 1. Verdict
JSON Schema draft 2020-12 for the soak scenario manifests: required [schema, defaults, scenario], additionalProperties:false everywhere, typed defaults (sample_ms 1..600000, warmup_fraction (0,0.5], leak ceilings with x-lumen-unit annotations), 5 scenario kinds (play-loop/seek-storm/playlist-cycle/open-close-churn/reopen-stress) with kind-conditional requirements. Consumers: crates/playback-engine/tests/soak_scenarios/manifest.rs (Rust loader enforcing "the same contract at load time with loud refusals"), main.rs, tests/soak/scenarios.toml (references it). Baseline: `test shipped_manifests_validate_against_the_schema_contract ... ok` and `test tests::json_validator_accepts_and_rejects_real_shapes ... ok` (cargo-test-workspace.txt:4533-4534) — the schema is actually enforced against shipped manifests, not PAPER-ONLY.
## 2. Findings
F-001 [S3]
Where: $comment L5 — "the regression-guard wiring gap is recorded in docs/soak/soak-report-20261110.md §6.2"
What is wrong: the schema's own comment admits a wiring gap — schema enforcement lives in tests, not in an independent CI validation step (CI never executed at all per soak-nightly.yml "NONE OF THESE JOBS HAS EVER EXECUTED"). Confidence: HIGH — Verification: read (both files).
## 3. What actually works (verified)
Enforcement test passes in baseline (cargo-test-workspace.txt:4533-4534); loader has 15+ loud-refusal unit tests (duplicate ids, out-of-range, unknown keys — :4505-4525) all ok.
