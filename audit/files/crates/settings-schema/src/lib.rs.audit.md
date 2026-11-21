# AUDIT: crates/settings-schema/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2026-11-21 |
| Lines / functions | 213 / 6 (4 items + 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — two lanes. (a) Shipped `lumen` binary: apps/cli/Cargo.toml:61 depends on this crate; `config.set-setting` → `runtime::validate_value` at apps/cli/src/host.rs:1402, `config.get-setting` → `runtime::schema_meta` at host.rs:1371/1400 (the embedded G4 schema, runtime.rs:18 `include_str!`). (b) Tooling: `load` (lib.rs:100) ← schema_gen bin (src/bin/schema_gen.rs:102) ← xtask `schema-gen` subcommand (tools/xtask/src/cmd/schema_gen.rs, subprocess bridge) ← CI ci.yml:278 `schema-gen --check`; drift-free state proven in baseline by tests/integration.rs::committed_outputs_have_no_drift (`ok`, orchrun-test-workspace-nofailfast.log.txt:4934). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | grep dependents: `settings-schema` in all Cargo.tomls → apps/cli:61 (shipped), plugin-host-wasm:53; grep callers of `validate_value`/`schema_meta`/`embedded` → apps/cli/src/host.rs:1371,1400,1402, help.rs:157, plugin-host-wasm/src/supervisor.rs:123,171; grep `parse_and_validate_str`/`settings_schema::load` → schema_gen.rs:102 only; grep `Warning`/`Severity` across crate → diagnostic.rs definition only (dead variant); grep unwrap/expect/panic/unsafe/todo/unimplemented in src/*.rs + bin → zero non-comment hits; sed -n verified all quotes. Baseline: orchrun-test-workspace-nofailfast.log.txt:4872–4939 (settings_schema lib 43/43 ok incl. both lib.rs tests; integration 8/8 ok); ui-settings_check.log.txt (settings:check PASS). |

## 1. One-paragraph verdict
This is the one config-system crate that is genuinely wired end-to-end: the
shipped `lumen` binary embeds the schema and validates every
`config.set-setting` write through it (host.rs:1402, errors mapped to
`invalid-argument`, no silent coercion), and the generator lane (schema_gen
binary ← xtask ← CI) keeps the three committed artifacts drift-free — proven
by the baseline integration run. Both lib-level tests pass in the baseline,
and the committed-seed test is a real integration invariant with an
independent raw-TOML count so a lossy loader cannot hide. The file's defects
are small and latent: a documented-but-dead `Severity::Warning` advisory
channel that the loader could not honor even if someone emitted it (every
diagnostic is fatal at this seam), and a "no panic" doc claim whose lint
enforcement does not cover the `unreachable!` macros the file's own tests use.

## 2. What actually works (verified)
— Both lib.rs tests pass in baseline: orchrun-test-workspace-nofailfast.log.txt:4899 `test tests::every_seed_default_ref_resolves_in_product_toml ... ok` and :4913 `test tests::committed_seed_validates_against_committed_product ... ok` (43/43 crate lib suite, 8/8 integration, zero failures — the only crate in my assignment set with a fully green baseline).
— G4 seam verified in shipped code: apps/cli/src/host.rs:1400–1403 — `schema_meta(&full)` then `validate_value(&full, &toml_value).map_err(|err| invalid(err.to_string()))` — the doc's "type/range/enum, `invalid-argument`, never silent coercion" claim (lib.rs:17–19) is literally the code path.
— Generated-artifact freshness (G1–G3) verified: baseline integration test `committed_outputs_have_no_drift ... ok` at orchrun log :4934 (runs schema-gen `--check`), `regeneration_is_idempotent_byte_for_byte ... ok` :4936, `stale_output_drift_fails_check_with_exit_one ... ok` :4937, matching schemas/settings.toml's header claim "a second run is byte-identical".
— UI-side consumption of the same source of truth: `npm run settings:check` → "OK (10 ui areas, 102 ui entries, 121 total)" (ui-settings_check.log.txt) — the card-4.E1 "value-bearing table for the React screens" promised at lib.rs:10–11 exists and drift-checks clean.
— Crate-wide panic hygiene claim holds for non-test code: grep for `.unwrap()`/`.expect(`/`panic!`/`unsafe`/`todo!`/`unimplemented!` across crates/settings-schema/src/*.rs and bin/ → zero non-comment hits.

## 3. Findings

### F-001 [S3] `Severity::Warning` is a dead advisory channel, and `parse_and_validate_str` is severity-blind: every diagnostic is fatal
Where: crates/settings-schema/src/lib.rs:90-95 (fatal path), crates/settings-schema/src/diagnostic.rs:15-20 (dead variant)
Code:
```rust
    let diags = validate(&schema, product);
    if diags.is_empty() {
        Ok(schema)
    } else {
        Err(LoadError::Invalid(render_all(&diags)))
    }
```
and the contract it cannot honor:
```rust
pub enum Severity {
    /// Build-breaking violation.
    Error,
    /// Advisory finding; regeneration still succeeds.
    Warning,
}
```
What is wrong: `Severity::Warning` is constructed nowhere in the crate (grep `Warning`/`Severity` over src/*.rs and bin/ → only the definition and its Display arm), and no consumer filters by severity: `parse_and_validate_str` (and therefore `load` and schema_gen via lib.rs:89) fails on ANY non-empty diagnostic vector, and flattens the typed `SchemaDiagnostic`/`Severity` (re-exported at line 56 as public API) into a rendered `String`.
Why it matters: The documented semantics "Advisory finding; regeneration still succeeds" exists in no code path. If a future validator rule emits a Warning (the enum invites exactly that), `cargo xtask schema-gen` and every `load` caller start failing on advisories — a silent contract break, because nothing at this seam distinguishes severities. This is the config-wave "declared but never read" pattern in miniature: a public severity channel, documented, produced by no one, consumed by no one.
What correct looks like: Either delete the variant until an advisory rule exists, or make the boundary severity-aware: `let errors: Vec<_> = diags.iter().filter(|d| d.severity == Severity::Error).collect();` fail on errors, render warnings to stderr, succeed.
Confidence: HIGH — Verification: read (greps listed in header; all validate/parse constructors observed to be error-severity — `Severity` unreferenced outside diagnostic.rs).

### F-002 [S4] "no `unsafe`/`unwrap`/`expect`/`panic` (workspace lints)" — the lint net does not cover `unreachable!`, which this file's tests use
Where: crates/settings-schema/src/lib.rs:46-47 (claim), 162-164 (use)
Code:
```rust
//! strings (doc 01 §2.3 (d)); no `unsafe`/`unwrap`/`expect`/`panic`
//! (workspace lints).
```
```rust
        let seed_text = std::fs::read_to_string(root.join("schemas/settings.toml"))
            .unwrap_or_else(|err| unreachable!("reading the committed seed: {err}"));
        let seed = toml::from_str::<toml::Table>(&seed_text)
            .unwrap_or_else(|err| unreachable!("parsing the committed seed: {err}"));
```
What is wrong: The workspace lint set (root Cargo.toml:53–64) denies `clippy::panic`, `unwrap_used`, `expect_used`, `todo`, `unimplemented` — but NOT `unreachable` (hence the repo-wide count of 4154 `unreachable!` in the baseline hygiene census). `unreachable!` is a panic macro; the doc's parenthetical "(workspace lints)" overstates what the lints enforce. The two uses here are in `#[cfg(test)]` code with error context attached — acceptable practice — but the claim says "this crate", unscoped.
Why it matters: A reader trusting the parenthetical believes panics are lint-impossible in the crate; they are only lint-impossible outside `unreachable!` (and outside `assert!`-family macros, also not denied). Precision debt on a safety-ish claim.
What correct looks like: Scope the claim ("non-test code; tests use `unreachable!` with context") or add `clippy::unreachable` to the crate's own lint exceptions discussion.
Confidence: HIGH — Verification: read (root Cargo.toml:53–64 lint list; baseline INDEX hygiene-counts: `unreachable!` 4154).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + `pub mod`/re-exports | 1–64 | REAL | — | G1–G4/D1–D6 claims verified (see §6); re-export list matches the module items (checked against parse.rs/validate.rs/emit.rs/runtime.rs/model.rs/diagnostic.rs signatures via grep). |
| `enum LoadError` | 67–81 | REAL | — | Io{path,source} carries the path; Invalid(String) is the F-001 flattening sink. thiserror; no panic paths. |
| `parse_and_validate_str` | 85–96 | REAL (defect) | S3 | F-001: severity-blind fatal path; structure destroyed via `render_all`. Correct otherwise: parse errors and validation errors both rendered with full diagnostics. |
| `load` | 100–127 | REAL | — | Reads schema (+ optional product.toml), maps IO errors with path, product syntax errors with path + toml error. Only caller: schema_gen.rs:102 (tooling lane, CI-verified). |
| `tests::committed_seed_validates_against_committed_product` | 136–186 | REAL | — | Loads the REAL schemas/settings.toml + product.toml; asserts empty failure string; independently re-counts `[[setting]]` rows (165–168) and `[areas].known` (169–174) via raw toml and compares to `schema.len()`/`schema.areas.len()` — a loader that drops or invents entries fails. Comment (155–160) honestly documents the c71cd5a 27→32 desync history that motivated it. `Err(_) => return` at 152–154 is dead-by-preceding-assert (harmless guard). |
| `tests::every_seed_default_ref_resolves_in_product_toml` | 188–212 | REAL | — | Err arm (196–199) same assert-then-return pattern. Asserts `refs.len() >= 20` (211) — deliberately loose shape floor per its comment ("assert the shape"); actual seed has 24 `default_ref =` lines (grep count). Weak but honest. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| committed_seed_validates_against_committed_product | YES | Mutation: parse_str drops every 10th entry → `schema.len()` (121) vs raw count mismatch → fail. Mutation: any D-rule check deleted → committed seed still clean → this test still passes (by design — rule enforcement is validate::tests' 20 cases, all green in baseline); this test proves file-cleanliness + loader fidelity, and its assert message names exactly that. |
| every_seed_default_ref_resolves_in_product_toml | YES (weakly) | Mutation: parse_str drops all `default_ref` fields → refs.len() == 0 < 20 → fail. Floor 20 < actual 24 — passes while the shape holds. |
Both ran and passed: baseline orchrun-test-workspace-nofailfast.log.txt:4899, :4913.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| G1 `emit::generate` TsTypes → `schemas/generated/settings.d.ts`; "the value-bearing table for the React screens lands with card 4.E1" | lib.rs:9–11 | REAL | File exists; ui/scripts/gen-settings-screens.mjs (card 4.E1) consumes the same schemas/settings.toml; `npm run settings:check` PASS in baseline ("10 ui areas, 102 ui entries, 121 total"). |
| G2 CliHelp golden / G3 Docs page generated + committed | lib.rs:12–15 | REAL | Baseline integration: `committed_outputs_have_no_drift ... ok` :4934, `regeneration_is_idempotent_byte_for_byte ... ok` :4936 (orchrun log). |
| G4 runtime embedded via `include_str!`; `validate_value` is the `config.set-setting` seam (type/range/enum, invalid-argument, never silent coercion); `schema_meta` backs `config.get-setting` | lib.rs:16–20 | REAL | runtime.rs:18 `include_str!`; apps/cli/src/host.rs:1400–1403 (set: validate → InvalidArgument), :1371/1375–1385 (get: schema_meta fields); shipped dep apps/cli/Cargo.toml:61. |
| `validate::validate` enforces D1–D6 with "file + line + entry path + rule + fix hint" | lib.rs:24–29 | REAL | 20 named d1–d6/e-gate tests + diagnostic render tests all `ok` in baseline (orchrun log :4874–4923). |
| product.toml join is data-level, not a crate edge; future direct call [PROPOSAL] | lib.rs:31–40 | REAL-UNPROVEN (honestly labeled) | validate takes `Option<&toml::Value>` (validate.rs:92); no product-config dependency exists; marked PROPOSAL — no false claim. |
| "no `unsafe`/`unwrap`/`expect`/`panic` (workspace lints)" | lib.rs:46–47 | REAL (non-test) / overstated (lint scope) | F-002: zero non-comment hits crate-wide for the denied macros; `unreachable!` used in tests, not covered by lints. |
| schemas/settings.toml header: "Regenerate with `cargo xtask schema-gen`; a second run is byte-identical" | schemas/settings.toml:8–9 | REAL | Baseline `regeneration_is_idempotent_byte_for_byte ... ok`; xtask bridge exists (tools/xtask/src/cmd/schema_gen.rs, subprocess pattern documented). |

## 7. Dead / duplicated / unreachable code
- `Severity::Warning` (diagnostic.rs:19) — dead variant, no producer, no severity-aware consumer (F-001).
- `parse_and_validate_str` is `pub` with one in-crate caller (`load`); as a public seam it is forward-looking for the doc's [PROPOSAL] product-config join — not flagged dead, but noted: nothing outside the crate calls it today (grep).
- No unreachable-from-shipped code in this file: both helper fns feed the schema_gen tooling lane; the crate's runtime lane is separately shipped via apps/cli.

## 8. Dependencies & flags
- Crate deps: toml + thiserror (per crate family comment); lib.rs itself uses std only beyond the re-exports. No feature gates, no unsafe, no allow-attributes.
- `[lints] workspace = true` inherited; clippy `-D warnings` clean in baseline (runs #5/#6).
- Downstream shipped consumers: apps/cli (lumen), crates/plugin-host-wasm (supervisor validates plugin-provided setting values, supervisor.rs:171).

## 9. Open questions for the next agent
- A02 (runtime.rs): `embedded()` validates with `validate(&schema, None)` — confirm no D1 rule is silently weakened in the embedded lane (product join skipped by design?).
- A08 (schema_gen.rs bin): exit-code contract 0/1/2 — verify `--check` drift path really exits 1 (integration test `stale_output_drift_fails_check_with_exit_one` says yes, baseline ok) and that warnings, if ever added, have a defined exit behavior (ties to F-001).
- UI lane (Wave 12+): the UI renders settings screens from the same schema (102 ui entries) — do UI setting changes reach `config.set-setting`/`validate_value`, or does the tauri desktop lane store them separately? (Wave-8 `audio.set-eq` precedent: UI seams have awaited nonexistent backends before.)
