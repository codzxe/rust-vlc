# AUDIT: tools/xtask/src/main.rs
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-23 |
| Lines / functions | 193 / 6 (XtaskError enum + 2 ctors + Display + Error impl, main, dispatch, print_usage, eprint_usage) + 3 unit tests |
| File verdict | REAL (dispatch works, executed) — the interesting defects are in what it dispatches TO: 8 of 17 registered subcommands are invoked by no script or CI workflow |
| Reachable from shipped entry point? | NO (dev-tool binary; not in the lumen-cli/desktop release lane). It IS the xtask entry point — executed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/xtask help` → exit 0 (17 rows); `./target/debug/xtask definitely-not-a-subcommand` → exit 2 + name list; wiring greps over .github/workflows/ + scripts/ |

## 1. One-paragraph verdict
The dispatch core is small, correct, and honestly tested: typed `XtaskError` (Usage/NotYetImplemented/Internal/Io) with exit 2 on tool errors vs exit 1 on tree verdicts, exact-name lookup over a const registry, and integration tests (tests/cli.rs) that run the real binary through help/no-args/unknown-flag/gate-dry-run paths — I re-executed two of them myself. The registry itself (cmd/mod.rs SUBCOMMANDS, 17 entries, all `implemented: true`, uniqueness + "everything registered is implemented" invariant tests) is the real story: mapping every subcommand to actual invocations in `.github/workflows/` and `scripts/` shows **9 wired** (schema-gen --check, codegen --check, check-architecture, rebrand, fixtures --check, lint-literals, check-identity, sbom, web-budget — via ci.yml `cargo run -p xtask --locked -- <cmd>` and full_gate.sh) and **8 invocation-orphans** (gate, tokens, check-panics, visual, feature-audit, config-validator, parity-report, compat-matrix — compat-matrix is a by-design manual status writer; the others are gates/reports whose enforcement claims are false, per my check_panics and config_validator audits). Additionally, the project runs TWO different "local gates" — this binary's unwired `gate` subcommand (10 steps: fmt/check/clippy/test/tree/ui×3/tauri/wasm-pack) and the actually-used `scripts/full_gate.sh` (11 different sections incl. packaging, fuzz-registry, web-budget) — with doc 10 §2.2 still calling `cargo xtask gate` "One entry point".

## 2. What actually works (verified)
- `./target/debug/xtask help` → exit 0, full 17-row table (every registered name + card + implemented state, verbatim output above).
- `./target/debug/xtask definitely-not-a-subcommand` → exit 2, `xtask error: unknown subcommand … — known subcommands: gate, schema-gen, …, web-budget`.
- tests/cli.rs exercises the real binary: help exit 0 + table contents, no-args exit 2, unknown exit 2, `gate --no-such-flag` exit 2, `gate --dry-run` exit 0 + report shape + `target/gate-report.json` written, `--json` envelope, `codegen --help` bridge probe (all green in baseline cargo-test-workspace.txt; I re-ran two paths manually).
- Unit tests (3) cover XtaskError Display shapes (baseline:6578-6580 area).
- `.cargo/config.toml` defines the `[alias] xtask = "run -p xtask --locked --"` so `cargo xtask <cmd>` works as all docs assume (verified: file read).
- Registry invariants tested in cmd/mod.rs: names unique; all wave-1 names present; everything registered is implemented.

## 3. Findings
F-001 [S3] 8 of 17 registered subcommands are invoked by nothing — the registry advertises a gate surface that automation never reaches.
Where: tools/xtask/src/cmd/mod.rs:72-193 (SUBCOMMANDS); .github/workflows/*; scripts/*
Code (verbatim, the registry's uniform state):
```
    Subcommand {
        name: "check-panics",
        card: "7-e",
        summary: "Panic-surface gate: panic family outside test context (audit F-5)",
        implemented: true,
        run: check_panics::run,
    },
