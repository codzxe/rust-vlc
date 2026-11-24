# AUDIT: tests/soak/scenarios-nightly.toml
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 2,884 bytes / 1aec62b8b315f160054ed3a864a08498f46ea39cfdb2d8d2a0cf69af7d0e9c30 |
| File verdict | REAL-UNPROVEN (validated shape; the 4 h tier itself never executed anywhere) |
| Reachable from shipped entry point? | NO — soak nightly config |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; cat; rg consumers; baseline + workflow check |

## 1. Verdict
The 4 h nightly soak profile: 5 scenarios (play-loop 48min, seek-storm 48min, playlist-cycle 48min, open-close-churn 24M cycles/48min cap, reopen-stress 1.95M cycles/48min), assert_target=true at 5 MB/h, leak ceiling 50 MB/h, RSS budget 8MB, fd budget 4, live-bytes budget 512KiB. Fixtures referenced (avi_mjpeg_pcm.avi, avi_mjpeg_2pcm.avi, trunc3.avi, noidx0.avi) all exist under tests/fixtures/generated/. Consumers: crates/playback-engine/tests/soak_scenarios/main.rs:301 (test asserting this manifest's shape), the soak-nightly.yml `soak-scenarios` job (SOAK_SCENARIOS=tests/soak/scenarios-nightly.toml). Baseline: shipped-manifest validation test ok (cargo-test-workspace.txt:4533); the CI-short default profile ran `soak_scenario_suite ... ok` in 39.22s (:4532).
## 2. Findings
F-001 [S3]
Where: .github/workflows/soak-nightly.yml:27-29
Code: `# HONESTY NOTE — NONE OF THESE JOBS HAS EVER EXECUTED: CI is billing-blocked on this private repository (every Actions run dies at 0 steps; docs/KNOWN-LIMITS.md R-16).`
What is wrong: the nightly tier this file configures has zero real executions — its cycle-count calibrations (churn 9.24k cycles/s, reopen 749.6 cycles/s from the 2026-11-10 sandbox run) and the asserted 5 MB/h target have never been tested at 4 h. Documented honestly, but the config's assertions are unproven. Confidence: HIGH — Verification: read + baseline has no 4h soak run.
## 3. What actually works (verified)
Shape validated against the schema contract in baseline (:4533); referenced fixtures exist (ls verified during malformed-family audits).
