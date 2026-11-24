# AUDIT: crates/product-config/src/schema.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines / functions | 639 / 14 (+7 tests) |
| File verdict | REAL (with release-lane reachability gap — see §7) |
| Reachable from shipped entry point? | NO from release lane (lumen-cli/desktop do not depend on product-config); YES from apps/control-server binary: main.rs:174 `load_product` -> main.rs:145 `text.parse::<product_config::Schema>()` -> schema.rs:50 `FromStr` -> load.rs `load_file` -> `Schema::validate` (schema.rs:120) -> `check_node` (133) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | rg over product.schema.json (keyword inventory, exit 0); rg for `default`/`root()`/`const` readers (exit 0/1); rg deps loop over Cargo.tomls (exit 1 = only control-server+xtask match); sed quote verification (exit 0); baseline test citation |

## 1. One-paragraph verdict
This is a real, working hand-rolled JSON-Schema-subset validator: the keyword walk in `check_node` (type/const/enum/not.enum/minimum/maximum/minLength/maxLength/pattern/minItems/maxItems/items/required/additionalProperties) matches the keyword inventory of the committed product.schema.json (verified by keyword grep: no exclusiveMin/Max, multipleOf, oneOf/anyOf/allOf, format, if/then/else, prefixItems — the subset claim in the header is honest). All 7 unit tests pass (baseline cargo-test-workspace.txt:5056-5062) and the fixture matrix exercises the real schema including `$ref` targets (pattern-msgid/bcp47/urls rows). The defects are silent-permissiveness paths: an unresolvable `$ref` silently disables validation for that subtree (the loud-drift promise is implemented for `pattern` only), 6 `default` annotations in the schema are read by nothing, and `Schema::root()` plus the `const` reader have zero exercisers. The Wave-1 claim "6 default keywords read by nothing" is CONFIRMED.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt lines 5056-5062: all 7 `schema::tests::*` pass (valid/missing-required/below-minimum/unknown-key-partial/partial-mode/unmatched-pattern/wrong-type).
— verified by `rg -n '"default"' product.schema.json` -> exactly 6 hits (118,119,236,238,239,240) and `rg '"default"|get("default")'` over crates/product-config/src + tools/xtask/src/cmd/config_validator.rs -> zero readers.
— verified by `rg -n '"$ref"|\$defs' product.schema.json` -> 12 refs, all one-hop into `$defs` (msgid/bcp47/https_url/repo_path), all four defs exist; no chained refs in the committed schema.

## 3. Findings
F-001 [S2]
Where: crates/product-config/src/schema.rs:104-116
Code:
```
    /// Resolve a local `$ref` (`#/$defs/name`) to its target node.
    fn resolve<'a>(&'a self, node: &'a Value) -> &'a Value {
        match node.get("$ref").and_then(Value::as_str) {
            Some(reference) if reference.starts_with("#/") => {
                let pointer = &reference[1..];
                match self.root.pointer(pointer) {
                    Some(target) => target,
                    None => node,
                }
            }
            _ => node,
        }
    }
```
What is wrong: three silent-permissiveness failures in one function. (a) An unresolvable pointer (`None => node`) returns the bare `{"$ref": ...}` node to `check_node`, which then finds no type/pattern/enum on it and emits ZERO diagnostics — validation for that key is silently off. (b) Resolution is single-level: if a `$defs` target itself contains a `$ref` (chained ref, legal in 2020-12), the second hop is never resolved and its constraints are silently skipped. (c) A remote `$ref` (not starting `#/`) falls to `_ => node` — also silently unvalidated. Why it matters: the module's own contract is the opposite — `from_str` promises "schema drift fails loudly instead of validating nothing" (schema.rs:53-54) and `check_pattern_coverage` (74-93) implements exactly that for `pattern`; `$ref` drift gets no such check at load time. Renaming a `$defs` entry (e.g. `repo_path` -> `repo_file`) would silently stop validating all 4 brand asset paths, both msgids, 8 bcp47 array slots, and 4 urls, with every test still green unless a fixture happens to be invalid at exactly those keys. What correct looks like: a load-time `check_ref_coverage` mirroring `check_pattern_coverage` (walk, resolve, error on `None`/non-local/ref-chains), plus a loop in `resolve`. Currently latent: all 12 committed refs resolve one-hop (verified). Confidence: HIGH — Verification: read + rg inventory of refs.

