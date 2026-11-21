# AUDIT: tools/xtask/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-21 |
| Lines / functions | 19 / 6 config units (package ×5 fields, lints) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as a tool (not shipped in the product): `cargo xtask` alias resolves (`.cargo/config.toml:14-15`) and executed successfully 9× in the baseline (INDEX #8–#16: feature-audit, parity-report, config-validator, check-architecture, fixtures, lint-literals, check-identity, sbom, web-budget — every one ran to a real verdict, several exit 1 with specific violations). CI drives the same binary via the alias's exact expansion (`cargo run -p xtask --locked --`, ci.yml:238,242 etc.). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read tools/xtask/Cargo.toml, src/main.rs (193 lines), src/cmd/mod.rs (263 lines), .cargo/config.toml; wc -l tools/xtask/src/cmd/*.rs (20,716 lines across 19 modules); rg "mod json" (inline hand-written JSON parser at config_validator.rs:652); rg "desktop-tauri" in lint_literals/check_identity/check_panics (absent); rg scan roots: lint_literals.rs:102,576 (`crates/*/src/**` + `apps/*/src/**` — covers desktop-tauri), check_identity.rs:108 (reuses that scan), check_panics.rs:318 (crates/-only fixed list); baseline INDEX #8–#16 (alias works), #19 (full_gate), #11 (check-architecture); Read .github/workflows/ci.yml tauri-check job (lines 673-790) + toolchain asserts. |

## 1. One-paragraph verdict
The manifest's mechanical claims are true: the crate is std-only with zero dependency sections, workspace-linted, and dispatched through the frozen registry exactly as described — and the alias entry point demonstrably works (nine baseline runs). Two things are wrong with what the manifest SAYS about itself. First, "zero external dependencies (C5 — nothing to approve, nothing to audit)" is false as written: avoiding dependencies did not remove audit surface, it moved it in-house — 20,716 lines of gate code including a hand-written TOML parser (toml_lite.rs, 611 lines) and an inline hand-written JSON parser (config_validator.rs:652), whose under-coverage relative to the crates they parallel is already a demonstrated defect (my product.schema.json audit F-001/F-003: the config-validator gate enforces no `pattern` constraints, no magic bytes, no F1/F2). Second, the header still describes the 1.A3-era crate ("v0 implements `gate`") and a dispatch table "FROZEN after 1.A3" that has in fact been extended at least eight times since (17 subcommands now, each addition documented in mod.rs as a granted-ownership precedent — honest there, stale here).

## 2. What actually works (verified)
— Alias: `.cargo/config.toml:14-15` `[alias] xtask = "run -p xtask --locked --"` — resolved and executed by baseline INDEX #8–#16 (`cargo xtask feature-audit` → exit 1 with 2 specific violations; `cargo xtask parity-report` → exit 0 with the 0/252 table; etc.). If the alias were broken, cargo would have errored "no such command" instead of producing gate verdicts.
— Dispatch: main.rs:124-125 routes through `cmd::lookup`; mod.rs:72-193 registers 17 subcommands, all `implemented: true`, each mapped to its own `src/cmd/<name>.rs` module (all 19 modules present); mod.rs tests (209-262) assert name uniqueness and the all-implemented invariant.
— Std-only, structurally: no `[dependencies]`, no `[dev-dependencies]`, no `[features]` in this manifest — nothing outside std can be resolved by its builds.
— `[lints] workspace = true` holds: main.rs:27-28 documents the deny-set compliance; the crate propagates typed `XtaskError`s (main.rs:37-98) instead of panics.
— The alias carries `--locked` — the gate entry point cannot silently drift the lockfile.

## 3. Findings

### F-001 [S3]
Where: tools/xtask/Cargo.toml:9-10
Code:
```
# v0 implements `gate` (doc 10 §2 local gate). Std-only by design: zero
# external dependencies (C5 — nothing to approve, nothing to audit).
```
What is wrong: The parenthetical "(C5 — nothing to approve, nothing to audit)" is false. Zero external crates does not mean zero audit surface: this crate is 20,716 lines of gate code that decides what merges, and to stay std-only it embeds its own parsers and validators — `toml_lite` (611 lines, registered as a non-subcommand module at mod.rs:52) and an inline hand-written JSON parser (`mod json {` at config_validator.rs:652). The consequence is measured, not hypothetical: the config-validator gate built on these reimplementations enforces a strict subset of the schema it claims to validate (no `pattern`, no magic bytes, no F1/F2 — config_validator.rs:14-21 admits it; my product.schema.json audit F-001/F-003 documents the enforcement gaps and the divergent duplicate `pascal_case`).
Why it matters: "nothing to audit" is the kind of sentence that stops an auditor from reading 20k lines of gate code — precisely the code whose bugs make merge gates pass while testing less than claimed. The std-only choice is defensible; the claim that it eliminates verification burden is not.
What correct looks like: "Std-only by design (C5): no external crates to approve; the cost is in-house parsers (toml_lite, json) and validator reimplementation — audit surface shifted, not removed. The product-config crate remains the authoritative validator; this tool's subset is documented per subcommand."
Confidence: HIGH — Verification: read (sizes, module layout, admission comments) + cross-file evidence from my product.schema.json audit.

### F-002 [S4]
Where: tools/xtask/Cargo.toml:3-7
Code:
```
# `cargo xtask <subcommand>` (alias in `.cargo/config.toml`) dispatches to the
# modules under `src/cmd/`. The dispatch table in `src/main.rs` +
# `src/cmd/mod.rs` is FROZEN after 1.A3: later cards implement their
# pre-registered subcommand by editing ONLY their own `src/cmd/<name>.rs`
# stub (each stub header names its owning card).
```
What is wrong: Stale in two ways. (a) "v0 implements `gate`" (line 9) — 17 subcommands are implemented today. (b) The freeze story implies the registry is fixed at its 1.A3 shape and later work only fills stubs; mod.rs:20-31 records at least eight post-freeze REGISTRY additions (check-panics 7-e, visual 4.G1, feature-audit/config-validator/parity-report 8.F1, lint-literals/check-identity 7.w21a, sbom 7.w23f, compat-matrix 8.B2, web-budget 7.K1), each under an explicitly granted ownership precedent. The process is honest in mod.rs; this manifest still sells the original freeze.
Why it matters: The manifest is the first thing a new card's author reads to learn the extension rules; it teaches them a rule the codebase already superseded (new subcommands DO get added, via Lead-granted ownership — the actual, documented practice).
What correct looks like: "FROZEN except by Lead-granted ownership (AGENTS.md §3.7 — see cmd/mod.rs's ownership map for the eight precedents); implemented today: 17 subcommands."
Confidence: HIGH — Verification: read (mod.rs ownership map vs the 17-entry registry).

## 4. Per-key table (config units)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comments (alias, frozen dispatch, v0 scope, std-only) | 1-10 | REAL | S3/S4 | Alias claim TRUE (baseline #8-#16); freeze/v0 stale (F-002); "nothing to audit" misleading (F-001) |
| [package] name = "xtask" | 12-13 | REAL | — | Matches alias `run -p xtask` and CI's expanded form |
| version = "0.1.0" | 14 | REAL | — | Crate-local |
| edition = "2024" | 15 | REAL | — | Toolchain 1.99.1 supports |
| publish = false | 16 | REAL | — | Correct |
| [lints] workspace = true | 18-19 | REAL | — | Member of the workspace (root members list names tools/xtask explicitly); deny-set compliance traced in main.rs |

## 5. Test honesty (tests for this crate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tools/xtask/src/cmd/mod.rs:209-262 (registry uniqueness, wave-1 presence, all-implemented invariant) | YES | Registry drift (duplicate name, missing wave-1 entry, unflipped stub) fails the suite |
| tools/xtask/src/main.rs:165-192 (error Display tests) | YES (narrow) | Message-format tests only — low value but honest |
| tools/xtask/tests/{cli,config_validator,feature_audit,lint_literals,check_panics,check_architecture}.rs | PARTIAL (read for structure only) | config_validator.rs:69 writes a MIRRORED schema fixture rather than the committed product.schema.json — gate-vs-crate drift untested (recorded as F-003c in my product.schema.json audit); full honesty verdicts belong to those files' auditors |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`cargo xtask <subcommand>` (alias in `.cargo/config.toml`) dispatches to the modules under `src/cmd/`" | Cargo.toml:3-4 | REAL | .cargo/config.toml:14-15; main.rs:124-125; mod.rs:72-193; baseline #8-#16 executed |
| "The dispatch table ... is FROZEN after 1.A3: later cards implement their pre-registered subcommand by editing ONLY their own stub" | Cargo.toml:4-7 | FALSE (stale) | Registry extended ≥8× post-freeze (mod.rs:20-31) — F-002 |
| "v0 implements `gate`" | Cargo.toml:9 | FALSE (stale) | 17 implemented subcommands (mod.rs:72-193) |
| "Std-only by design: zero external dependencies" | Cargo.toml:9-10 | REAL | No dep sections in the manifest |
| "(C5 — nothing to approve, nothing to audit)" | Cargo.toml:10 | FALSE | 20,716 lines incl. hand-rolled TOML/JSON parsers; demonstrated enforcement gaps — F-001 |
| ci.yml note 1: "`cargo xtask <cmd>` DOES NOT RESOLVE in a fresh clone" (cross-ref) | .github/workflows/ci.yml:71-75 | FALSE (stale, contradicts this manifest's claim) | Alias restored in .cargo/config.toml (its own header says so, citing "ci.yml note 1"); baseline #8-#16 resolved it — ci.yml's "CURRENT REALITY" reconciliation is outdated and its CI steps still avoid the alias |

## 7. Dead / duplicated / unreachable code
- toml_lite.rs (611 lines) + config_validator.rs's inline `mod json` duplicate parsing that the `toml`/`serde_json` crates (already in the workspace graph via product-config) provide — the std-only tradeoff, quantified; cross-ref product.schema.json audit F-003.
- No dead config keys in the manifest itself.

## 8. Dependencies & flags
- Zero external dependencies (verified: no sections). No features. `[lints] workspace = true`. The crate is a workspace member by explicit root-manifest entry ("tools/xtask"), so `[lints]` resolves.

## 9. Open questions for the next agent
- DOMAIN ANSWER (workspace-exclusion blast radius, evidence gathered here for the root-Cargo.toml/ci.yml auditors): apps/desktop-tauri escapes — workspace build/test/clippy/fmt (baseline #1,#3,#5), cargo-deny `sources` (tauri-check runs only `bans licenses advisories`, ci.yml:743), the SBOM inventory (root Cargo.lock only; the tauri closure's own lockfile is absent from the committed SBOM — baseline #15's 158 components), check-architecture's edge rules (boundaries.toml governs members; the excluded crate self-declares edges in Cargo.toml comments), and check-panics (crates/-only scope, check_panics.rs:318). It does NOT escape lint-literals/check-identity (directory glob `apps/*/src/**`, lint_literals.rs:102,576) or feature-audit (evidence-path scanning). fmt is the sharpest uncompensated gap: no lane anywhere runs `cargo fmt` on apps/desktop-tauri.
- DOMAIN ANSWER (toolchain pin): 1.99.1 is asserted in ci.yml, ci-full.yml, release.yml, tauri-launch.yml (dtolnay action pinned to the 1.99.1 branch ref + a triple-equality assert step). It is NOT respected in audit.yml (deliberate @nightly for Miri/Kani, continue-on-error) nor in rust.yml/rust-clippy.yml — GitHub starter-template workflows with no toolchain pin that still trigger on push/PR to main (rust.yml runs `cargo build`/`cargo test`); those two files are unpinned stock templates (finding for the CI auditor).
- DOMAIN ANSWER (xtask alias): works as documented (baseline #8-#16); ci.yml note 1 (lines 71-81) is stale — it documents the alias as broken and CI steps keep the expanded form; .cargo/config.toml's header explicitly says it was restored to obsolete that note. One of the two files is wrong and it is ci.yml.
- tokens.rs:52 `run(_args: &[String])` discards CLI args (recorded in my theme-tokens audit) — same ignored-parameter pattern may exist in other thin subcommands; worth a sweep by the xtask source auditors.
