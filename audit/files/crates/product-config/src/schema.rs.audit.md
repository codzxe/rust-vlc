# AUDIT: crates/product-config/src/schema.rs
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-21 |
| Lines / functions | 639 / 25 (incl. 7 tests, 1 inner fn, 3 free fns) |
| File verdict | REAL (code) — UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO. `Schema::validate` ← `load_str`/`load_partial_str` (load.rs:78,115) ← `load_file`/`load_partial_file` (load.rs:153,163) ← **apps/control-server** (main.rs:145,154; src/config.rs:226,240) + tests only. `apps/cli/Cargo.toml` `[dependencies]` has no product-config; `apps/desktop-tauri` (the other shipped surface) re-parses product.toml with its own `parse_window_table` (apps/desktop-tauri/src/app.rs:243). Packaging gate ships lumen-cli/desktop/tauri only (scripts/packaging/gate.sh:85). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 4 |
| Commands run | `rg product_config::` (consumers: control-server + tests only); `sed -n` on schema.rs quotes; full read of product.schema.json (all 4 `$ref`s resolve, only known `type`s, only "MiB" units); read tools/xtask/src/cmd/config_validator.rs:1-130 (separate std-only validator); read patterns.rs (all 10 pattern strings registered); baseline logs grepped (see §2/§5). |

## 1. One-paragraph verdict
This is a genuine, carefully written hand-rolled JSON-Schema-subset engine: every supported keyword listed in the module doc is really implemented in `check_node`, pattern drift fails loudly at load (`check_pattern_coverage`), and the committed product.toml/schema.json pair demonstrably runs through it in the baseline (via control-server unit tests). But it is validation theater from the shipped product's point of view: neither shipped entry point links it, the CI gate that "validates product.toml" (`cargo xtask config-validator`) is a third, weaker std-only reimplementation that skips `pattern` entirely, and the shipped GUI parses product.toml itself with a hand-rolled subset parser. Inside the engine, a dangling or chained `$ref` silently disables every constraint on that node, and the MiB unit expansion double-converts fields whose value is already in MiB (`web.size_budgets.phase2_vertical_max_mib` → "must be >= 1 (0.0 MiB)").

## 2. What actually works (verified)
— `Schema::from_str` + `Schema::validate` executed against the committed product.toml + product.schema.json: baseline `cargo test --workspace` ran `control_server` lib unittests (audit/runs/baseline/test-workspace.log.txt:1495, "test result: ok. 11 passed"), including `config::tests::repo_product_today_refuses_the_feature_gate ... ok` and `config::tests::engine_info_carries_product_identity_and_catalog_checksum ... ok`, both of which call `product_config::load_file` on the repo files (apps/control-server/src/config.rs:234-244) — i.e. schema parse, pattern-coverage check, and full structural pass on the real files, exit ok.
— Pattern-coverage claim verified statically: all 10 `pattern` strings in product.schema.json are registered in patterns.rs:20-40 (`matcher_for`), cross-pinned by patterns.rs test `every_matcher_is_registered_under_its_schema_string` (patterns.rs:300-323).

## 3. Findings

### F-001 [S3] Unit expansion divides MiB-valued bounds by MiB again — nonsense diagnostics for `web.size_budgets.*`
Where: crates/product-config/src/schema.rs:407-409 (with product.schema.json:216-227)
Code:
```rust
                Some(_) => format!("{v} ({:.1} MiB)", v / MIB as f64),
```
What is wrong: `x-lumen-unit: "MiB"` is attached both to byte-valued fields (`limits.probe_max_bytes`, minimum 16777216) and to MiB-valued fields (`web.size_budgets.phase2_vertical_max_mib`, minimum 1, maximum 64; `full_p0_max_mib`). The renderer unconditionally divides by `MIB` (1_048_576) as if every annotated value were bytes. For a `phase2_vertical_max_mib = 65` violation the math traces to: rule "must be >= 1 (0.0 MiB) and <= 64 (0.0 MiB)", found "65 (0.0 MiB)" — 1/1048576 and 64/1048576 both format as "0.0".
Why it matters: the crate's entire value proposition is actionable diagnostics (lib.rs:31-33 "expected vs found + a fix hint"); here the rendered constraint is numerically wrong and actively misleading, for the two fields whose whole job is to be human-reviewed size budgets. The golden test covers only the byte-valued case (tests/golden_s35.rs:46 asserts "16777216 (16 MiB)" — correct), so the defect is untested and invisible.
What correct looks like: either the schema annotation distinguishes "value is bytes, display in MiB" from "value already MiB", or the renderer prints MiB-valued fields bare ("must be >= 1 and <= 64"); add a fixture/golden for an out-of-range `phase2_vertical_max_mib`.
Confidence: HIGH — Verification: read + arithmetic traced by hand; no run (rule 8). Reproducible from the quoted code.

