# AUDIT: crates/product-config/tests/semantic.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 291 / 19 (helpers semantic_diagnostics, one, asset_base; 17 #[test] fns) |
| File verdict | REAL (strict semantic-pass tests incl. honest deferral negatives; executed in orchestrator baseline) |
| Reachable from shipped entry point? | The code UNDER TEST (`validate_semantics`, `SemanticInputs`, `FeatureRegistry`) has ZERO callers outside crates/product-config's own tests (rg over apps/, tools/, crates/: no hits). Only apps/control-server uses product-config, and only `load_file` (structural) — apps/control-server/src/main.rs:145-154. So the semantic pass is test-only enforcement today. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | diff assigned fixtures vs product.toml (exit 1); od hexdumps of the 4 asset files (exit 0); rg validate_semantics/SemanticInputs callers (exit 1 — none outside crate); sed verifications (exit 0). Baseline: cargo-test-workspace.txt — `Running tests/semantic.rs ... running 17 tests / test result: ok. 17 passed`. |

## 1. One-paragraph verdict
The strictest test file in the product-config suite: every semantic rule (cross-field min_width, schema_version↔migrations, telemetry↔privacy-url, id consistency, reserved short_name, inverted migrations, 3 injected enums, catalog-key existence, F1/F2 feature intersection, asset existence + magic bytes) is proven with an exact-one-diagnostic assertion, pinned rule text, found/expected values, and lines; deferral behavior (no listing → no diagnostic) is honestly proven with two dedicated negative tests instead of being hidden. Two real gaps: the "clean pass" over the real product.toml only runs with `SemanticInputs::none()`, which defers every injected/asset check — so the four REAL missing brand assets (proven by the xtask config-validator baseline FAILURE) are invisible to this suite; and the validate_semantics layer has no production caller at all.

## 2. What actually works (verified)
- Baseline: `Running tests/semantic.rs (target/debug/deps/semantic-5f11f3123bd85b98) / running 17 tests / test result: ok. 17 passed; 0 failed`. Package exit_code=0.
- Asset fixture bytes verified by od: icon.ico = `4e 4f 54 2d 49 43 4f` ("NOT-ICO" — deliberately wrong), icon-512.png starts `89 50 4e 47 0d 0a 1a 0a` (valid PNG magic), icon.icns starts `icns`, logo.svg starts `<?xml` — consistent with the magic table in src/semantic.rs:502-520 and the test's "exactly one diagnostic total" (260).
- migrations-inverted.toml diff vs product.toml: adds `entries = [{ from = 3, to = 2, summary = "inverted", migrator = "m" }]` — matches the README mutation and the two asserted diagnostics (from<to + schema_version desync).

## 3. Findings
F-001 [S3]
Where: crates/product-config/tests/semantic.rs:278-291
Code:
```
    let lines = product_config::lines::LineIndex::new(&src);
    let diags = validate_semantics("product.toml", &config, &SemanticInputs::none(), &lines);
    assert!(diags.is_empty(), "{}", render_all(&diags));
```
What is wrong: the test's name and placement suggest the committed root product.toml passes the semantic rules — but `SemanticInputs::none()` defers asset existence/magic, injected enums, catalog keys, and F1/F2 (proven by the two deferral tests at 162-173 and 231-239). The real tree FAILS the deferred checks: audit/runs/baseline/xtask-config-validator.txt:10-15 reports `violation: brand.icon_ico: asset path 'assets/brand/icon.ico' does not exist under the root` (×4) and `result: FAILURE (4 violation(s))`. So "root product.toml passes" is true only for the in-document subset. The deferral is a documented design (the tool supplies listings), but nothing anywhere supplies them for the real root — the full semantic pass is never run against the real product.toml. Why it matters: the strongest validator in the repo never sees the real config with real inputs; a rebrand that ships broken asset paths is caught only by the separate, weaker xtask v0. What correct looks like: a test (or gate step) running validate_semantics on the real root with base_dir=repo_root and real listings.
Confidence: HIGH — Verification: read + baseline citation.

F-002 [S3]
Where: crates/product-config/src/semantic.rs (code under test) — no production caller; evidenced from this suite being the sole consumer
Code:
```
use product_config::{FeatureRegistry, SemanticInputs, Stage, render_all, validate_semantics};
```
What is wrong: rg over apps/, tools/, crates/ shows `validate_semantics`/`SemanticInputs` referenced ONLY inside crates/product-config (src + tests). The semantic layer — cross-field rules, F1/F2, injected enums, asset magic — is library code whose only driver is this test file. The xtask `config-validator` that runs in CI is a separate v0 that explicitly does NOT use this crate (xtask-config-validator.txt:7 note: "pattern constraints not enforced by this v0 ... the product-config crate's CI suite owns them"). Two validators, one wired, one complete-but-unwired. Why it matters: rules enforced only by fixtures can regress in wiring forever without user-visible effect; the doc 07 §3 story ("config-validator backs C4") is split across two implementations. What correct looks like: xtask config-validator should call product-config (structural + semantic) instead of duplicating a subset.
Confidence: HIGH — Verification: rg (exit 1 for external callers) + baseline file.

