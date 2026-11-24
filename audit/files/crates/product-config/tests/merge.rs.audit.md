# AUDIT: crates/product-config/tests/merge.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines / functions | 332 / 15 tests |
| File verdict | REAL (test file; honest about v1 scope but 9 of 16 doc-§4.5 rows don't exist anywhere) |
| Reachable from shipped entry point? | N/A (integration test). The code UNDER TEST (product-config src/merge.rs resolve_layers/layer) is NOT reachable from the release lane: rg shows zero `product_config::` calls in apps/cli and apps/desktop (desktop/src/main.rs:3 mentions "product-config → registry" in a doc comment only); the sole app consumer is apps/control-server (feature-gated off in product.toml, not in the release lane). Merge machinery is test-only today. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg` for T2..T14 rows across crate (no hits, exit 1); `sed -n` quote verification (exit 0); baseline `cargo-test-workspace.txt` product-config section (15 merge tests pass, exit 0) |

## 1. One-paragraph verdict
These are real tests over the real merge engine, driven by the REAL committed product.toml + product.schema.json (tests/common), with negative paths asserted before divergence (no bare unwrap). They exercise layer ordering rejection, deep-table leaf merge, per-key features merge, cross-field blame at merge stage, programmatic-partial validation, and 100-iteration determinism. All 15 pass in the orchestrator baseline. Two honesty defects: T16 is named for `config.reset-setting` but no reset/removal API exists (PartialProduct has exactly one method, `is_empty`) — the test proves only that an absent key falls through, which is T15-adjacent, not reset; and doc 07 §9 row 9 / §10.2 promise a "full precedence matrix T1–T16" in `cargo test --workspace`, while rows T2, T3, T5, T6, T7, T8, T9, T11, T14 exist NOWHERE in the crate (rg across tests/*.rs and src/*.rs: zero hits).

## 2. What actually works (verified)
— verified by `sed -n '<product-config section>' audit/runs/baseline/cargo-test-workspace.txt` -> `merge_is_deterministic_across_runs`, `cross_field_violation_after_merge_is_blamed_on_the_layer`, `layer_list_must_be_ordered`, `programmatic_partial_out_of_range_is_rejected`, t1/t4/t10/t12/t13/t15/t16 all `ok`; product-config exit_code=0 (41+27+1+15+17+3+3 tests).

## 3. Findings
F-001 [S3]
Where: crates/product-config/tests/merge.rs:137-155
Code:
```
/// T16 — reset (removing the key from the user layer) exposes the
/// next-lower value again.
#[test]
fn t16_reset_returns_the_next_lower_value() {
```
and:
```
    // `config.reset-setting` removes the key from the user layer (doc 07
    // §4.4 rule 5): resolve without it.
    let after_reset = match resolve_user(&base, &user_layer("[playback]\nstart_paused = true\n")) {
```
What is wrong: the test never removes anything — it builds a SECOND, different user layer that never contained `window.width`. No removal API exists to call: `PartialProduct` (src/partial.rs) has exactly one method, `is_empty` (verified: `rg -c "pub fn"` → 1). The doc-07 §4.5 T16 row ("`config.reset-setting` after override → next-lower value returns") is therefore only half-covered: the resolve side yes, the reset operation no — because the operation is unimplemented in this crate and unpersisted in the hosts (see parse.rs audit round-trip). Would the test fail if reset semantics were broken? No — there is nothing to break here; the name promises more than the body tests.
Confidence: HIGH — Verification: read + rg over partial.rs.

F-002 [S3]
Where: docs/07-config-driven-system.md:849 + :863 vs crates/product-config/tests/merge.rs:1-3
Code (doc):
```
9  cargo test --workspace --all-features   (unit + precedence matrix T1–T16)
```
Code (test header):
```
//! Precedence merge v1 (doc 07 §4.4 + §4.5 matrix rows in v1 scope:
//! product defaults + user) and the extension-point behaviors later cards
```
What is wrong: doc 07 §9 (CI row 9) says the workspace test run includes "precedence matrix T1–T16"; §10.2 lists "The full precedence matrix T1–T16 (§4.5) as named tests" plus "lock semantics incl. T8/T9". Reality: T2, T3, T5, T6, T7, T8, T9, T11, T14 do not exist as tests anywhere in the crate (rg `t2_|t3_|t5_|…|T14` over tests/ and src/ → zero hits; Profile/Admin/Flags/Session layers have loaders deferred to "Phase 2+" per src/merge.rs:27-36). The test-file header is honest about "v1 scope"; the DOC's CI description overstates what runs. Not a fraud (§10 discipline note at doc 07:870 says nothing counts as verified until tests exist), but the §9 CI row is written as fact of the lane.
Confidence: HIGH — Verification: read + rg.

F-003 [S4]
Where: crates/product-config/tests/merge.rs:201-224
Code:
```
/// §4.4 rule 7 — determinism: the merge is a pure function of the ordered
/// layer list; the resolved snapshot is byte-identical across runs
/// (doc 07 §10.2, 100 iterations).
#[test]
fn merge_is_deterministic_across_runs() {
```
What is wrong: 100 iterations of resolving the SAME in-memory inputs inside one process. This can only diverge on iteration-order nondeterminism (e.g. a HashMap swap) or serialization instability — which it would genuinely catch — but it cannot catch file-parse nondeterminism, key-order dependence on disk layout, or time/locale effects, because nothing is re-read from disk. The doc cites this exact test as the §10.2 determinism acceptance. Weak-but-not-tautological.
Confidence: HIGH — Verification: read (mutation reasoning; not executed by me).

F-004 [S4]
Where: crates/product-config/tests/merge.rs:15-20 (and 22-27)
Code:
```
fn base() -> ProductConfig {
    match load_root_product() {
        Ok(c) => c,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    }
}
```
What is wrong: `unreachable!` on the REAL committed product.toml — if someone commits a product.toml that fails validation, these tests abort with a panic message instead of a clean assertion, and every test in the file fails identically. Common.rs documents this as deliberate ("failure paths assert first and then diverge"), and the panic does carry the diagnostics, so blast radius is developer-experience only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
Test file — per-test honesty table instead (§5). Helpers: `base` (real product.toml), `user_layer` (real schema-validated partial), `resolve_user` (wraps resolve_layers with a User LayerSource) — all real, no mocks of the thing under test.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| t1_only_product_layer_resolves_product_defaults | YES | asserts equality with base + concrete values (1280/"dark") |
| t4_user_overrides_product | YES | override + 3 fall-through assertions |
| t10_unknown_key_in_user_layer_is_an_error_with_hint | YES | pins rule text, hint "did you mean `width`", file, line 2 |
| t12_out_of_range_in_user_layer_is_an_error | YES | pins "must be >= 480", file, line 2 |
| t13_array_policy_replace | YES | full replacement asserted |
| t15_layers_agree_resolve_once | YES | resolved == base |
| t16_reset_returns_the_next_lower_value | NO (for reset) | F-001: tests absence, not removal |
| deep_merge_replaces_only_the_named_leaf | YES | sibling keys asserted unchanged (3 levels) |
| features_table_merges_per_key | YES | per-key merge + w31.3 default pin (streaming-http=true) |
| merge_is_deterministic_across_runs | PARTIAL | F-003: catches order nondeterminism only |
| programmatic_partial_out_of_range_is_rejected | YES | direct PartialProduct construction rejected at resolve |
| cross_field_violation_after_merge_is_blamed_on_the_layer | YES | Stage::Merge + file + hint pinned |
| layer_list_must_be_ordered | YES | User-before-Profile rejected with `<layer-list>` path |
| product_layer_is_not_a_list_entry | YES | Product-as-entry rejected ("resolution base") |
| layer_convenience_is_product_plus_user | YES | width override + height fall-through |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "unit + precedence matrix T1–T16" runs in workspace tests | docs/07-config-driven-system.md:849 | FALSE (7 of 16 rows exist) | rg over crate: t2/t3/t5–t9/t11/t14 absent |
| "The full precedence matrix T1–T16 (§4.5) as named tests; lock semantics incl. T8/T9" | docs/07-config-driven-system.md:863 | PAPER-ONLY (acceptance row, unmet) | same rg; Layer::Admin has no loader (src/merge.rs:29-30 "Phase 2+ loader") |
| "unknown-key rejection with did-you-mean (fixture typos)" | docs/07-config-driven-system.md:863 | REAL | t10 passes (baseline) |
| "determinism … 100 iterations" | docs/07-config-driven-system.md:863 | SHALLOW | F-003 in-process only |

## 7. Dead / duplicated / unreachable code
None in-file. Under-test code (resolve_layers) currently unreachable from release lane (see header table).

## 8. Dependencies & flags
product_config (lib), std only + serde_json via ProductConfig::Serialize in the determinism test. No feature gates.

## 9. Open questions for the next agent
- src/merge.rs, src/partial.rs, src/semantic.rs, src/load.rs, src/schema.rs are NOT my files — F-001's "no removal API" claim rests on `rg "pub fn"` over partial.rs (1 hit); a fuller partial.rs audit belongs to its owner.
- The xtask config-validator enum hole (Wave 1: subtitle_mode=3 passes) is CONFIRMED by read in tools/xtask/src/cmd/config_validator.rs:285-299 — the `enum` check lives inside the `TValue::Str(s)` match arm, so integer values skip it entirely (the crate-level schema.rs:179-202 enum check is type-agnostic and would catch it; the hole is only in the xtask's parallel reimplementation). Owner: whoever audits tools/xtask.
