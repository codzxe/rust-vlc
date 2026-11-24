# AUDIT: crates/localization/src/fallback.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines / functions | 317 / 3 functions + 2 Display impls + 3 types (+6 tests) |
| File verdict | UNREACHABLE (real, correct, tested — and linked by nothing) |
| Reachable from shipped entry point? | NO. No Cargo.toml in the workspace declares a dependency on the `localization` crate (checked every apps/*/Cargo.toml and crates/*/Cargo.toml: zero path-deps; `Localizer` appears only in crates/localization/src/{fallback,lib}.rs). Not in lumen-cli, desktop, control-server, or any other crate. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | per-manifest rg loop for `localization`/`^localization` deps (exit 0/1); rg "Localizer" repo-wide (only 2 crate-internal hits); rg "localization::" across apps/ crates/ (crate-internal only); ls tools/xtask/src/cmd/ (no l10n-audit command); baseline citation cargo-test-workspace.txt:2904-2909 (6/6 fallback tests pass); sed quote verification |

## 1. One-paragraph verdict
The fallback chain is implemented correctly and honestly: `build` produces requested → language-primary parent → product default → en-US with duplicate collapse, `translate` walks it per-key, renders with the *translating* catalog's locale (plural rules follow the resolving locale — the right choice), never panics, records a `MissingKey` diagnostic, and degrades to `missing:<key>` (debug) / raw key (release) exactly as doc 07 §6.3 specifies; six meaningful tests pin all of this and pass in the baseline. The problem is not the chain, it is that the chain drives nothing: no package in the workspace depends on the `localization` crate, the shipped UI implements its own fallback in TypeScript (`catalog[key] ?? messages[key]`, ui/src/i18n/messages.ts:3013-3016), and doc 07 §6.4's claimed runner `cargo xtask l10n-audit` does not exist in tools/xtask/src/cmd/. This is parallel machinery: a second, richer implementation of a chain that already runs — in another language, elsewhere.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:2904-2909: all 6 `fallback::tests::*` pass (chain order, distinct default, requested-catalog render, per-key parent+reference fallback, missing-everywhere diagnostic, locale selection).
— verified by reading ui/src/i18n/messages.ts:3013-3016: the shipped UI's fallback is the TS one-liner; the Rust chain is not consulted.
— Chain correctness spot-checks (read): duplicate collapse when requested == default == reference (test chain_order_matches_doc07 expects `["pt-BR", "pt", "en-US"]`, i.e. default en-US and reference en-US collapse); absent requested+parent falls to default then reference (chain_with_distinct_default expects `["es", "en-US"]`).

