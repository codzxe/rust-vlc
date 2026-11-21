# AUDIT: crates/product-config/src/partial.rs
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-21 |
| Lines / functions | 165 / 14 (13 structs + `PartialProduct::is_empty`; no in-file tests — suite lives in tests/merge.rs) |
| File verdict | UNREACHABLE (real, internally consistent, zero consumers) |
| Reachable from shipped entry point? | NO. `PartialProduct` ← `load_partial_str`/`load_partial_file` (load.rs:101,163) and `resolve_layers`/`layer` (merge.rs:93,113) — **no caller exists outside crates/product-config's own tests**: `rg 'load_partial\|resolve_layers\|PartialProduct\|LayerSource'` excluding the crate, docs and audit/ → the ONLY hit is a `$comment` in product.schema.json:209. apps/control-server uses `load_file` (full model) only; apps/cli and apps/desktop-tauri do not depend on product-config. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg load_partial|resolve_layers|PartialProduct|LayerSource` (consumer census — none); full read of model.rs (type agreement) and merge.rs (merge semantics); `rg 'is_empty\(\)'` in crates/product-config (no callers of the method); read tests/merge.rs test list; `rg validate_semantics|FeatureRegistry` in apps/ + tools/ (semantic stage has no external consumer either). |

## 1. One-paragraph verdict
The struct set is mechanically excellent: every field of the full `ProductConfig` is mirrored as `Option<T>` with identical types (verified field-by-field against model.rs), `deny_unknown_fields` everywhere, enums shared with the full model, and the one deliberate divergence — no `web` field — is a documented, tested design carve-out. But the file is the shape of a feature that does not exist at runtime: the entire doc 07 §4 layered-precedence system (this file + merge.rs + `load_partial_*`) has zero consumers outside its own test suite, no shipped binary loads a user-preferences layer, and the crate's test suite itself never executed in any baseline run. This is the persistence machinery whose absence makes the settings seam's `SettingApplied` a dead store (see the runtime.rs audit, F-001) — the scaffold is built, tested by nobody, and wired to nothing.

## 2. What actually works (verified)
— Nothing executed for this file in baseline. The product-config crate's suite (tests/merge.rs, which exercises this file via 18 tests: T1/T4/T10/T12/T13/T15/T16 rows plus deep-merge, per-key features merge, determinism, programmatic-partial range rejection, cross-field blame, layer ordering) never ran in either baseline workspace test pass (run #3 aborted at `desktop`, run #4 at `first-vertical`; `rg 'product_config-'` in both logs → only a `Compiling` line). The indirect baseline execution (control_server unittests) covers `load_file`/full model only, not this file.
— Statically verified: field-for-field type agreement with model.rs (schema_version u32, version_code u64, window u32s, limits u64s, shared `SubtitleMode`/`HwDecodePolicy`/`CrashReports`/`MigrationEntry` enums); every struct carries `deny_unknown_fields`; `Serialize` is consumed in-crate (merge.rs:173 serializes the partial for layer validation, with nulls stripped by merge.rs:77-90).

## 3. Findings

### F-001 [S3] The whole layer system is a consumer-less scaffold — doc 07 §4's user-preferences persistence exists only in tests
Where: crates/product-config/src/partial.rs:14-32 (the shape) + the consumer census
Code:
```rust
/// v1 (card 1.B1) loads this from user preferences; profile/admin/flag/
/// session loaders are Phase-2+ cards that reuse this shape.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialProduct {
```
What is wrong: the doc comment's "v1 … loads this from user preferences" names a loader that does not exist. No code anywhere — apps, tools, tests outside the crate — constructs a `PartialProduct` from a user file or calls `load_partial_file`/`resolve_layers`/`merge::layer`. The comment cites "card 1.B1" as if the loading were done; card 1.B1 delivered the shape and its tests, nothing more. Meanwhile doc 07 §4.2 assigns per-platform user-preferences file locations and §4.5 a 16-row precedence matrix, and the settings seam (settings-schema runtime.rs + CLI host) stores validated writes into a `BTreeMap` that dies with the process instead of into this layer system.
Why it matters: this is the wave's hostile question at its purest — config machinery declared (schema'd, typed, merge-tested) and consumed by no shipped entry point. Every `config.set-setting` "restart_required" promise is vacuous partly because the layer that would carry the value across restarts (this file) is unreachable dead weight in every shipped binary.
What correct looks like: either the CLI/desktop host loads and persists a layer-4 document through `load_partial_file` + `resolve_layers` at startup/shutdown, or the docs and this header state plainly that layered persistence is unimplemented (Phase 2+) and `SettingApplied` stops implying otherwise.
Confidence: HIGH — Verification: rg consumer census (command list above); read of load.rs/merge.rs/lib.rs; no run (rule 8).

