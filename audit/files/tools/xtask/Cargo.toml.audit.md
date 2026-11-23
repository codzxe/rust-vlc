# AUDIT: tools/xtask/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A06 |
| Date | 2026-11-23 |
| Lines / functions | 19 / 1 config block set ([package] + [lints]) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (dev-tool manifest; xtask is a workspace member — root Cargo.toml members include tools/xtask — but not a dependency of lumen-cli/desktop). Built by `cargo check/clippy/test --workspace` lanes. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `awk` over Cargo.lock xtask entry -> zero dependencies (entry is name+version only); `rg -c 'name: "' tools/xtask/src/cmd/mod.rs` -> 17 registered subcommands, 18 `pub mod`; `cargo test -p xtask ...` (earlier, this session) -> all green, proving the manifest builds and lints resolve; `sed` quote verification |

## 1. One-paragraph verdict
Functionally correct and minimal: `[package]` (name xtask, version 0.1.0, edition 2024, publish=false), `[lints] workspace = true` (inheritance proven working by the _template canary, executed this session), and genuinely ZERO external dependencies — confirmed against Cargo.lock, where the xtask entry carries no dependency list at all, matching the "Std-only by design (C5)" claim; every source file I audited (gate.rs, feature_audit.rs, codegen.rs, tests/feature_audit.rs) is std+crate-internal only. The two findings are comment drift, not manifest defects: (1) "v0 implements `gate`" describes a 1-subcommand crate that today hosts 17 registered subcommands (cmd/mod.rs registry, attributed to cards 7-e, 7.w21a, 7.w23f, 7.K1, 8.F1, 8.B2, ...); (2) "later cards implement their pre-registered subcommand by editing ONLY their own `src/cmd/<name>.rs` stub" is contradicted by the registry's own card attributions — post-1.A3 cards also edited the frozen dispatch table itself under granted ownership (cmd/mod.rs:20,27-31 record exactly that precedent).

## 2. What actually works (verified)
— verified by `awk` on Cargo.lock: the `[[package]] xtask` block lists no dependencies — the zero-dep claim is true.
— verified by execution this session: `cargo test -p xtask cmd::gate` (23 pass), `cargo test -p xtask cmd::feature_audit` (4 pass), `cargo test -p xtask --test feature_audit` (8 pass) — the manifest builds, the workspace-lints inheritance resolves, no dev-dependencies are needed for the test suites.

## 3. Findings
F-001 [S4]
Where: tools/xtask/Cargo.toml:9
Code:
```
# v0 implements `gate` (doc 10 §2 local gate). Std-only by design: zero
# external dependencies (C5 — nothing to approve, nothing to audit).
```
What is wrong: "v0 implements `gate`" is stale by ~16 subcommands: `rg -c 'name: "' tools/xtask/src/cmd/mod.rs` -> 17 registered commands (gate, check-architecture, schema-gen, codegen, fixtures, rebrand, tokens, feature-audit, parity-report, config-validator, check-panics, lint-literals, check-identity, sbom, web-budget, compat-matrix, visual). Version still 0.1.0. A reader sizing audit scope from the manifest comment would miss 94% of the crate. What correct looks like: "v0 seeded `gate`; the registry in src/cmd/mod.rs is the authoritative command list."
Confidence: HIGH — Verification: read + rg count.
F-002 [S4]
Where: tools/xtask/Cargo.toml:4-6
Code:
```
# `cargo xtask <subcommand>` (alias in `.cargo/config.toml`) dispatches to the
# modules under `src/cmd/`. The dispatch table in `src/main.rs` +
# `src/cmd/mod.rs` is FROZEN after 1.A3: later cards implement their
# pre-registered subcommand by editing ONLY their own `src/cmd/<name>.rs`
```
What is wrong: the "editing ONLY their own stub" mechanism claim is contradicted by the registry's own attributions — cmd/mod.rs:20 ("registered under card 7-e's granted ownership of tools/xtask"), :27-28 (7.w21a), :29 (7.w23f), :31 (7.K1) record post-1.A3 cards editing the dispatch table under ownership grants. The freeze was amended by precedent, which is fine as a process, but the manifest comment describes a mechanism the repo's own comments disavow. No functional impact.
Confidence: HIGH — Verification: read (cmd/mod.rs attributions cited).

## 4. Per-function table
Not applicable (config file, 19 loc). Config keys audited:
| Key | Verdict | Notes |
|---|---|---|
| [package] name/version/edition/publish | REAL | edition 2024 OK under pinned rust 1.99.1; publish=false |
| [lints] workspace = true | REAL | inheritance executes (canary proof, this session) |
| (absent) [dependencies] | REAL | zero deps confirmed in Cargo.lock — matches the C5 claim |
| (absent) [dev-dependencies] | REAL | test suites are std-only (verified in tests/feature_audit.rs) |

## 5. Test honesty
No tests in this file. The manifest's buildability and lint inheritance are exercised every `cargo test -p xtask` run (executed this session, green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by design: zero external dependencies" | Cargo.toml:9-10 | REAL | Cargo.lock xtask entry has no deps; all audited sources std-only |
| "`cargo xtask <subcommand>` (alias in `.cargo/config.toml`)" | Cargo.toml:3 | REAL | .cargo/config.toml contains `[alias] xtask = "run -p xtask --locked --"` (read this session); note ci.yml:71-81 records the alias was once dropped and restored — currently present |
| "v0 implements `gate`" | Cargo.toml:9 | FALSE (stale) | 17 registered subcommands today (F-001) |
| "later cards ... editing ONLY their own src/cmd/<name>.rs stub" | Cargo.toml:5-6 | FALSE (as literal history) | registry attributions show table edits under ownership grants (F-002) |

## 7. Dead / duplicated / unreachable code
None (no dead keys; every declared key is consumed by cargo).

## 8. Dependencies & flags
Zero dependencies (verified). No features. Workspace member via root members list. No [[bin]] override needed (src/main.rs defaults to bin name "xtask", matching the alias).

## 9. Open questions for the next agent
- Root Cargo.toml's members comment says "Card 1.A3 (xtask v0) appended 'tools/xtask' to members — the single planned post-1.A1 edit" — verify whether any OTHER membership edits happened since (the registry precedent suggests the workspace's "frozen" declarations generally drift; a root-manifest audit should reconcile).
