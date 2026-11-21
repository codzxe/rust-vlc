# AUDIT: crates/product-config/src/merge.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-21 |
| Lines / functions | 487 / 10 (incl. 2 types + enum) |
| File verdict | UNREACHABLE (real, tested library code; zero shipped callers) |
| Reachable from shipped entry point? | NO — only `crates/product-config/tests/{merge,web_budgets}.rs` call this API (rg proof below) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |

## 1. One-paragraph verdict

The layer-precedence resolver is genuine, careful code — three-stage
(validate-each-layer → deep-merge → cross-field re-check with blame), no
unwrap/expect/panic, deterministic, and covered by a 15-test integration
suite that all passed in the baseline no-fail-fast rerun
(`orchrun-test-workspace-nofailfast.log.txt:4700-4718`). But it is a library
seam with **zero production consumers**: nothing in `lumen`, `lumen-tauri`,
desktop, web, or control-server calls `resolve_layers`/`layer`/`LayerSource`/
`load_partial_*` — the shipped CLI's `config.set-setting` writes a
process-local `BTreeMap` and never persists, so doc 07 §4.1's "layer 4:
persisted settings written by `config.set-setting`" is false end-to-end and
user settings do not round-trip. On top of that, the blame logic mis-attributes
a realistic cross-field violation to the read-only `product.toml` when a user
override is the actual cause, contradicting the function's own doc comment.

## 2. What actually works (verified)

