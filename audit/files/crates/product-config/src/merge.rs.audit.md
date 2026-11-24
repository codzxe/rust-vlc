# AUDIT: crates/product-config/src/merge.rs
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 487 / 8 (Layer, Layer::precedence, LayerSource::new, strip_nulls, layer, resolve_layers, layer_name, record_provenance, blame, merge_over) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO. Only caller outside the crate: none. `resolve_layers`/`layer`/`load_partial_str` are invoked exclusively by crates/product-config/tests/merge.rs and tests/web_budgets.rs (rg over apps/, crates/, tools/: only product-config itself). apps/control-server — the sole shipped consumer of the product-config crate — calls only `load_file` (apps/control-server/src/main.rs:154). No app loads a user-preferences layer or calls the merge API (rg `user.toml|preferences|resolve_layers` over apps/ -> no matches). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `target/debug/deps/merge-da548c1b4875dc3f` (run in crates/product-config) -> 15 passed, exit 0; rg callers of `resolve_layers|merge::layer|LayerSource|merge_over` over crates/+apps/+tools/; rg `product-config` in all Cargo.tomls (Grep tool); baseline audit/runs/baseline/cargo-test-workspace.txt:5023-5187 product-config exit 0. |

## 1. One-paragraph verdict
The merge engine itself is real and correctly implements doc 07 §4.4: adjacent-pair strict-precedence enforcement (transitively complete), per-layer schema validation before merge, hand-written deep-merge that I verified field-by-field against PartialProduct (complete coverage, no silently dropped keys; `[web]` correctly rejected as unknown key via `deny_unknown_fields`), array-replace policy, and post-merge cross-field re-validation with layer blame. I ran the 15-test merge suite myself (exit 0). The two defects that matter: cross-field blame only consults the REPORTED key path, so a violation caused by a layer setting one participant of a rule (e.g. `window.width`) is mis-attributed to the base file `product.toml`; and the whole layered-resolution API is dead weight in the shipped product — no entry point anywhere loads a user layer, making doc 07 §4.2's "user preferences — the v1 override layer" a library-only reality.

## 2. What actually works (verified)
— verified by `target/debug/deps/merge-da548c1b4875dc3f` (run in crates/product-config) -> `test result: ok. 15 passed; 0 failed`, exit 0: precedence ordering, deep merge, array replace, determinism (100 iterations), cross-field blame on the reported key, layer-list validation, programmatic-partial validation.
— verified by field-by-field comparison of merge_over (merge.rs:297-487) against PartialProduct (partial.rs:20-165): every overridable field is handled; model.rs `web: Option<WebSection>` is intentionally non-overridable and `#[serde(deny_unknown_fields, default)]` on PartialProduct makes a layer `[web]` table a loud unknown-key error — matches the model.rs:29-31 contract.