### F-002 [S3] Dangling or chained `$ref` silently disables all constraints on that node — contradicts the module's own fail-loudly design
Where: crates/product-config/src/schema.rs:107-114
Code:
```rust
            Some(reference) if reference.starts_with("#/") => {
                let pointer = &reference[1..];
                match self.root.pointer(pointer) {
                    Some(target) => target,
                    None => node,
                }
```
What is wrong: if a `$ref` target does not exist (typo, e.g. `#/$defs/msid`), `resolve` returns the `{"$ref": ...}` node itself; `check_node` then finds no `type`/`properties`/`required`/`pattern` on it and validates nothing — silently. Chained refs (a `$ref` whose target is itself a `$ref`) are equally un-followed: `resolve` runs once per node. The module explicitly brags the opposite philosophy for patterns (schema.rs:53-54 "schema drift fails loudly instead of validating nothing"), and `SchemaError` even has the machinery to fail loudly; `$ref` integrity just was not given the same treatment.
Why it matters: a one-character schema edit converts any node (e.g. all four `brand.*` paths, all `urls.*`) into a no-op validator with zero diagnostics. The committed schema is currently clean (verified: exactly 4 refs — msgid, bcp47, https_url, repo_path — all resolving into existing `$defs`), so this is latent, but the failure mode is exactly the silent-validation-bypass class the design says it refuses.
What correct looks like: `Schema::from_json` walks every `$ref` and rejects dangling/non-`#/`-prefixed/chained refs with a `SchemaError::UnresolvedRef` variant (mirroring `UnmatchedPattern`); a load-time test pins it.
Confidence: HIGH — Verification: read; committed schema refs enumerated by hand.

### F-003 [S3] Three parallel implementations of the product.toml contract; this (strongest) one runs in zero shipped surfaces and zero CI gates
Where: crates/product-config/src/schema.rs:120-130 (the engine) vs tools/xtask/src/cmd/config_validator.rs:16-18 vs apps/desktop-tauri/src/app.rs:243-245
Code:
```rust
//! * `pattern` constraints are NOT enforced — std has no regex. The
//!   `product-config` crate's committed CI suite owns them (msgid/bcp47/
```
(apps/desktop-tauri/src/app.rs:243:)
```rust
    let config = parse_window_table(include_str!("../../../product.toml")).map_err(|error| {
```
What is wrong: the same product.toml contract is implemented (a) here (full: patterns, ranges, enums, typo hints), (b) again in xtask's `config-validator` (1,008 lines, std-only, no `pattern` enforcement — its own header admits it, quoted above), and (c) again in the shipped tauri shell (`parse_window_table` over `include_str!`). The gate that CI actually runs on product.toml is (b); the shipped binary uses (c); this file — the only implementation that enforces everything the schema declares — is linked only into `lumen-control-server` (not shipped, not in the packaging payload) and test binaries.
Why it matters: schema drift between the three is undetectable at the point of use; a product.toml that violates a pattern passes the xtask gate and never reaches this engine in any shipped artifact. This is the declared-but-unconsumed pattern the wave prompt asks about, at the validator level.
What correct looks like: one engine — either xtask invokes the crate (it is in-workspace) or the crate's suite is wired into the same gate as `config-validator`; the tauri shell's `[window]` parse reuses the crate via the workspace rather than `include_str!` + a private parser.
Confidence: HIGH — Verification: read (all three sites) + `rg product_config::` consumer list + packaging gate payload list (scripts/packaging/gate.sh:71-85).

### F-004 [S4] `type_matches` unknown-type fallback silently passes
Where: crates/product-config/src/schema.rs:497-498
Code:
```rust
        "number" => value.is_number(),
        _ => true,
```
What is wrong: a typo'd or future `type` keyword (`"interger"`, `"null"`) makes every value pass the type check; unlike patterns, the type vocabulary is not validated at `Schema::from_json` time. Latent (committed schema uses only the 6 known types — verified by full read).
Why it matters: same silent-bypass class as F-002, one notch cheaper to hit.
What correct looks like: `_ => false`, or a load-time vocabulary check like `check_pattern_coverage` for `type`.
Confidence: HIGH — Verification: read.