### F-002 [S4] `PartialProduct::is_empty` is dead code
Where: crates/product-config/src/partial.rs:34-39
Code:
```rust
impl PartialProduct {
    /// True when no key is set (an empty layer is a no-op, §4.5 T15).
    pub fn is_empty(&self) -> bool {
        *self == PartialProduct::default()
    }
}
```
What is wrong: zero call sites — `rg 'is_empty\(\)'` over crates/product-config matches only `str`/`Vec`/slice `is_empty` uses (suggest.rs, lines.rs, load.rs, merge.rs), never this method. It cites §4.5 T15 ("two layers agree"), but the T15 test (tests/merge.rs:127) does not call it either.
Why it matters: dead public API in a contract crate; the §4.5 citation makes it look load-bearing.
What correct looks like: delete it, or use it in `resolve_layers` to skip no-op layers (a plausible original intent).
Confidence: HIGH — Verification: rg census.

### F-003 [S4] The designed `[web]`-in-a-layer refusal surfaces as "this is a product-config crate bug — please report it"; the header also mis-attributes the did-you-mean mechanism
Where: crates/product-config/src/partial.rs:6-8 + crates/product-config/src/load.rs:130
Code (partial.rs:6-8):
```rust
//! layer. Unknown keys are rejected at parse time (`deny_unknown_fields`) so
//! typos surface as startup errors with did-you-mean hints (§4.4 rule 4,
//! §4.5 T10) — never as silent no-ops.
```
Code (load.rs:130):
```rust
            hint: "the schema pass and the partial model disagree — this is a product-config crate bug; please report it with the offending file.".to_string(),
```
What is wrong: two mechanisms are conflated. Typos inside known tables are caught by the *schema structural pass* (schema.rs `additionalProperties: false` + `did_you_mean`, exercised by test T10), never reaching serde; `deny_unknown_fields` is in practice the `[web]`-refusal backstop (the only key the schema accepts but the partial model lacks). When that backstop fires, the user is told the crate has a bug — but the divergence is deliberate (model.rs:29-31, product.schema.json:209, proven by tests/web_budgets.rs `layers_cannot_override_web_budgets`). The header's "with did-you-mean hints" claim is wrong for exactly the path `deny_unknown_fields` guards.
Why it matters: a designed refusal that tells users to file bug reports is false-bug-report bait, and the header teaches the wrong mechanism to the next maintainer.
What correct looks like: the load.rs typed-stage hint distinguishes the web carve-out ("`[web]` is a product decision, not a user preference — not layer-overridable (doc 07 §4.4)") from genuine model/schema divergence; the partial.rs header credits the schema pass for typo hints.
Confidence: HIGH — Verification: read of load.rs:101-133, schema.rs unknown-key path (schema.rs:351-375), web_budgets.rs:58-70.