## 3. Findings
F-001 [S3]
Where: crates/product-config/src/merge.rs:105-112 (contract) vs 279-293 (blame) and 210-215 (fallback)
Code:
```
/// stage 3 re-runs the pure cross-field rules over the resolved config so a
/// combination that is legal per-key but illegal as a whole (e.g. a user
/// `min_width` above the product `width`) fails with an actionable
/// diagnostic attributed to the highest layer that set an involved key.
```
What is wrong: `blame()` looks up only the diagnostic's `key_path` (and its prefixes) in the provenance map — the OTHER keys involved in the rule are never consulted. Reachable mis-attribution with the committed product.toml: base has `min_width = 940` (product.toml:70); a user layer sets `width = 500` (per-key legal: schema floor 480, proven by test t12's "must be >= 480"). Cross-field rule reports `window.min_width` (semantic.rs:221-232) — a base-set key — so `blame` finds nothing, and the diagnostic keeps the hard-coded fallback `file: "product.toml"` (merge.rs:212) even though the USER layer caused the violation. Same shape for the derived-id rule: user sets `identity.name`, violation reported at `identity.ids.windows.app_user_model_id` (base-set) → blames product.toml. Why it matters: the doc promises attribution to "the highest layer that set an involved key"; the operator is told to fix the wrong file. What correct looks like: rules report all involved key paths (or a set), and blame takes the max-precedence layer among them. Confidence: HIGH — Verification: read + executed test suite (the existing test only covers the reported-key case, tests/merge.rs:255-273, not the inverse).
F-002 [S3]
Where: crates/product-config/src/merge.rs:92-103 (`layer`) and 113-117 (`resolve_layers`) — reachability
Code:
```
/// v1 convenience: product defaults + one user layer (card 1.B1 scope).
pub fn layer(
```
What is wrong: The v1 user-override layer is wired to nothing shipped. rg over apps/, crates/, tools/: `resolve_layers`/`layer`/`load_partial_str` are called only from product-config's own tests. lumen-cli, desktop, desktop-tauri and control-server never construct a LayerSource; control-server uses only `load_file` for base product.toml (apps/control-server/src/main.rs:154). Doc 07 §4.1/§4.2 makes "User preferences (§4.2 layer 4) — the v1 override layer" the v1 scope; that scope exists as library + tests only. No "honest gap" note covers it (unlike runtime.rs's explicit host-gap register). Why it matters: the entire precedence machinery — the file's stated purpose — has zero production blast radius; doc 07 §4 reads as shipped behavior. Confidence: HIGH — Verification: ran (exhaustive rg).
F-003 [S3]
Where: crates/product-config/src/merge.rs:169-196
Code:
```
    // Stage 1: per-layer validation of every present value (§4.4 rule 4).
    let empty_lines = LineIndex::new("");
```
What is wrong: Stage-1 layer diagnostics are built against an empty LineIndex, and `LayerSource` carries no line data — so an out-of-range value in a real user.toml merged via `resolve_layers` reports `line: None` even though the same error via `load_partial_str` reports `Some(2)` (proven by tests t10/t12, which go through `load_partial_str`, NOT through resolve_layers' stage 1). The line-info machinery (lines.rs) is discarded on the merge path. Blast radius today: zero (F-002 — nothing ships this path), but any future host gets line-less diagnostics. Confidence: HIGH — Verification: read (Ctx.line_of on empty index; t10/t12 assert Some(2) only on the load path).
F-004 [S4]
Where: crates/product-config/src/merge.rs:190
Code:
```
        record_provenance(&json, "", &source.file, &mut provenance);
```
What is wrong: provenance is keyed by `file: String` only, not (layer, file); a host passing two layers with the same file name (e.g. two LayerSource both named "user.toml") makes blame indistinguishable. Precedence is still correct (later insert wins = higher layer), so this is purely diagnostic quality. Confidence: LOW — Verification: read.
F-005 [S4]
Where: crates/product-config/src/merge.rs:74-90
Code:
```
fn strip_nulls(value: serde_json::Value) -> serde_json::Value {
```
What is wrong: JSON round-trip serialization (`serde_json::to_value(&source.partial)` at 173) is used to obtain the layer's key tree; `Option::None` fields relying on `skip_serializing_if` vs explicit null serialization is an implicit coupling — today PartialProduct derives plain Serialize (None -> null -> stripped), correct, but any future `#[serde(skip_serializing_if = "Option::is_none")]` on a partial field would change validation behavior silently (field absent instead of null-stripped — same result today, fragile invariant, undocumented). Confidence: LOW — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Layer (enum) | 24-37 | REAL | — | 6 layers per doc 07 §4.1; Profile/Admin/Flags/Session have NO loaders anywhere (v1) |
| Layer::precedence | 41-50 | REAL | — | 1..6, matches §4.1 order |
| LayerSource::new | 65-71 | REAL | — | plain ctor |
| strip_nulls | 77-90 | REAL | S4 | F-005 fragility note |
| layer | 93-103 | REAL | S3 | convenience; zero shipped callers (F-002) |
| resolve_layers | 113-231 | REAL | S3 | 3-stage pipeline correct; blame gap (F-001), line-less stage 1 (F-003) |
| layer_name | 233-242 | REAL | — | diag helper |
| record_provenance | 245-275 | REAL | S4 | path->file leaves only; arrays indexed (matches semantic key paths `migrations.entries.{i}.from`) |
| blame | 279-293 | REAL | S3 | exact + prefix lookup only; no involved-key set (F-001) |
| merge_over | 297-487 | REAL | — | field-complete vs PartialProduct (verified); tables deep, scalars/arrays replace; features per-key insert-merge (matches "tables deep-merge" policy) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| t1_only_product_layer_resolves_product_defaults | YES | asserts resolved == base + concrete values |
| t4_user_overrides_product | YES | asserts overridden + untouched keys |
| t10_unknown_key_in_user_layer_is_an_error_with_hint | YES | asserts rule text, hint, file, line Some(2) |
| t12_out_of_range_in_user_layer_is_an_error | YES | asserts ">= 480", file, line |
| t13_array_policy_replace | YES | asserts full replacement |
| t15_layers_agree_resolve_once | YES | asserts equality with base |
| t16_reset_returns_the_next_lower_value | YES | asserts pre/post values |
| deep_merge_replaces_only_the_named_leaf | YES | asserts sibling keys untouched (nested ids) |
| features_table_merges_per_key | YES | asserts per-key merge + base keys kept |
| merge_is_deterministic_across_runs | WEAK | 100 identical sequential runs of a pure function — determinism against map-iteration-order bugs is plausible (BTreeMap) but this can't catch cross-run state; acceptable for a pure fn |
| programmatic_partial_out_of_range_is_rejected | YES | catches the non-file layer path |
| cross_field_violation_after_merge_is_blamed_on_the_layer | YES (but narrow) | covers ONLY the reported-key case; the inverse (layer sets the other involved key) is untested — F-001 |
| layer_list_must_be_ordered | YES | asserts the <layer-list> diagnostic |
| product_layer_is_not_a_list_entry | YES | asserts "resolution base" rule |
| layer_convenience_is_product_plus_user | YES | asserts both override + fallthrough |
Suite executed by me: 15 passed, exit 0.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "tables deep-merge, scalars replace, arrays replace (v1 policy)" | merge.rs:8-9, doc 07 §4.4 | REAL | merge_over field walk + tests t13/deep_merge_replaces_only_the_named_leaf (ran, 15/15) |
| "every layer's values are validated against the same schema entries before merging" | merge.rs:9-10 | REAL | stage 1 (169-200) + programmatic_partial test |
| "unknown keys are errors" | merge.rs:10 | REAL | PartialProduct deny_unknown_fields (partial.rs:19) + t10 |
| "attributed to the highest layer that set an involved key" | merge.rs:111-112 | FALSE for multi-key rules | blame() consults only the reported key path (F-001); user width=500 vs base min_width=940 blames product.toml |
| "user overrides" as v1 scope / §4.2 layer 4 | merge.rs:1-2, 31-32, doc 07 §4 | REAL as library, UNREACHABLE in product | no shipped caller (F-002) |
| "the merge is a pure function of the ordered layer list" | merge.rs:11 | REAL | merge_is_deterministic test + read of merge_over (no hidden state) |

## 7. Dead / duplicated / unreachable code
- Profile/Admin/Flags/Session Layer variants: declared, precedence-ranked, error-checked — no loader constructs them anywhere (Phase 2+ stubs by design, per header).
- The entire resolve_layers/layer API is unreachable from shipped entry points (F-002).
- No duplicated logic found; merge_over is verbose but complete.

## 8. Dependencies & flags
- serde_json (to_value round-trip), crate-internal: diagnostic/lines/model/partial/schema/semantic. No feature gates. Crate is L1 (doc 01 §2.3); its only shipped dependent is apps/control-server, which does not use this module.

## 9. Open questions for the next agent
- Will the Phase-2+ profile/admin loaders (doc 07 §4.3 locks) reuse LayerSource.file-only provenance, or need (layer,file) identity (F-004)?
- settings-schema crate: does IT plan to call resolve_layers for user settings, or is the merge API permanently user-preferences-only? Check settings-schema audit (W11-A06 file 12).