### F-005 [S4] `Schema::root()` is dead public API
Where: crates/product-config/src/schema.rs:69-72
Code:
```rust
    /// The parsed schema document (for tools and tests).
    pub fn root(&self) -> &Value {
        &self.root
    }
```
What is wrong: zero callers repo-wide (`rg '\.root\(\)'` over crates/, apps/, tools/, tests/ → no hits). The doc comment names "tools" that do not exist.
Why it matters: dead surface in a contract crate; the "for tools" rationale is fiction.
What correct looks like: delete it, or point a real consumer (e.g. the xtask validator) at it.
Confidence: HIGH — Verification: rg over the whole repo.

### F-006 [S4] Unit filter hard-codes the single legal value "MiB"
Where: crates/product-config/src/schema.rs:227-230
Code:
```rust
            let unit = node
                .get("x-lumen-unit")
                .and_then(Value::as_str)
                .filter(|u| *u == "MiB");
```
What is wrong: any other `x-lumen-unit` value silently loses unit expansion (falls to the `None` arm). Latent — every committed annotation is "MiB" (verified: 3 sites in product.schema.json).
Why it matters: the extension point pretends to be open; a schema author adding `"KiB"` gets no expansion and no error — the same silent-degradation class the module claims to reject.
What correct looks like: unknown unit values rejected at schema load (like unmatched patterns), or a match over a unit enum.
Confidence: HIGH — Verification: read + schema enumerated.