F-003 [S4]
Where: crates/product-config/tests/semantic.rs:243-245 + fixtures/README.md:62-63
Code:
```
fn asset_base() -> PathBuf {
    common::repo_root().join("crates/product-config/tests/fixtures/assets")
```
What is wrong: fixtures stale-base issue (see fixtures_matrix.rs F-001) applies to assets-wrong-magic.toml too — it is an old product.toml copy plus the asset-path mutation; the README's one-mutation regeneration contract is false for it as for the other 39 fixtures. Test remains valid (content-marker assertions).
Confidence: HIGH — Verification: diff.

## 4. Per-function table (test inventory — all 17 #[test])
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cross_field_min_width_must_be_below_width | 46-55 | REAL | — | Stage::Semantic, found "1600", line pinned. |
| cross_field_schema_version_must_match_migrations | 58-65 | REAL | — | found "2" expected "1". |
| cross_field_telemetry_requires_privacy_url | 68-74 | REAL | — | rule mentions urls.privacy; hint mentions privacy policy. |
| id_consistency_rule_3_3_b | 77-93 | REAL | — | expected "dev.lumen.Lumen"; negative: macos.bundle_id empty. |
| reserved_short_name_is_semantic | 96-102 | REAL | — | "vlc" lowercase — semantic-pass half of reserved-name. |
| migrations_inverted_entry_is_rejected | 105-117 | REAL | — | TWO diagnostics (from<to + schema_version desync) — chain effect covered. |
| injected_enum_theme_must_exist_in_themes_dir | 122-133 | REAL | — | Listing [dark, light]; expected "dark, light". |
| injected_enum_locale_must_exist_in_locales_dir | 136-146 | REAL | — | Listing [en-US] — matches the real locales/ dir (ls: en-US.toml only). |
| injected_enum_profile_must_exist_in_profiles_dir | 149-159 | REAL | — | |
| injected_enums_are_deferred_when_no_listing_is_supplied | 162-173 | REAL | — | Honest deferral negative. |
| title_idle_key_must_exist_in_the_reference_catalog | 178-193 | REAL | — | Catalog listing; negative for description_key. |
| f1_unknown_feature_id_is_rejected | 198-212 | REAL | — | Registry from_status; hint "config alone enables nothing". |
| f2_exposed_but_unimplemented_feature_is_rejected | 215-228 | REAL | — | expected contains "= false". |
| f1_f2_are_deferred_when_no_registry_is_supplied | 231-239 | REAL | — | Honest deferral negative. |
| brand_icon_with_wrong_magic_bytes_is_rejected | 248-261 | REAL | — | base_dir=fixture assets; asserts EXACTLY one diagnostic total — the other 3 assets must pass magic (verified via od). |
| missing_brand_asset_is_rejected | 264-274 | REAL | — | must exist + missing.svg in found. |
| root_product_toml_passes_semantic_none | 279-291 | REAL | S3 | F-001 — "none" defers the checks the real tree fails. |
| helpers semantic_diagnostics/one/asset_base | 12-41, 243-245 | REAL | — | Fail-fast arms; render_all in panic text. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 15 rule tests | YES | Exact-one-diagnostic + rule text + found/expected (+ line where locatable). A semantic pass that skips a rule, double-reports, or misstates values fails. Baseline: 17/17 pass. Not re-executed locally (sandbox linker fault — log). |
| Deferral negatives (162-173, 231-239) | YES | Would fail if deferral turned into silent enforcement or spurious diagnostics. |
| root_product_toml_passes_semantic_none | YES but weak | Fails on any in-document regression, but cannot see deferred-rule violations — the real tree currently HAS 4 (baseline config-validator FAILURE). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the clean pass over the committed root product.toml" | semantic.rs:2-3 (self) | SHALLOW | Passes only with inputs=none(); asset/F1/F2/enum/catalog checks deferred (F-001). |
| fixtures = one mutation of current product.toml | tests/fixtures/README.md:3-4 | FALSE | Same evidence as fixtures_matrix.rs F-001 (39/40 stale, diff-verified on assigned 4). |

## 7. Dead / duplicated / unreachable code
Local `one()` duplicates fixtures_matrix.rs's `one()` verbatim (both 25-41) — test-local duplication, acceptable. The code under test (validate_semantics) is itself unreachable from shipped entry points (F-002).

## 8. Dependencies & flags
Test-only; exercises product_config::{validate_semantics, SemanticInputs, FeatureRegistry, lines::LineIndex}. No features.

## 9. Open questions for the next agent
- Who owns the xtask config-validator ↔ product-config unification (F-002)? Doc 07 §9 C4 promises `cargo xtask config-validator` backed by "schema + semantics + D-rules" — the wired v0 does existence checks only.