```
What is wrong: grep of every `.github/workflows/*.yml` and `scripts/**` for `xtask <name>` / `cargo run -p xtask --locked -- <name>`:
- WIRED (9): schema-gen (ci.yml:278), codegen (ci.yml:280), check-architecture (ci.yml:238 + full_gate.sh:29), rebrand (ci.yml:444 smoke), fixtures (ci.yml:242,282 + full_gate.sh:36), lint-literals (ci.yml:462 + full_gate.sh:50), check-identity (ci.yml:464 + full_gate.sh:57), sbom (ci.yml:528 + full_gate.sh:64 + scripts/quality/07_supply_chain.sh:20 + scripts/packaging/gen-sboms.sh), web-budget (ci.yml:659 + full_gate.sh:71).
- ORPHANS (8): gate, tokens, check-panics, visual, feature-audit, config-validator, parity-report, compat-matrix (compat-matrix.yml:44 mentions it only in a comment; the workflow runs tests/compat instead — the subcommand is the by-design manual "only status writer").
`implemented: true` means "the module has code", not "the gate runs"; three orphans (check-panics, config-validator, feature-audit/parity-report) are gates whose owning docs claim CI enforcement (see my two prior audit files and the baseline failures: feature-audit FAILURE, parity-report 0/252, config-validator FAILURE). The dispatch is healthy; the wiring layer is where the theater lives.
Confidence: HIGH — Verification: ran (greps, two dispatch paths).

F-002 [S3] Two competing "local gates" with different step tables; the one docs call canonical is the unwired one.
Where: main.rs:100-109 (gate dispatched); tools/xtask/src/cmd/gate.rs:188-280 (10 steps); scripts/full_gate.sh (11 sections); docs/10-testing-ci-quality.md:97
Code (doc 10, verbatim):
```
### 2.2 One entry point: `cargo xtask gate` `[PROPOSAL]`
```
What is wrong: `cargo xtask gate` is invoked by no script/workflow (F-001); the gate the project actually runs (scripts/full_gate.sh, used for the wave-0 baseline) has a different, larger table (fmt/clippy/test/check-architecture/fixtures/fuzz-registry/lint-literals/check-identity/sbom/web-budget/packaging vs gate's fmt/check/clippy/test/tree/ui-lint/ui-typecheck/ui-test/tauri/wasm-pack). The two gates don't even agree on what a gate IS (tauri+wasm-pack vs packaging+fuzz-registry+web-budget). The `[PROPOSAL]` tag keeps the doc honest, but the divergence means "the gate passed" is ambiguous in every doc that doesn't name the script — and gate.rs's step list can drift silently since nothing executes it end-to-end except `--dry-run` in a unit test (which checks the table is printed, not that it matches full_gate.sh).
Confidence: HIGH — Verification: read (gate.rs step table, full_gate.sh, doc 10) + ran (both grep sets).

F-003 [S4] Doc 07 §9's "merge-blocking" job sequence names a subcommand that does not exist.
Where: docs/07-config-driven-system.md:844-846 vs cmd/mod.rs:72-193
Code (doc 07, verbatim):
```
4  cargo xtask config-validator            (C4 — schema + semantics + D-rules)
5  cargo xtask schema-gen --check          (generated files match source)
6  cargo xtask l10n-audit                  (C3)
```
What is wrong: `l10n-audit` is not in the registry — `cargo xtask l10n-audit` exits 2 "unknown subcommand". ci.yml:468 knows ("COVERED C3 (partial): cargo test -p localization above (shipped-tree audit; the l10n-audit xtask subcommand itself does not exist)." — honest note), so doc 07's job sequence is partially fictional. The dispatch is fine; the doc isn't. (Steps 4-5 of the same sequence: config-validator unwired (F-001), schema-gen wired.)
Confidence: HIGH — Verification: read (registry + doc + ci.yml note).

F-004 [S4] Exit-code comment under-describes the contract.
Where: main.rs:16-23
Code (verbatim):
```
//! * `1` — `gate` ran to a verdict and at least one gate command failed
//!   (doc 10 §2.4 `result: FAILURE`).
```
What is wrong: exit 1 is also returned by check-panics, config-validator, lint-literals, check-identity, sbom --check, web-budget --check, feature-audit, parity-report for tree violations — the comment frames it as gate-only. No functional harm (callers branch on nonzero), but the "stable contract — CI and scripts depend on these" text should describe what the subcommands actually do.
Confidence: HIGH — Verification: read (exit sites in the subcommand files I audited) + ran (exit 1 from check-panics and config-validator).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| XtaskError (enum + ctors + Display + Error) | 36-98 | REAL | — | typed errors, no panics; Display/Error tested |
| main | 100-109 | REAL | — | Err → stderr + exit 2; executed |
| dispatch | 112-132 | REAL | — | no-args usage error, help aliases, exact-name lookup, unknown → names list; executed |
| print_usage | 135-154 | REAL | — | table from registry with implemented state; executed |
| eprint_usage | 157-159 | REAL | — | |
| tests (3) | 161-193 | REAL | — | error-message shapes |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| not_yet_implemented_message_names_subcommand_and_card | YES | exact Display string |
| usage_error_renders_message | YES | exact string |
| io_error_includes_context_and_source | YES | context + source concatenation |
| cli: help_exits_zero_and_lists_the_frozen_subcommand_table | YES | exit 0 + all wave-1 names + "card 1.A3" in real binary output |
| cli: no_args_is_a_usage_error | YES | exit 2 + "Usage" on stderr |
| cli: unknown_subcommand_is_a_usage_error_listing_known_ones | YES | exit 2 + echo of the bad name + known names |
| cli: codegen_bridge_dispatches_to_catalog_gen | YES | real subprocess bridge probe (--help) |
| cli: gate_bad_flag_is_a_usage_error | YES | exit 2 + flag name |
| cli: gate_dry_run_executes_nothing_and_reports_the_full_table | PARTIAL | proves table shape + report file written; does NOT prove steps match full_gate.sh or that steps pass (dry run executes nothing — by design) |
| cli: gate_dry_run_json_prints_the_envelope | YES | schema/status fields |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "FROZEN INTERFACE … every subcommand that later wave-1 cards need is registered NOW" | main.rs:5-14 | REAL | 17 registered; additions each cite granted ownership (cmd/mod.rs:20-31) |
| "Exit codes (stable contract)" 0/1/2 | main.rs:16-23 | REAL (underspecified) | executed 0/2; 1 observed from subcommands (F-004) |
| "One entry point: `cargo xtask gate`" | doc 10 §2.2 | FALSE-in-effect | unwired; full_gate.sh is the real gate (F-002) |
| job sequence "merge-blocking" incl. config-validator + l10n-audit | doc 07 §9 | FALSE | config-validator unwired; l10n-audit doesn't exist (F-003) |
| "Workspace lints deny unwrap/expect/panic!/todo!/unimplemented!/dbg!" | main.rs:27-28 | to verify in Cargo.toml audit (next file) | — |

## 7. Dead / duplicated / unreachable code
- `XtaskError::NotYetImplemented` variant + its Display + unit test: currently UNREACHABLE-in-practice — all 17 registry entries are `implemented: true`, so no `run` fn returns it (kept deliberately as the frozen stub protocol; the test pins the message shape). Not dead code by intent; zero live producers.
- The 8 orphan subcommands (F-001) are reachable only by humans typing the command.

## 8. Dependencies & flags
- std-only crate (tools/xtask/Cargo.toml has no dependencies — verified in file-1 work). Entry via `.cargo/config.toml` alias. No feature flags.

## 9. Open questions for the next agent
- Does scripts/full_gate.sh vs gate.rs divergence have an owner (doc 10 §2.2 is [PROPOSAL] — who resolves it)?
- `visual` (4.G1, screenshot-diff harness): is any workflow or soak script supposed to run it? Nothing found; confirm with the visual.rs owner.
- rc-smoke.sh / soak-nightly.yml don't invoke xtask subcommands beyond what I mapped — confirm no other invocation sites exist outside .github/scripts (I searched docs/gates and package.json: only node token scripts).