### F-007 [S4] Test name promises line reporting; the test never asserts a line
Where: crates/product-config/src/schema.rs:569-574
Code:
```rust
        let diags = validate(&Value::Object(serde_json::Map::new()));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].key_path, "a");
        assert!(diags[0].rule.contains("required"));
```
What is wrong: `missing_required_is_reported_with_ancestor_line` asserts len, key_path, rule — never `diags[0].line`. If `Diagnostic::line` regressed to `None`/wrong line, this test still passes.
Why it matters: test name asserts more than the body proves (checklist: names promising more than the body).
What correct looks like: `assert!(diags[0].line.is_some())` with the expected line from the fixture source.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SchemaError` (enum) | 26-31 | REAL | — | 2 variants; `UnmatchedPattern` used by coverage check; no variant for $ref integrity (F-002) |
| `RequiredMode` (enum) | 36-40 | REAL | — | Full/Partial; Partial skips `required` only (verified in check_node:329) |
| `Schema` (struct) | 43-46 | REAL | — | holds parsed `Value` root |
| `FromStr::from_str` | 50-58 | REAL | — | parse → from_json; executed in baseline via control-server tests |
| `Schema::from_json` | 63-67 | REAL | — | coverage check then Ok |
| `Schema::root` | 69-72 | DEAD | S4 | zero callers (F-005) |
| `Schema::check_pattern_coverage` + `walk` | 74-93 | REAL | — | recursive walk, object+array; returns first unmatched pattern |
| `Schema::empty_for_tests` | 97-102 | REAL | — | `#[cfg(test)]`; bypasses coverage check by design (test scaffolding) |
| `Schema::resolve` | 105-116 | SHALLOW | S3 | single-level `#/`-ref resolution; dangling/chained → silent no-op (F-002); non-`#/` refs (e.g. remote) silently un-resolved too |
| `Schema::validate` | 120-130 | REAL | — | entry; returns all diagnostics; executed on committed files in baseline |
| `Schema::check_node` | 133-384 | REAL | S3/S4 | engine: type, const, enum, not.enum, ranges, lengths, pattern, items, required(Partial-aware), additionalProperties false/schema; wrong-type short-circuits (documented); unknown `type` passes (F-004); unit filter (F-006) |
| `Schema::range_diagnostic` | 387-445 | REAL | S3 | bound/value formatting; MiB double-division for MiB-valued fields (F-001) |
| `Schema::diagnostic` | 448-477 | REAL | — | severity Error / stage Structural; line via LineIndex; `<root>` for empty path |
| `MIB` const | 480 | REAL | — | 1024*1024 |
| `child_path` | 482-488 | REAL | — | dot-joins; array items indexed `a.0` |
| `type_matches` | 490-500 | REAL | S4 | 6 known types; `_ => true` (F-004); `integer` = i64/u64 only — TOML floats like `5.0` correctly rejected (schema never uses integer-or-float union) |
| `describe_value` | 502-520 | REAL | — | strings quoted, bools/numbers bare; recursive |
| `generic_hint` | 522-530 | REAL | — | x-lumen-hint → description → static fallback |
| test `valid_value_produces_no_diagnostics` | 561-566 | REAL | — | passes on in-range fixture |
| test `missing_required_is_reported_with_ancestor_line` | 568-574 | SHALLOW | S4 | no line assertion (F-007) |
| test `below_minimum_reports_range_with_expected_and_found` | 576-585 | REAL | — | asserts rule/found/expected on byte-less fixture (no unit) |
| test `unknown_key_is_reported_even_in_partial_mode` | 587-603 | REAL | — | did-you-mean hint asserted |
| test `partial_mode_skips_required_but_keeps_value_rules` | 605-617 | REAL | — | Partial keeps range rules |
| test `unmatched_pattern_fails_schema_load_loudly` | 619-629 | REAL | — | pins the load-time failure |
| test `wrong_type_short_circuits_value_checks` | 631-638 | REAL | — | one diagnostic per defect |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 7 in-file tests | YES for the mechanic each pins | Mutation reasoning: break `type_matches` → `wrong_type_short_circuits_value_checks` fails; break coverage walk → `unmatched_pattern_fails_schema_load_loudly` fails; break Partial handling → two Partial tests fail; break range compare → `below_minimum...` fails. |
| BUT: in-file tests use a toy schema, never product.schema.json | — | Real-schema coverage lives in tests/ (fixtures_matrix, golden_s35, semantic) — **which never executed in baseline**: both workspace test runs aborted before the crate (run #3 stopped at `desktop` zero_alloc failure — log ends there; run #4 stopped at `first-vertical` evidence failure; `rg 'product_config-'` in both logs → only the `Compiling` line in #3). Execution proof for this file in baseline is only the indirect control-server path (§2). |
| tests/golden_s35.rs (golden prose) | YES | asserts exact rendered lines incl. "16777216 (16 MiB)" — would catch renderer regressions for probe_max_bytes, but not the `*_mib` fields (F-001 untested) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Data-driven constraints (type, required, additionalProperties, enum, const, minimum, maximum, minLength, maxLength, minItems, maxItems, pattern, not.enum) are enforced by crates/product-config's structural pass" | product.schema.json:5 ($comment) | REAL | each keyword located in check_node:145-384 (read) |
| "schema drift fails loudly instead of validating nothing" | schema.rs:53-54 | FALSE (for $ref/unit/type drift) | F-002/F-004/F-006: only `pattern` drift fails loudly; $ref integrity, type vocabulary, and unit vocabulary degrade silently |
| "pattern constraints not enforced by this v0 … the product-config crate's committed CI suite owns them" | tools/xtask/src/cmd/config_validator.rs:16-18 | REAL-UNPROVEN | the crate's suite exists (read) but never ran in any baseline log — the "CI suite owns them" ownership is currently unexecuted in the recorded evidence |
| Empty `enum: []` defers to semantic pass | schema.rs:10-12 + schema `defaults.*` | REAL | check_node:179-181 skips empty enums; semantic pass owns injection (semantic.rs per fixture tests injected-enum-*.toml) |

## 7. Dead / duplicated / unreachable code
- Dead: `Schema::root()` (F-005).
- Duplicated: the whole engine vs xtask config-validator (std-only, no patterns) vs tauri `parse_window_table` (F-003) — three implementations of one contract.
- Unreachable from shipped entry points: the entire module (see header table). Reachable only from `lumen-control-server` (internal tool, not in packaging payload) and test binaries.

## 8. Dependencies & flags
- Crate deps: serde/serde_json/toml/thiserror only (Cargo.toml read) — matches its "no internal deps, L1" claim.
- No feature flags gate this file; nothing cfg'd out.
- `#[allow(clippy::too_many_arguments)]` ×3 (132, 386, 447) — structural for diagnostic builders; benign but noted per checklist.

## 9. Open questions for the next agent
- xtask `config_validator.rs` (1008 lines): does its subset walk actually agree with this engine on `additionalProperties`/`required` semantics, or does it accept/reject files this engine would not? (File belongs to another agent.)
- `LineIndex::line_of(path)` (lines.rs) maps dotted key paths to lines — unverified here; if it fails for array-item paths (`a.0`), diagnostics lose lines. Owner: whoever audits lines.rs.
- The product-config crate's own suite (fixtures_matrix/golden_s35/semantic/merge/web_budgets) has no baseline execution record — see REQUEST-ORCH-RUN in W11-A02 log.