## 3. Findings
F-001 [S2]
Where: crates/localization/src/fallback.rs:93-161 (whole `Localizer`); consumption: none
Code:
```
    /// Translate `key` with `args`, walking the fallback chain per key.
    /// Never panics (doc 07 §6.3).
    pub fn translate(&self, key: &MessageKey, args: &[(&str, Arg)]) -> Translation {
```
What is wrong: `Localizer`, `Translation`, and `select_locale` are exported (lib.rs:110 `pub use fallback::{Localizer, Translation, select_locale}`) and consumed by zero callers outside the crate's own tests. No workspace manifest declares a dependency on `localization` (per-manifest rg over apps/ and crates/: no hits). The doc that specifies this behavior presents it as the product's runtime behavior — doc 07 §6.3: "The chain is total — a missing translation can never crash rendering" — but no shipped binary contains this chain; the runtime that actually renders UI text is the TS layer with its own separate fallback. Why it matters: the mechanism with plural-correctness, per-key walk, parent stepping, and miss diagnostics — the hard parts — is dead code, while the live implementation (TS) has none of those properties (single-step en-US fallback, no parent locales, no plural-aware resolution). Any claim that "missing translations degrade honestly with diagnostics" is true only of code that never runs. What correct looks like: wire the UI/engine message rendering through this crate (or delete it and admit the TS layer is the mechanism). Confidence: HIGH — Verification: read + rg (exhaustive manifest scan).
F-002 [S3]
Where: docs/07-config-driven-system.md:678 vs tools/xtask/src/cmd/ (directory listing)
Code:
```
Run by `cargo xtask l10n-audit` (backed by `crates/localization::audit_catalogs`, doc 01 §2.3):
```
What is wrong: `cargo xtask l10n-audit` does not exist — tools/xtask/src/cmd/ contains check_architecture, check_identity, check_panics, codegen, compat_matrix, config_validator, feature_audit, fixtures, gate, lint_literals, mod, parity_report, rebrand, sbom, schema_gen, tokens, toml_lite, visual, web_budget — no l10n-audit, and tools/xtask/Cargo.toml does not depend on the localization crate. The audit rules run only as the localization crate's own `cargo test` suite (tests/seed_integrity.rs, tests/audit_proof_matrix.rs). The doc names a runner that was never built. Confidence: HIGH — Verification: read + ls + rg.
F-003 [S3]
Where: crates/localization/src/fallback.rs:109-119
Code:
```
        for preference in preferences {
            for catalog in available {
                if *catalog.locale() == preference
                    && !chain
                        .iter()
                        .any(|chosen| chosen.locale() == catalog.locale())
                {
                    chain.push(catalog.clone());
```
What is wrong: if `available` ever contains two catalogs with the same locale (two files for one tag, or a loader bug), the second is silently dropped — its keys become unreachable with no diagnostic. Today's loaders produce one catalog per file name and the locales-duplicate fixture is a test-only error case, so this is latent; but this function is the one place that could detect and report it. Silent data loss on realistic misconfiguration. Confidence: MEDIUM — Verification: read (no caller exists to build the scenario).
F-004 [S4]
Where: crates/localization/src/fallback.rs:100 (with locale.rs:129-137)
Code:
```
        let parent = requested.language_primary();
```
What is wrong: `language_primary` collapses `zh-Hans-CN` straight to `zh`, skipping the BCP-47 intermediate `zh-Hans` step; the chain can never consult a script-region catalog when a language-primary catalog exists. This matches doc 07 §6.3 as written ("its language-primary parent"), so it is a documented design limit, not a spec violation. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/localization/src/fallback.rs:56-59
Code:
```
    /// The locale whose catalog resolved the key (`None` = not found
    /// anywhere — the text is the debug `missing:<key>` form or the raw
    /// key in release builds).
```
What is wrong: documentation nit — "the raw key in release builds" is accurate, but the struct doc at line 54 says "Always present; degraded visibly on misses" while release builds degrade *invisibly* (raw key looks like a working label to a user). Cosmetic honesty gap in a comment. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| select_locale | 27-29 | REAL-UNPROVEN* | - | correct one-liner per doc §6.3; tested; *no runtime caller |
| FallbackDiagnostic::Display | 38-49 | REAL | - | trivial |
| RenderOrFallback::Display | 74-81 | REAL | - | trivial |
| Localizer::build | 98-121 | REAL | S3 | correct chain + dedup; silent same-locale drop (F-003) |
| Localizer::locales | 124-126 | REAL | - | test/debug accessor, used by tests |
| Localizer::translate | 130-160 | REAL | S2 | never panics, per-key walk, correct plural locale; UNREACHABLE (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| chain_order_matches_doc07 (196) | YES | pins exact order incl. default/reference collapse |
| chain_with_distinct_default (211) | YES | pins absent-requested handling [es, en-US] |
| translates_with_requested_catalog (233) | YES | pins text + resolved_locale + empty diagnostics |
| falls_back_per_key_to_parent_then_reference (252) | YES | pins per-key resolution from pt and en-US |
| missing_everywhere_records_diagnostic_and_never_panics (281) | YES | pins missing: prefix, None locale, 1 diagnostic |
| locale_selection_prefers_setting_then_os_then_default (304) | YES | all three preference combos |
All six run only against in-test fixture catalogs; no test loads the real `locales/` directory through `Localizer::build` (that provenance lives in seed_integrity.rs).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "requested locale → language-primary parent → product default → en-US → (debug) missing:<key>" | doc 07 §6.3 / fallback.rs:3-9 | REAL | implementation at 98-152 matches exactly; 6 tests pass (baseline:2904-2909) |
| "The chain is total — a missing translation can never crash rendering" | doc 07 §6.3 | REAL-UNPROVEN as a product claim | true of this code; false as a claim about the shipped app, which does not link this crate (F-001); the shipped UI's TS fallback is the only live chain |
| "Run by `cargo xtask l10n-audit` (backed by crates/localization::audit_catalogs)" | doc 07 §6.4 | FALSE | command does not exist in tools/xtask/src/cmd/ (F-002) |
| "plural rules follow the translating locale, not the requested one" | fallback.rs:85-87 | REAL | render is called with catalog.locale() (133); ru plural fixtures exist |

## 7. Dead / duplicated / unreachable code
- The whole file is unreachable from shipped entry points (F-001): zero dependents on the crate.
- Duplicated concept: ui/src/i18n/messages.ts:3013-3016 implements the "doc 07 §6.3 chain totality" fallback independently in TS.

## 8. Dependencies & flags
crate::{catalog, key, locale, message} only; no external crates; no feature gates; `cfg!(debug_assertions)` gates the missing: form.

## 9. Open questions for the next agent
- Is there a plan card that wires `Localizer` into the desktop/UI bridge? If the wiring card exists but is unfinished, F-001 downgrades to "scheduled"; I found no reference in doc 07 §6 to a pending wiring card.
- locales/ directory: how many catalogs exist and who loads them at runtime (nobody found on the Rust side; the TS side reads locales via its own pipeline — verify in the UI audit).
