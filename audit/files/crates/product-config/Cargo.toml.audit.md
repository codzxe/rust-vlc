# AUDIT: crates/product-config/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines | 24 (manifest: 1 package, 4 deps, no dev-deps, no features) |
| File verdict | REAL |
| Reachable from shipped entry point? | PARTIAL: the crate is a path-dep of apps/control-server (binary `lumen-control-server`, apps/control-server/Cargo.toml:43) but NOT of the release-lane pair — grep over every manifest: lumen-cli and apps/desktop do not depend on product-config. So: reachable from an installable app outside the audited release lane; not from `cargo build --release -p lumen-cli -p desktop`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read + Grep over all Cargo.tomls for `^product-config` (1 hit: apps/control-server); cross-read docs/dependencies/preaudit-foundation.md:24-28; baseline cargo-test-workspace.txt product-config block (all suites pass, exit 0) |

## 1. One-paragraph verdict
Every claim in the header comment checks out against the manifest body and the pre-audit ledger: "No internal deps" — [dependencies] contains zero path entries (verified); "external deps are all pre-audit pass rows … serde 1.x, serde_json 1.x, toml 1.x, thiserror 2.x" — exactly those four appear, and all four are `**pass**` rows in docs/dependencies/preaudit-foundation.md (serde L24, serde_json L25, toml L26, thiserror L28). The tests use `serde_json::to_string` (tests/merge.rs:214) with no [dev-dependencies] — legal because regular dependencies are visible to integration tests, and the baseline proves it compiles and passes (merge: 15 passed). Zero findings; what was checked is enumerated below.

## 2. What actually works (verified)
- Header ↔ body consistency (read side-by-side): 4 declared externals = 4 claimed externals, no internal deps.
- Pre-audit verdicts verified at docs/dependencies/preaudit-foundation.md:24-28 — all four `**pass**` (toml carries an honest "[VERIFY-P1 …] license re-check at pin" note).
- `[lints] workspace = true` matches the root policy (root Cargo.toml:53-66) — the crate inherits deny-unwrap/expect/panic, which explains the `unreachable!` pattern audited in tests/common/mod.rs.
- Baseline: audit/runs/baseline/cargo-test-workspace.txt — product-config exit_code=0 (lib 41, fixtures_matrix 27, golden_s35 1, merge 15, rebrand_host 7, semantic 17, web_budgets 3 — all ok).
- Version ranges are major-only ("1"/"2") consistent with workspace style; exact pins live in Cargo.lock (not re-verified here — UNVERIFIED: lockfile audit is another agent's file).

## 3. Findings
None. Checked and how: (a) internal-deps claim — Grep for path entries: none; (b) external-deps claim — read + cross-checked the pre-audit table; (c) license posture — pre-audit rows; (d) lint inheritance — root manifest; (e) test compile viability without dev-deps — baseline run; (f) release-lane membership — Grep for dependents (control-server only).

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Header comment | 1-8 | REAL | — | claims verified (see §2) |
| [package] | 10-15 | REAL | — | publish=false, edition 2024 |
| [lints] | 17-18 | REAL | — | workspace inheritance |
| [dependencies] | 20-24 | REAL | — | serde+derive/serde_json/toml/thiserror, all pre-audit pass |

## 5. Test honesty
No tests in the manifest; the crate's suites were verified via baseline (see §2) and two direct executions (semantic F1 tests) in the fixture audits.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No internal deps (doc 01 §2.3 (c))" | Cargo.toml:6 | REAL | no path deps in body |
| "external deps are all pre-audit pass rows … serde, serde_json, toml, thiserror" | Cargo.toml:6-8 | REAL | preaudit-foundation.md:24-28 all `pass` |
| crate is "L1 contracts & config" layer | Cargo.toml:3 | REAL-UNPROVEN | layer assignment is doc-01 taxonomy; the crate's behavior (typed model + diagnostics) matches, doc-01 itself not re-audited here |

## 7. Dead / duplicated / unreachable code
Not applicable to a manifest. Note only: the crate's only consumer app (control-server) sits outside the audited release lane — flagged in the reachability row, not scored (product.toml validation also runs in CI via xtask config-validator, which reads product.toml through this crate's logic — see audit/runs/baseline/xtask-config-validator.txt: FAILURE on 4 missing brand assets, a product.toml data issue, not a manifest issue).

## 8. Dependencies & flags
No features, no default-features overrides, no [dev-dependencies]. External: serde 1 (derive), serde_json 1, toml 1, thiserror 2.

## 9. Open questions for the next agent
- Cargo.lock pins for these four (I did not audit the lockfile).
- Whether lumen-cli should surface product identity (name/version) at runtime — if yes, product-config joins the release lane and the reachability row flips to YES.
