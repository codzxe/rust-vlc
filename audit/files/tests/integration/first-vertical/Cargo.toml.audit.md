# AUDIT: tests/integration/first-vertical/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines | 43 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test crate manifest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; root Cargo.toml members check; baseline check |

## 1. One-paragraph verdict
Manifest of the merge-blocking integration suite: package `first-vertical`, std-only (no external deps), path deps on `commands` + `lumen-cli` (the comment L30-35 states the intent: drive everything through the command layer, never an engine API directly), dev-dep `testkit`, `[lints] workspace = true`. Registered in root Cargo.toml:41 members (`"tests/integration/first-vertical"`). Baseline: package runs in the per-package test sweep — 10 lib tests ok, 2 suite files ok, class_b 5/5 ok, evidence.rs 2 FAILED (:2722-2795) — i.e. the suite is wired and merge-blocking in the real gate, and its evidence-guards currently FAIL (the known first-vertical drift).
## 2. What actually works (verified)
- Membership: root Cargo.toml:41 (read).
- The claim "no external dependencies" (L18-21) is true in the manifest: only path deps `[dependencies] commands, lumen-cli` + `[dev-dependencies] testkit`.
- The claim "merge-blocking" — .github/workflows/ci.yml runs cargo test (other agents verify exact wiring); the baseline runs it.
## 3. Findings
None. Checked: dependency hygiene (2 path deps + 1 dev path dep), lints inheritance, publish=false, description accuracy vs. actual contents (tests drive lumen_cli::dispatch — verified via class_b.rs scenarios import from first_vertical::scenarios which uses the command layer per its docs; deep verification of scenarios.rs is another agent's).