### F-004 [S4] The overridable-surface carve-out is inconsistent: user layers may rewrite `migrations`, `schema_version`, `identity`, `brand`, and flip `features.*` — only `[web]` is protected
Where: crates/product-config/src/partial.rs:21,22,24,26,31 + crates/product-config/src/merge.rs:482-486
Code (merge.rs:482-485):
```rust
    if let Some(p) = &partial.migrations
        && let Some(v) = &p.entries
    {
        config.migrations.entries = v.clone();
```
What is wrong: the `[web]` carve-out's recorded rationale is "budgets are a product decision, not a user preference" (product.schema.json:209) — but `migrations.entries` (the schema-version migration ledger), `schema_version` itself, the whole `identity`/`brand` rebrand surface, and every `features.*` flag are all layer-overridable through this shape. `resolve_layers` stage 3 re-runs only `cross_field_checks` (merge.rs:217), not the F1/F2 feature-registry intersection (`validate_semantics` + `FeatureRegistry`, semantic.rs:141 — which itself has zero consumers outside the crate's tests), so a layer can flip `features.hw-decode = true` or forge a consistent `schema_version` + `migrations.entries` pair and the merge returns Ok.
Why it matters: latent (no runtime consumer — F-001), but the day a loader is wired, the protection envelope is drawn around the wrong keys: product-owned surfaces (identity, brand, migrations, feature gates) are exactly the ones a *user* layer must not rewrite.
What correct looks like: the not-overridable set (web, migrations, schema_version, identity, brand, features) enforced in one place — e.g. `PartialProduct` omitting them as it omits `web`, or `resolve_layers` rejecting layer documents that set them.
Confidence: HIGH — Verification: read of merge.rs merge_over (297-487) and semantic.rs entry points; no run.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PartialProduct` struct | 18-32 | REAL (shape) | S3 | mirrors ProductConfig minus `web` (deliberate); `deny_unknown_fields`+`default` correct serde idiom; zero consumers (F-001) |
| `PartialProduct::is_empty` | 34-39 | DEAD | S4 | no callers (F-002); equality-vs-default is correct for the derived PartialEq |
| `PartialIdentity` | 41-51 | REAL | — | mirrors Identity (model.rs:44-52) incl. version_code u64 |
| `PartialIdentityIds` | 53-61 | REAL | — | mirrors IdentityIds |
| `PartialWindowsIds` | 65-68 | REAL | — | mirrors WindowsIds |
| `PartialMacosIds` | 70-74 | REAL | — | mirrors MacosIds |
| `PartialLinuxIds` | 76-80 | REAL | — | mirrors LinuxIds |
| `PartialWebIds` | 82-86 | REAL | — | mirrors WebIds (identity.ids.web — unrelated to the `[web]` section carve-out) |
| `PartialWindow` | 88-99 | REAL | — | u32 geometry fields match Window; booleans match |
| `PartialBrand` | 101-109 | REAL | S4 | layer-overridable brand paths (F-004) |
| `PartialDefaults` | 111-117 | REAL | — | mirrors Defaults |
| `PartialPlayback` | 119-128 | REAL | — | shares SubtitleMode/HwDecodePolicy enums with the full model; `default_rate: Option<f64>` — serde accepts TOML integers into f64 (serde f64 visitor takes visit_i64), no coercion trap |
| `PartialLimits` | 130-141 | REAL | — | u64s match Limits; range-checked at load via schema (T12 proves) |
| `PartialTelemetry` | 143-150 | REAL | — | shares CrashReports enum |
| `PartialUrls` | 152-159 | REAL | — | Option-in-Option matches Urls' optional strings; layers can set but never clear (no TOML null — doc 07 §4.4 rule 5 handles via reset command) |
| `PartialMigrations` | 161-165 | REAL | S4 | layer-replaceable migration ledger (F-004) |

## 5. Test honesty
| Test (crates/product-config/tests/merge.rs) | Would it fail if the code were broken? | Reason |
|---|---|---|
| t1/t4 (defaults, user override) | YES | resolved-value asserts on synthetic stacks |
| t10_unknown_key_in_user_layer_is_an_error_with_hint | YES | proves the schema-pass typo hint path (partial.rs:7's real mechanism) |
| t12_out_of_range_in_user_layer_is_an_error | YES | proves layer values are range-validated before merge (stage 1) |
| t13_array_policy_replace | YES | list replaces whole |
| t16_reset_returns_the_next_lower_value | YES | removal semantics |
| deep_merge_replaces_only_the_named_leaf | YES | sparse-merge precision |
| features_table_merges_per_key | YES | per-key insert, not table replace |
| programmatic_partial_out_of_range_is_rejected | YES | proves programmatic PartialProduct (no TOML) still validated — closes the bypass |
| cross_field_violation_after_merge_is_blamed_on_the_layer | YES | stage 3 + provenance blame |
| layer_list_must_be_ordered / product_layer_is_not_a_list_entry | YES | precedence sanity |
| layer_convenience_is_product_plus_user | YES | merge::layer wrapper |
| **ALL of the above** | **UNEXECUTED in baseline** | product-config suite absent from both workspace test logs (see §2); REQUEST-ORCH-RUN filed in W11-A02 log |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Layers 2–6 … expressed as sparse overrides over the product defaults" | partial.rs:3-5 | REAL (as a data shape) | struct set + merge.rs semantics + tests/merge.rs |
| "v1 (card 1.B1) loads this from user preferences" | partial.rs:16-17 | **PAPER-ONLY** | no loader exists anywhere (F-001 census); only tests construct it |
| "Unknown keys are rejected at parse time (deny_unknown_fields) so typos surface as startup errors with did-you-mean hints" | partial.rs:6-8 | SHALLOW | outcome true, mechanism mis-attributed: did-you-mean comes from the schema pass; deny_unknown_fields fires only for `[web]` and then reports "crate bug" (F-003) |
| "NOT layer-overridable in v1: PartialProduct has no web field, so a layer document carrying [web] fails loudly as an unknown key" | product.schema.json:209 | REAL | tests/web_budgets.rs:58-70 proves the refusal (suite unexecuted in baseline, read-verified) |
| §4.4 rules 1–7 (deep merge, validate-before-merge, unknown keys errors, determinism) | docs/07-config-driven-system.md:495-506 | REAL-UNPROVEN | implemented in merge.rs (read) + tests exist; zero baseline execution (§2) |
| §4.5 T1–T16 precedence matrix | docs/07-config-driven-system.md:507+ | PARTIAL — only rows with in-repo tests exist (T1,T4,T10,T12,T13,T15,T16 named in tests/merge.rs); T2/T3/T5/T6/T7/T8/T9/T11/T14 (profile/admin/flags/session/lock rows) have no implementation and no test — consistent with "Phase 2+" but the doc table does not say which rows are unimplemented | tests/merge.rs test list |

## 7. Dead / duplicated / unreachable code
- Dead: `PartialProduct::is_empty` (F-002).
- Unreachable: the entire file from every shipped entry point (F-001); also its collaborators merge.rs `layer`/`resolve_layers` (no external callers) and load.rs `load_partial_*`.
- Not duplicated: the shape genuinely mirrors model.rs once (no copy-paste divergence found — field-by-field check passed).

## 8. Dependencies & flags
- No direct imports beyond serde/std (BTreeMap) — pure data module, consistent with the crate's L1 no-internal-deps claim.
- `#[serde(deny_unknown_fields, default)]` on all 13 structs — uniform, correct.
- No feature flags, no unsafe, no panics. Nothing cfg-gated.

## 9. Open questions for the next agent
- merge.rs is not in my assignment: its stage-1 validation serializes the partial and validates against a `LineIndex::new("")` — layer diagnostics from `resolve_layers` carry no line numbers by construction (merge.rs:170,210). Verify whether that violates doc 07 §3.5's "file + line" contract for layer-attributed errors.
- If the orchestrator runs `cargo test -p product-config` (my REQUEST-ORCH-RUN), the T-matrix and web-refusal claims in §6 move from REAL-UNPROVEN to REAL/failed — that run is the single highest-value missing evidence for this whole crate.
- semantic.rs `validate_semantics` (F1/F2, brand-asset magic bytes, injected enums) also has zero consumers outside the crate's tests — whoever audits semantic.rs should census the same way.
