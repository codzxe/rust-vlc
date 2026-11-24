# AUDIT: crates/settings-schema/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2025-06-14 |
| Lines / functions | 213 / 2 prod fns + 2 enums + re-exports + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — re-exports consumed by apps/cli/src/help.rs:33-34 (generate_str, embedded), apps/cli/src/host.rs:1371/1382/1400/1402 (schema_meta, validate_value), crates/plugin-host-wasm/src/supervisor.rs (runtime::embedded, validate_value). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p settings-schema --lib` -> 43 passed / 0 failed (my run, includes both lib.rs tests); rg ui/ for settings.d.ts consumers -> none; sed quote verification. |

## 1. One-paragraph verdict
A clean crate root: re-export surface, two file-level loaders (`parse_and_validate_str`, `load`) that genuinely parse + D-rule-validate + product-join, and two real integration tests that load the committed seed against the committed product.toml (pass — ran). The loader chain feeds the schema_gen dev binary; the runtime path (embedded) is what ships. The one real problem: G1's generated TypeScript surface (`schemas/generated/settings.d.ts`) is committed and drift-gated but has ZERO consumers — the UI never imports it, so the "TS type surface" is a dead artifact.

## 2. What actually works (verified)
— `cargo test -p settings-schema --lib` -> `43 passed; 0 failed`, including `tests::committed_seed_validates_against_committed_product ... ok` and `tests::every_seed_default_ref_resolves_in_product_toml ... ok` — the committed seed + product.toml round-trip the full parse + D1-join + D2-D6 pipeline with zero findings.

## 3. Findings
F-001 [S3]
Where: crates/settings-schema/src/lib.rs:9-11 vs ui/ (whole tree)
Code:
```
//! * **G1** [`emit::generate`] with [`emit::GenTarget::TsTypes`] →
//!   `schemas/generated/settings.d.ts` (the TS *type surface*; the
//!   value-bearing table for the React screens lands with card 4.E1).
```
What is wrong: rg over ui/ for `settings\.d\.ts|SettingsSchema|schemas/generated` returns NO files. The G1 artifact is generated, committed, and drift-checked by xtask schema-gen --check — but nothing in the React app imports it; the UI's settings screens (controls.tsx etc.) do not use the type surface. The doc comment sells a "TS type surface" whose only consumer is the drift check that regenerates it. Why it matters: dead generated output + a doc claim that implies the UI is typed by it. What correct looks like: either ui/src imports the .d.ts (or a copied bridge like the tokens.css pipeline) or the doc marks G1 as future-work like the value table. Confidence: HIGH — Verification: ran rg (ui/ -> 0 hits); read.
F-002 [S4]
Where: crates/settings-schema/src/lib.rs:46-47 vs lib.rs:162-164
Code (doc claim, :46-47):
```
//! strings (doc 01 §2.3 (d)); no `unsafe`/`unwrap`/`expect`/`panic`
//! (workspace lints).
```
vs the test (:161-164):
```
        let seed_text = std::fs::read_to_string(root.join("schemas/settings.toml"))
            .unwrap_or_else(|err| unreachable!("reading the committed seed: {err}"));
        let seed = toml::from_str::<toml::Table>(&seed_text)
            .unwrap_or_else(|err| unreachable!("parsing the committed seed: {err}"));
```
What is wrong: the "no panic" claim holds for src/, but the in-crate tests reach for `unreachable!` (panic family) 4x in lib.rs's tests. Harmless (tests), but the blanket doc sentence overstates; the repo-wide pattern is already flagged in the wave-0 baseline (4126 `unreachable!`). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/settings-schema/src/lib.rs:209-211
Code:
```
        // The load above already proves D1 resolution; assert the shape:
        // the seed mirrors product.toml areas heavily via default_ref.
        assert!(refs.len() >= 20, "expected >=20 default_refs, got {refs:?}");
```
What is wrong: the test's name promises "every seed default_ref resolves" but the body asserts only a count >= 20; the resolution proof is delegated to the first test's zero-findings load (which does prove it — D1 fires on unresolvable refs). Honest comment, weak name. Confidence: HIGH — Verification: read + ran (both pass).

## 4. Per-function table (213 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LoadError enum | 67-81 | REAL | — | Io/Invalid with source chaining |
| parse_and_validate_str | 85-96 | REAL | — | parse -> validate -> render_all on failure |
| load | 100-127 | REAL | — | Reads schema + optional product.toml; dev-lane (schema_gen bin + xtask) |
| tests::committed_seed_validates… | 136-186 | REAL | S4 | Strong: zero-findings + loader-count vs independent raw-TOML read (OF-2 guard) |
| tests::every_seed_default_ref… | 188-212 | REAL | S4 | F-003: shape-only assertion |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_seed_validates_against_committed_product | YES | Zero-findings assertion + count assertions derived from an independent TOML read (a loader dropping rows fails it) |
| every_seed_default_ref_resolves_in_product_toml | PARTLY | Count-only; resolution itself proven by the other test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| G1 → settings.d.ts "the TS type surface" for React screens | lib.rs:9-11 | PAPER-ONLY (no consumer) | F-001 rg: 0 ui/ hits |
| "no unsafe/unwrap/expect/panic (workspace lints)" | lib.rs:46-47 | TRUE for src, overstated for tests | F-002 |
| G4 runtime::validate_value is the config.set-setting seam | lib.rs:16-19 | REAL | host.rs:1402 (see validate.rs audit F-002 for the dead-store caveat) |

## 7. Dead / duplicated / unreachable code
- schemas/generated/settings.d.ts: generated + committed + drift-checked, zero importers (F-001).
- `load`/`parse_and_validate_str`: reachable only via the schema_gen bin / xtask (dev lane) — by design.

## 8. Dependencies & flags
toml, thiserror. No feature gates. No unsafe.

## 9. Open questions for the next agent
- Card 4.E1 (value-bearing settings table for React): does it exist yet, and will it import settings.d.ts or duplicate the types?