- Tests executed in baseline: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`
  — `Running tests/merge.rs` (line 4700), 15/15 ok: T1 product-only, T4 user
  override + fall-through, T10 unknown-key did-you-mean, T12 out-of-range with
  file+line, T13 array replace, T15 agree-resolves-once, T16 reset, deep
  merge, per-key features merge, 100-iteration determinism, programmatic
  partial validation, cross-field blame (min_width case), layer ordering,
  product-not-a-list-entry, `layer()` convenience.
- `web_budgets.rs` layer test also ran (`layers_cannot_override_web_budgets ... ok`, line 4761).
- `merge_over` coverage cross-checked field-by-field against `partial.rs`:
  all 12 sections and all 44 leaf fields are merged — nothing silently
  dropped today (verified by reading both files side by side).
- No `Severity::Warning` exists in schema/semantic/load (rg), so
  `resolve_layers`' "fail if any diagnostic" cannot abort on mere warnings.
- `rg -n 'resolve_layers|LayerSource|PartialProduct|load_partial' apps crates tools`
  → callers: only `crates/product-config/tests/merge.rs:11-38,281-310`,
  `tests/web_budgets.rs:9,61`, and the crate's own `lib.rs:54-55` re-exports.
  control-server (the one shipped consumer of product-config) uses only
  `load_file`/`ProductConfig`/`Schema` (`apps/control-server/src/main.rs:135-154`,
  `config.rs:26,240`). This is the reachability proof — negative.

## 3. Findings

### F-001 [S2]
Where: crates/product-config/src/merge.rs:105-112 (claim), 216-226 (attribution), 279-293 (blame); crates/product-config/src/semantic.rs:209-218 (rule)
Code:
```rust
/// Stage 1 validates every layer's present values against the same schema
/// entries (rule 4); stage 2 merges (tables deep, scalars/arrays replace);
/// stage 3 re-runs the pure cross-field rules over the resolved config so a
/// combination that is legal per-key but illegal as a whole (e.g. a user
/// `min_width` above the product `width`) fails with an actionable
/// diagnostic attributed to the highest layer that set an involved key.
```
```rust
fn blame(provenance: &BTreeMap<String, String>, key_path: &str) -> Option<String> {
    // Exact key first.
    if let Some(file) = provenance.get(key_path) {
        return Some(file.clone());
    }
```
What is wrong: `blame` consults only the diagnostic's own `key_path` (plus
prefixes), never the *other* keys involved in the rule. The geometry rule
always reports `window.min_width` (semantic.rs:212) — never `width`. So when
a user layer sets `window.width = 500` (legal per schema: width min 480,
product.schema.json:114) over the product's `min_width = 940`
(product.toml:70), the merged config violates `min_width < width`;
`blame("window.min_width")` finds nothing (the user set `width`, and `window`
is never a leaf in provenance), returns `None`, and the diagnostic keeps
`file: "product.toml"` (merge.rs:212) — blaming the read-only distribution
file for a violation the user layer caused. The appended hint then asserts
"the file shown is the layer that set the key" (merge.rs:224), which is false
in exactly this case. The doc comment's promise — "attributed to the highest
layer that set an involved key" — is not implemented for any involved key
other than the reported one.
Why it matters: doc 07 §3.5's contract is that a diagnostic names the file
the user can fix; layer 1 is "read-only at runtime" (§4.1), so pointing the
user at `product.toml` (with the hint "raise window.width or lower
window.min_width") sends them to edit a file they must not touch. The
realistic v1 case: user shrinks the window below the 940 UI floor.
What correct looks like: pass all involved key paths from the rule (e.g.
`window.min_width` and `window.width`) to attribution and pick the highest
layer among them; or have the rule report the highest-precedence involved key.
Confidence: HIGH — Verification: read (full control-flow trace: Stage 1
schema-only at 191-196, cross-field only at 217, blame fallback at 219-221);
the passing baseline test covers only the min_width-set-by-user direction
(`cross_field_violation_after_merge_is_blamed_on_the_layer`, log line 4703).

### F-002 [S2]
Where: crates/product-config/src/merge.rs:92-103 (v1 API), docs/07-config-driven-system.md:447-448 (§4.1 layer 4), apps/cli/src/host.rs:181,246,1403
Code:
```rust
/// v1 convenience: product defaults + one user layer (card 1.B1 scope).
pub fn layer(
    base: &ProductConfig,
    user: &PartialProduct,
    schema: &Schema,
) -> Result<ProductConfig, Vec<Diagnostic>> {
```
What is wrong: the entire layering API this file implements — the v1
"product defaults + user" story — has no consumer in any shipped entry
point. `rg` across `apps/`, `crates/`, `tools/` shows the only callers of
`resolve_layers`/`layer`/`LayerSource`/`load_partial_str`/`load_partial_file`
are the crate's own integration tests. Doc 07 §4.1 declares layer 4 as
"User preferences | persisted settings written by `config.set-setting`",
but the shipped CLI's `config.set-setting` inserts into
`settings: BTreeMap<String, SettingValue>` (host.rs:181, initialized empty
at 246, written at 1403) that dies with the process — no
`preferences.toml` reader or writer exists anywhere (rg `preferences` over
Rust sources: only unrelated localization-fallback hits; doc 07 §4.2's
per-platform `preferences.toml` paths are unimplemented), and
`config.reset-setting` is declared in the commands catalog
(crates/commands/src/catalog/command.rs:425-426) but absent from CLI
dispatch (apps/cli/src/dispatch.rs falls through to the typed refusal).
Why it matters: this is the wave's central question — config declared vs
consumed. The six-layer precedence model (MP §5) is declared, documented,
library-implemented and test-proven, but the shipped product never layers
anything: user settings do not round-trip (writes vanish at exit), and the
`restart_required: true` acknowledgment `config.set-setting` returns
(SettingApplied, host.rs:1404-1407) implies persistence that does not exist.
What correct looks like: either wire a loader (read `preferences.toml` at
startup, resolve via `resolve_layers`, persist on `config.set-setting`), or
mark layer 4 as not-yet-shipped in doc 07 §4.1 and stop implying persistence.
Confidence: HIGH — Verification: read + rg (caller search above, negative);
baseline rc-smoke/CLI transcripts contain no config.set-setting persistence
evidence.

### F-003 [S3]
Where: crates/product-config/src/merge.rs:24-36 (Layer enum), docs/07-config-driven-system.md:498-516 (§4.5 matrix), crates/product-config/tests/merge.rs:1-3
Code:
```rust
    /// `profiles/<id>.toml` (Phase 2+ loader).
    Profile,
    /// Managed config + lock list (Phase 2+ loader, §4.3).
    Admin,
```
What is wrong: doc 07 §4.5 states "The matrix is exhaustive over layer
*interactions*; each row is a named test" — 16 rows (T1–T16). The committed
suite names only T1, T4, T10, T12, T13, T15, T16 (7 of 16). Rows T2 (profile
overrides product), T3 (admin overrides profile+product) and T5 (flag
overrides 1–4) are expressible *today* — `resolve_layers` accepts any
`Layer` value and synthetic `PartialProduct`s (the ordering test already
builds a Profile LayerSource at tests/merge.rs:282) — but have no tests.
T6–T9, T11, T14 depend on unwired Phase-2+ semantics (locks, session scope).
Why it matters: the doc's "document it and test it" demand is presented as
met; it is met for less than half the matrix. Inflation indicator for the
§4.5 table.
What correct looks like: either add the T2/T3/T5 synthetic-stack tests the
API already supports, or scope the §4.5 claim to the v1 rows.
Confidence: HIGH — Verification: read of tests/merge.rs + doc; baseline log
lines 4701-4717 list the exact 15 tests (no T2/T3/T5 names).

### F-004 [S4]
Where: crates/product-config/src/merge.rs:297-487
Code:
```rust
fn merge_over(config: &mut ProductConfig, partial: &PartialProduct) {
    if let Some(v) = partial.schema_version {
        config.schema_version = v;
    }
    if let Some(p) = &partial.identity {
```
What is wrong: `merge_over` is 190 lines of hand-rolled per-field `if let`
chains with no exhaustiveness guarantee. Adding a field to `PartialProduct`
compiles cleanly with no merge arm; that field's overrides would pass
Stage-1 validation and then be silently dropped — a future "declared but
never consumed" bug baked into the design. Coverage is complete today
(verified against partial.rs: 12/12 sections, 44/44 leaves), so this is a
hazard, not a current defect.
What correct looks like: a macro or a `serde`-round-trip merge
(`merge::<toml::Value>` + deserialize) that fails to compile when a field is
added without merge support, or a test that asserts every `PartialProduct`
field round-trips through `merge_over` (reflection-style exhaustive test).
Confidence: HIGH — Verification: read; field-by-field diff vs partial.rs.

### F-005 [S4]
Where: crates/product-config/src/merge.rs:268-269, 285-291
Code:
```rust
        leaf => {
            let _ = leaf;
```
What is wrong: two bits of dead weight: (a) `let _ = leaf;` noise binding to
silence the match arm; (b) `blame`'s prefix loop starts at
`len == segments.len()` — the full path already checked exactly two lines
above — so the first iteration is a guaranteed duplicate lookup.
Why it matters: hygiene only; no behavioral effect.
What correct looks like: `serde_json::Value::Null | _ =>` arm or restructure;
iterate `len` from `segments.len() - 1`.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Layer` (enum) | 22-37 | REAL / UNREACHABLE | — | Six layers, Ord derived in declaration order = precedence order (a second source of truth vs `precedence()` — consistent today). Doc comments honestly mark Profile/Admin/Flags/Session "Phase 2+ loader". |
| `Layer::precedence` | 39-51 | REAL | — | 1–6 mapping; used only by the ordering check. Consistent with enum order. |
| `LayerSource` + `new` | 53-72 | REAL | — | Plain data; `file` is a diagnostic label only (no path semantics). |
| `strip_nulls` | 77-90 | REAL | — | Removes null object members from the serde_json form of a `PartialProduct` (serde writes `None` as null). Nulls inside arrays are kept (correct — positional). Correct recursion. |
| `layer` | 92-103 | REAL / UNREACHABLE | S2 (F-002) | v1 convenience; clones the user partial. No production caller. |
| `resolve_layers` | 113-231 | REAL / UNREACHABLE | S2 (F-001, F-002) | Three stages as documented; ordering + product-as-entry sanity (121-167); Stage-1 per-layer schema validation with empty LineIndex (line info intentionally dropped); Stage-3 blame loop (216-226). No panic paths; `continue` at 137-139 is dead-defensive (`layers.get(i-1)` is always `Some` when `i > 0`). |
| `layer_name` | 233-242 | REAL | — | Trivial display names. |
| `record_provenance` | 245-275 | REAL | S4 (F-005a) | Records leaf `path -> file`; later layers overwrite earlier (BTreeMap::insert) — correct "highest wins". Array elements indexed `path.i`. |
| `blame` | 279-293 | REAL | S2 (F-001) | Exact + prefix lookup only; cannot see sibling involved keys; prefix `window` can never hit (objects are not leaves). |
| `merge_over` | 297-487 | REAL | S4 (F-004) | Deep-merge tables per field, scalars/arrays replace, features per-key — matches doc 07 §4.4 rules 1–3. Coverage complete vs partial.rs (verified). |

## 5. Test honesty (crates/product-config/tests/merge.rs; all ran in baseline, 15/15 ok)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| t1_only_product_layer_resolves_product_defaults | YES | Empty layer list must equal base; asserts width=1280, theme=dark from the real product.toml. |
| t4_user_overrides_product | YES | Mutating `merge_over`'s window arm breaks width=1920 assert; fall-through asserts catch over-merge. |
| t10/t12 | YES (load.rs paths) | Unknown-key hint + range with file/line — assert on load_partial_str diagnostics. |
| t13_array_policy_replace | YES | Replacing the vec assign with extend would fail the exact-eq assert. |
| t15_layers_agree_resolve_once | YES | Asserts full-struct equality with base. |
| t16_reset_returns_the_next_lower_value | YES | Simulated reset (resolve without the key) — library-level only; the `config.reset-setting` command that would drive it does not exist in the CLI (F-002). |
| deep_merge_replaces_only_the_named_leaf | YES | Sibling `app_user_model_id` must stay `dev.lumen.Lumen`. |
| features_table_merges_per_key | YES | Wholesale features replacement would drop `core-playback`. |
| merge_is_deterministic_across_runs | YES (weakly) | 100 iterations byte-compare; only catches nondeterminism present in-process. |
| programmatic_partial_out_of_range_is_rejected | YES | Proves Stage-1 validation covers struct-built layers, not just file-loaded ones. |
| cross_field_violation_after_merge_is_blamed_on_the_layer | PARTLY | Covers only the min_width-set-by-user direction; the width-set-by-user misattribution (F-001) is untested. |
| layer_list_must_be_ordered / product_layer_is_not_a_list_entry | YES | Direct asserts on the sanity diagnostics. |
| layer_convenience_is_product_plus_user | YES | `layer()` must equal resolve with one User source. |
| Gap | — | No T2/T3/T5 synthetic-stack tests (F-003); no multi-layer blame test (would expose F-001). |

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "4 \| **User preferences** \| persisted settings written by `config.set-setting` \| the user" | docs/07-config-driven-system.md §4.1 (line 447) | FALSE | host.rs:181,246,1403 in-memory map; no preferences.toml I/O anywhere (rg); merge API has zero shipped callers (F-002). |
| Per-platform `preferences.toml` locations (layer 4) | doc 07 §4.2 table | PAPER-ONLY | No code reads/writes any of those paths (rg `preferences`). |
| "v1 implements product defaults + user: … then `merge::resolve_layers` (or the `merge::layer` convenience)" | crates/product-config/src/lib.rs:23-25 | REAL-UNPROVEN | Library code real + tests pass (baseline 4700-4718), but no shipped binary invokes it (F-002). |
| §4.4 rule 4: every layer validated against the same schema before merging; unknown keys errors | doc 07 §4.4 | REAL | Stage 1 (merge.rs:169-200); t10/t12/programmatic tests ok in baseline. |
| §4.4 rules 1–3: tables deep, scalars replace, arrays replace (v1) | doc 07 §4.4 | REAL | deep_merge/features/t13 tests ok. |
| §4.4 rule 7: determinism, pure function of ordered list | doc 07 §4.4 | REAL | 100-iteration test ok (baseline 4717). |
| §4.5: "The matrix is exhaustive over layer interactions; each row is a named test" | doc 07 §4.5 | FALSE (inflation) | 7 of 16 rows have named tests; T2/T3/T5 are testable today but absent (F-003). |
| "attributed to the highest layer that set an involved key" | merge.rs:105-112 (own doc) | FALSE | blame() sees only the reported key path; width-set-by-user case attributes to product.toml (F-001). |
| `config.reset-setting` removes the key from the user layer (§4.4 rule 5) | doc 07 §4.4 + commands catalog | UNREACHABLE | Command declared in catalog (commands/src/catalog/command.rs:425) but CLI dispatch has no handler (typed refusal); t16 tests the semantics in-library only. |

## 7. Dead / duplicated / unreachable code

The whole module is dead-in-shipped (F-002): `resolve_layers`, `layer`,
`LayerSource`, `Layer`, and (via load.rs) `load_partial_str`/`load_partial_file`
have zero production callers. `Layer::Profile/Admin/Flags/Session` variants
are additionally unconstructed outside the ordering test. No intra-file
duplication; the ordering check's `None => continue` (137-139) is unreachable
defensive code.

## 8. Dependencies & flags

Crate-internal only: `diagnostic`, `lines`, `model`, `partial`, `schema`,
`semantic` + `serde_json` for the validation form. No feature flags, no
unsafe, no unwrap/expect/panic (workspace lints; verified by read).
`serde_json` round-trip (`to_value` → validate) is the only allocation-heavy
step; runs once per layer at startup — not a hot path.

## 9. Open questions for the next agent

- Whoever owns apps/cli/host.rs: the `settings` BTreeMap (line 181) is the
  de-facto "session layer" — should it route through this crate's
  `resolve_layers` when persistence lands, or stay separate? The
  `restart_required` field returned by `SettingApplied` currently promises
  semantics (persist + restart) that nothing delivers.
- A02 (schema.rs/partial.rs owners): confirm `PartialProduct` growth gets a
  matching `merge_over` arm (F-004 hazard) — a cross-field exhaustiveness
  test would lock this.
- Is `Layer::precedence()` vs enum-declaration-order dual bookkeeping
  intended? A reordering of the enum silently changes Ord while
  `precedence()` keeps 1–6 — they can diverge.
