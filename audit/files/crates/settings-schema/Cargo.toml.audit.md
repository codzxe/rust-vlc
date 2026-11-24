# AUDIT: crates/settings-schema/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 43 / n/a (manifest + header contract) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the crate it configures ships in the CLI: apps/cli/Cargo.toml:61 `settings-schema = { path = "../../crates/settings-schema" }`; apps/cli/src/help.rs:34 `use settings_schema::{GenTarget, generate_str};` + help.rs:157 `settings_schema::runtime::embedded()`; apps/cli/src/host.rs:1371-1402 `schema_meta`/`validate_value` (G4 validation on set-setting writes). Also crates/plugin-host-wasm/Cargo.toml:53. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `ls -la schemas/generated/` (all 3 promised artifacts exist: settings.d.ts 7295B, settings-help.txt 15670B, settings-docs.md 20423B); rg Cargo.lock versions (serde 1.0.229, toml 1.1.8, thiserror 2.0.21); rg usage of all three deps in src (all used); rg [workspace.lints] in root Cargo.toml (present, :53/:59); baseline cargo-test-workspace.txt:5310-5389 (43 unit + 8 integration passed, exit 0); cross-cite W01-A05 audit: `xtask schema-gen --check` -> "no drift ... (121 entries)", exit 0. |

## 1. One-paragraph verdict
Every claim in this manifest's header is true, which is not something I get to write often. The three committed artifacts it promises exist on disk; the G4 runtime-validation claim is real and reaches the shipped CLI (host.rs validate_value on config.set-setting writes); the dependency list is exactly the three policy deps (serde/toml/thiserror), all used, all matching Cargo.lock and the preaudit-foundation pass rows; the bin registration matches src/bin/schema_gen.rs; the xtask subprocess bridge exists and was executed drift-free by another auditor. Zero findings — the checked list is below.

## 2. What actually works (verified)
— verified by `ls -la schemas/generated/`: settings.d.ts (7295), settings-help.txt (15670), settings-docs.md (20423) all committed — the "three committed artifacts" claim is real.
— verified by rg: serde (model.rs/parse.rs derive+Deserialize), toml (parse.rs:34 toml::from_str), thiserror (runtime.rs:23/32, lib.rs:67, emit.rs:35) — no unused dependencies, no undeclared dependencies.
— verified by Cargo.lock cross-check: serde 1.0.229 / toml 1.1.8+spec-1.1.0 / thiserror 2.0.21 = the "serde 1.x, toml 1.x, thiserror 2.x" policy, all **pass** rows in docs/dependencies/preaudit-foundation.md §1 (:24,:26,:28).
— verified by baseline: crate tests 43 + 8 passed, exit 0; clippy -D warnings CLEAN (baseline) means the inherited workspace lints it opts into ([lints] workspace = true, root [workspace.lints.rust]/[workspace.lints.clippy] at Cargo.toml:53/59) are satisfied.
— cross-agent evidence (W01-A05, audit/files/tools/xtask/src/cmd/schema_gen.rs.audit.md): `xtask schema-gen --check` executed -> "no drift ... (121 entries)" exit 0 — the "--check diffs for CI (exit 1 on drift)" claim is backed by a real run.

## 3. Findings
None. Checked and clean:
| Check | Method | Result |
|---|---|---|
| Header artifact claims (G1/G2/G3 paths) | ls schemas/generated/ | all 3 exist with plausible sizes |
| G4 embed claim | rg include_str -> runtime.rs:18 | real embed of schemas/settings.toml |
| G4 consumer claim | rg apps/cli/src | host.rs:1402 validate_value; help.rs:34,157 |
| Dep policy ("no new dependencies") | diff declared vs lock vs usage | 3 declared, 3 used, 3 policy-pass |
| [[bin]] path exists | ls src/bin/schema_gen.rs | present |
| xtask bridge claim | W01-A05 executed run | --check exit 0, no drift |
| lints inheritance valid | root Cargo.toml:53-59 | [workspace.lints.rust]/[clippy] present |
| edition/publish/version | read | 2024 / false / 0.1.0 — consistent with siblings |
| [features] absent | read | none declared, none referenced by dependents (apps/cli uses default) |

## 4. Per-function table
N/A (manifest). Section inventory: header contract (1-29), [package] (31-37), [lints] (39-40), [dependencies] (42-45), [[bin]] (47-49) — all verified above.

## 5. Test honesty
No tests in this file; the crate's tests (43 unit + 8 integration) pass in baseline, and the integration suite covers the generators.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "generates the three committed artifacts ... settings.d.ts / settings-help.txt / settings-docs.md" | Cargo.toml:6-10 | REAL | ls; W01-A05 --check no-drift run |
| "G4 ... schema embedded via include_str! and config.set-setting-style writes are validated" | Cargo.toml:12-15 | REAL | runtime.rs:18; apps/cli/src/host.rs:1371-1402 |
| "Regeneration is idempotent ... --check diffs for CI (exit 1 on drift)" | Cargo.toml:18-19 | REAL-UNPROVEN by me; REAL via W01-A05's executed --check | cited audit (cargo-build-budget + disk-full sandbox prevented my own run) |
| "serde 1.x, toml 1.x, thiserror 2.x ... No new dependencies" | Cargo.toml:27-28 | REAL | Cargo.lock + usage rg + preaudit-foundation §1 |
| subprocess-bridge pattern "first recorded by card 1.B4 (theme-tokens)" | Cargo.toml:22-24 | REAL | crates/theme-tokens/Cargo.toml documents the same bridge (verified while auditing emit.rs) |

## 7. Dead / duplicated / unreachable code
None in the manifest. (The crate-level duplication of diagnostic.rs with product-config is a src-level matter for the settings-schema src auditor — see my diagnostic.rs audit F-003.)

## 8. Dependencies & flags
- serde 1 (derive), toml 1, thiserror 2 — exactly the preaudit pass set; no features, no dev-deps, no optional deps. `[lints] workspace = true` resolves against root Cargo.toml:53-59.

## 9. Open questions for the next agent
- The toml row in preaudit-foundation.md carries a [VERIFY-P1] license caveat ("version endpoint returned null ... confirm at pin time") — the manifest's "all pre-audit pass rows" phrasing is fair but the toml license re-check appears still open; the dependency-compliance auditor should confirm it was closed.
- commands.checksum.txt in schemas/generated/ (modified 19:08, later than the other artifacts) belongs to the commands crate's pipeline, not this one — no cross-contamination observed, but the settings-schema src auditor should confirm nothing in this crate writes it.