F-002 [S3]
Where: product.schema.json:118-119,236-240 (consumed by this engine)
Code:
```
        "start_maximized":   { "type": "boolean", "default": false },
        "remember_geometry": { "type": "boolean", "default": true },
```
```
        "crash_reports": { "enum": ["off", "ask", "send"], "default": "off" },
        "usage_stats":   { "type": "boolean", "default": false },
        "update_checks": { "type": "boolean", "default": false }
```
What is wrong: all 6 `default` annotations are dead — nothing in the tree reads `default` (rg over crates/product-config/src, tools/xtask/src/cmd: zero hits). The real defaults live in `model.rs` `Default` impls (Telemetry: model.rs:216-224 `crash_reports: CrashReports::Off, usage_stats: false, update_checks: false`; Window start_maximized/remember_geometry) and are ALSO restated explicitly in product.toml:72-129. Three independent copies of the same 6 values, one of them (the schema) writable-but-inert: change `"default": "off"` and nothing changes anywhere. Why it matters: the schema is advertised as "the single source of truth" (schema.rs:4) — for defaults it is a third source that can silently drift. What correct looks like: either delete the annotations or make the defaults pipeline read them. Confidence: HIGH — Verification: read + rg. (Confirms Wave-1 finding; count is exactly 6.)

F-003 [S3]
Where: crates/product-config/src/schema.rs:69-72
Code:
```
    /// The parsed schema document (for tools and tests).
    pub fn root(&self) -> &Value {
        &self.root
    }
```
What is wrong: zero callers anywhere (rg `\.root\(\)` and `Schema::root` over apps/, tools/, crates/product-config, settings-schema, localization, theme-tokens: only control-server's unrelated `root_schema()` fns matched). The doc comment claims "for tools and tests" but no tool and no test uses it. Dead pub API. Confidence: HIGH — Verification: read + rg.

F-004 [S3]
Where: crates/product-config/src/schema.rs:163-176
Code:
```
        // const
        if let Some(const_value) = node.get("const")
            && value != const_value
        {
```
What is wrong: the `const` reader is implemented and advertised in the module header (schema.rs:6 lists `const` among read keywords), but nothing exercises it: product.schema.json contains no `const` (rg: 0 hits), no unit test covers it (tests at 532-639 touch type/required/range/pattern/unknown-key only), no fixture uses it (rg `"const"` in tests/: 0 hits). If the `const` branch were deleted, every test in the repo stays green. Untested keyword support is a claim, not a capability. Confidence: HIGH — Verification: read + rg.

F-005 [S4]
Where: crates/product-config/src/schema.rs:496-499
Code:
```
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        _ => true,
    }
```
What is wrong: an unknown `type` string in the schema (typo like `"strng"`) matches every value — silently permissive, no load-time type-name check (contrast: patterns get one). Same class as F-001, lower blast radius. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/product-config/src/schema.rs:226-231 and 376-378
Code:
```
            let unit = node
                .get("x-lumen-unit")
                .and_then(Value::as_str)
                .filter(|u| *u == "MiB");
```
```
                        Some(schema) => schema,
                        None => continue,
```
What is wrong: (a) `x-lumen-unit` values other than `"MiB"` silently lose unit expansion in diagnostics; (b) an object schema that forgets `additionalProperties` silently skips all unknown children. Both are permissive-by-default escape hatches; the committed schema only uses MiB and declares `additionalProperties` on every object (verified by reading product.schema.json in full), so today these are latent. Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/product-config/src/schema.rs:132, 386, 447
Code:
```
    #[allow(clippy::too_many_arguments)]
```
What is wrong: three lint suppressions instead of a context struct (file/lines/path/mode repeated through every helper). Part of the 121-#[allow] tree debt. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FromStr::from_str | 50-58 | REAL | - | parse + coverage gate; executed by control-server runtime + tests |
| Schema::from_json | 63-67 | REAL | - | delegates to check_pattern_coverage |
| Schema::root | 70-72 | UNREACHABLE | S3 | zero callers (F-003) |
| check_pattern_coverage | 74-93 | REAL | - | walk is correct incl. arrays; pattern-only (F-001) |
| empty_for_tests | 97-102 | REAL | - | cfg(test), used by load.rs:189 |
| resolve | 105-116 | REAL | S2 | silent fallback + single-level + remote-ref passthrough (F-001) |
| validate | 120-130 | REAL | - | entry point; executed via fixtures_matrix + control-server |
| check_node | 133-384 | REAL | S2 | core walk; keyword handling verified against real schema; permissive paths F-005/F-006 |
| range_diagnostic | 387-445 | REAL | - | MiB expansion; exercised by range fixtures (baseline ok) |
| diagnostic | 448-477 | REAL | - | line attribution via lines.line_of fallback-to-ancestor (lines.rs:64-76) |
| child_path | 482-488 | REAL | - | trivial |
| type_matches | 490-500 | REAL | S4 | `_ => true` permissive default (F-005) |
| describe_value | 502-520 | REAL | - | trivial |
| generic_hint | 522-530 | REAL | - | x-lumen-hint -> description -> generic |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| valid_value_produces_no_diagnostics (562) | weakly | asserts empty on one happy value; would NOT catch over-permissiveness (F-001/F-005/F-006 leave it green) |
| missing_required_is_reported_with_ancestor_line (569) | YES | required-loop deletion fails it |
| below_minimum_reports_range_with_expected_and_found (577) | YES | range-check deletion fails it |
| unknown_key_is_reported_even_in_partial_mode (588) | YES | covers did_you_mean hint |
| partial_mode_skips_required_but_keeps_value_rules (606) | YES | mode gating |
| unmatched_pattern_fails_schema_load_loudly (620) | YES | coverage gate |
| wrong_type_short_circuits_value_checks (632) | YES | early-return + type check |
None of the 7 uses `$ref`, `const`, `items`, `minItems`/`maxItems`, or the real product.schema.json — those paths rest on fixtures_matrix.rs (which does cover $ref via pattern-msgid/urls/bcp47 and items-recursion via `preferred_audio_languages.0`, fixtures_matrix.rs:297-328) and the control-server e2e.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "engine below reads type, required, additionalProperties, enum, const, minimum, maximum, minLength, maxLength, minItems, maxItems, items, pattern, not.enum, x-lumen-unit, x-lumen-hint" | schema.rs:4-9 | REAL | every keyword has a reader (lines 146,330,351,179,164,231-232,244,257,292,305,318,271,205,228,523); but `const` reader has zero exercisers (F-004) |
| "schema drift fails loudly instead of validating nothing" | schema.rs:53-54 | SHALLOW | true for `pattern` only; `$ref` drift is silent (F-001) |
| "An empty `enum: []` marks an injected enum... defers to semantic stage" | schema.rs:10-12 | REAL | code at 179-180 `!variants.is_empty()` gate; injected-enum fixtures exist (injected-enum-theme.toml etc.) |

## 7. Dead / duplicated / unreachable code
- `Schema::root()` — dead (F-003).
- `const` reader — dead-in-practice (F-004).
- Release lane: neither lumen-cli nor desktop depends on product-config (dep loop over all Cargo.tomls: only apps/control-server + tools/xtask). Desktop reads product.toml through its own `parse_window_table(include_str!("../../../product.toml"))` (apps/desktop/src/main.rs:31) — the shipped GUI bypasses this validator entirely. apps/cli/src/host.rs:969 admits CLI limits "ride the product.toml mirror constants". So from the shipped release lane this file is UNREACHABLE; the only binary consumer is control-server (built/tested in CI but not in the `-p lumen-cli -p desktop` release lane).
- Parallel implementation: tools/xtask/src/cmd/config_validator.rs re-implements the structural walk with its own toml_lite parser and enforces NO patterns (its header, lines 17-21) — the weaker validator is the one product.toml's header names (MP §5). Cross-file duplication of the schema semantics.

## 8. Dependencies & flags
serde_json (parse + pointer), thiserror, crate::diagnostic/lines/patterns/suggest. No feature gates. `let`-chains (if-let && ) require Rust 2024 — consistent with workspace edition.

## 9. Open questions for the next agent
- Does crates/product-config/src/semantic.rs actually enforce short_name reserved names (schema $comment at product.schema.json:24 claims it does)? Pattern-reader is fine; the semantic side is unverified here.
- Who validates product.toml in the desktop release lane if xtask's validator skips patterns and desktop bypasses the crate? (Candidate S2 for a cross-cutting audit; not chargeable to this file alone.)
