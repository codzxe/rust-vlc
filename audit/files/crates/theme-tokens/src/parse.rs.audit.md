# AUDIT: crates/theme-tokens/src/parse.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 737 / 18 (parse, walk_group, classify_value, read_meta, resolve_token, reference_target, check_layer_ref, lookup, parse_typed_value, parse_dimension, parse_px, parse_shadow, parse_shadow_shorthand, parse_shadow_color, build_set, resolve_reference, ramp_from_file, rust_category_prefix) |
| File verdict | REAL-UNPROVEN (execution evidence = orchestrator baseline; own run blocked by sandbox linker failure) |
| Reachable from shipped entry point? | NO — codegen toolchain only: `cargo xtask tokens` (tools/xtask/src/cmd/tokens.rs:58 `run`) shells `cargo run -p theme-tokens --bin tokens-gen` → crates/theme-tokens/src/bin/tokens_gen.rs `run()` → `theme_tokens::parse` (this file:50). No crate in the release lane depends on theme-tokens (rg over all Cargo.toml: zero `[dependencies]` entries); its committed OUTPUTS are consumed (ui/src/tokens/generated/tokens.css/.ts, crates/theme-tokens/src/generated/tokens.rs). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `cargo test -q -p theme-tokens` → FAILED at link: `collect2: fatal error: ld terminated with signal 7 [Bus error]` (sandbox disk, same class as baseline attempt1-diskpoisoned) — no test execution. Baseline cited instead. python3 scan of themes/tokens.json (modes/accents/295 tokens/no $type conflicts). sed verifications of all quotes (exit 0). |

## 1. One-paragraph verdict
This is a real, complete recursive-descent validator for a W3C-design-tokens subset with a per-mode extension: it enforces the 3-layer grammar (primitives raw; alias→primitive/alias; semantic→alias only), reference resolution with cycle detection and memoization, per-mode coverage, typed value parsing (color/dimension/number/fontFamily/shadow incl. CSS shorthand), and assembles accent ramps with contrast validation for non-default accents. It is honest engineering with one genuine spec deviation (group `$type` shadows a token's own `$type`, inverting W3C precedence) and a handful of hygiene issues. It is build-time tooling, not shipped runtime code — no release-lane binary contains it.

## 2. What actually works (verified)
- Baseline orchestrator run: audit/runs/baseline/cargo-test-workspace.txt:6250-6341 — theme-tokens: 22 lib tests + 2 diff + 12 pipeline + 2 semantic_lint passed, `### PACKAGE theme-tokens exit_code=0`. Includes `committed_outputs_match_regeneration` (idempotence) and `committed_source_parses_with_the_documented_shape` (exact counts 123/86/86 — matches my independent python count of 295 tokens).
- My python3 scan of themes/tokens.json: modes=[dark,light], 9 accents, 4 on-accent-candidates, 295 tokens; no token where own `$type` conflicts with inherited group `$type` (so F-001 is latent, not live).

## 3. Findings
F-001 [S3]
Where: crates/theme-tokens/src/parse.rs:103-110
Code:
```
            let token_type = own_type
                .clone()
                .or_else(|| {
                    child
                        .get("$type")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
```
What is wrong: `own_type` is the *inherited group* `$type`; it takes precedence over the token's own `$type` (the `or_else` only fires when no group type exists). The W3C Design Tokens draft the module header claims to subset (lines 3-4: "W3C Design Tokens draft subset — nested groups, `$value`, `$type` (inheritable via groups)") gives the token's own `$type` precedence over an inherited one. A token that sets `$type` under a typed group silently gets the group's type — a shadow under a `{"$type":"color"}` group would be parsed as a color and fail with a misleading "bad color" error. Why it matters: silent deviation from the declared format spec; misdiagnosed errors for valid-per-spec input. What correct looks like: try the child's own `$type` first, fall back to the inherited one. Latent today: my scan shows zero conflicts in the committed themes/tokens.json.
Confidence: HIGH — Verification: read + python scan of the committed source.

F-002 [S4]
Where: crates/theme-tokens/src/parse.rs:59-64
Code:
```
    for layer in ["primitive", "alias", "semantic"] {
        let node = root_obj
            .get(layer)
            .ok_or_else(|| TokenError::parse(layer, "missing required layer group"))?;
        walk_group(node, layer, None, &meta, &mut raw)?;
    }
```
What is wrong: any top-level group other than the three layers (e.g. a typo `semantics:` or a stray `tokens:` block) is silently ignored — no unknown-key diagnostic. A whole mis-named subtree of tokens vanishes without an error. Blast radius: dev-authored file only.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/theme-tokens/src/parse.rs:358-360
Code:
```
        _ => Ok(()),
    }
}
```
What is wrong: dead match arm in `check_layer_ref`. `from` is always a path whose first segment is one of `primitive`/`alias`/`semantic` (built by `walk_group` from those three roots), so the catch-all is unreachable; if it ever were reached it would *allow* an unknown layer reference — a fail-open default in a validator.
Confidence: HIGH — Verification: read (call-site analysis: `walk_group` is only invoked with the three layer roots, parse.rs:63).

F-004 [S4]
Where: crates/theme-tokens/src/parse.rs:426-434 (and parse_px:444-452)
Code:
```
            if v.is_finite() && v >= 0.0 {
                Ok((v, unit))
            } else {
                Err(format!(
                    "dimension must be finite and non-negative, got `{s}`"
```
What is wrong: negative geometry is rejected for ALL dimensions including shadow `offsetX/offsetY`, but CSS box-shadow semantics allow negative offsets (a shadow up-left of the box). The parser subset therefore cannot express a valid class of shadows. Not live: the committed themes/tokens.json contains no negative dimension (rg '\-[0-9.]+px' matches only inside `$description` prose, lines 315/391).
Confidence: HIGH — Verification: read + rg over tokens.json.

F-005 [S4]
Where: crates/theme-tokens/src/parse.rs:327-334
Code:
```
    let s = v.as_str()?;
    if s.len() > 2 && s.starts_with('{') && s.ends_with('}') {
        Some(s[1..s.len() - 1].to_owned())
```
What is wrong: `{ path }` with inner whitespace is not recognized as a reference; it flows into `parse_typed_value` and dies as e.g. "bad color `{ primitive.color.gray.0 }`" instead of an unknown-reference diagnostic. Error quality only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse | 50-78 | REAL | — | Entry: JSON→meta→walk→resolve→build. Solid. |
| walk_group | 81-135 | REAL | S3 | $type precedence bug (F-001); dual token+group extension is deliberate and documented (120-129). |
| classify_value | 140-154 | REAL | — | Empty object rejected; all-modes-keys ⇒ per-mode; inherent dark/light-vs-composite ambiguity documented. |
| read_meta | 156-249 | REAL | — | Validates modes/accents non-empty, defaults registered, primaries present for every accent; extra accent-primaries entries silently ignored (S4-grade nit). |
| resolve_token | 252-324 | REAL | — | Memoized DFS with cycle stack; per-mode→per-mode reference picks same mode (292-304); coverage enforced (310-317). Correct. |
| reference_target | 327-334 | REAL | S4 | F-005. |
| check_layer_ref | 338-361 | REAL | S4 | F-003. |
| lookup | 363-368 | REAL | — | Trivial. |
| parse_typed_value | 371-413 | REAL | — | color/number/fontWeight/dimension/fontFamily/shadow; unknown $type rejected (411). |
| parse_dimension | 416-435 | REAL | S4 | F-004. |
| parse_px | 438-453 | REAL | S4 | F-004. |
| parse_shadow | 459-484 | REAL | — | Object + shorthand forms; `as f32` narrowing of f64 px values is lossless for sane magnitudes. |
| parse_shadow_shorthand | 489-517 | REAL | — | Color-as-remainder handles rgba() spaces (486-488 doc). |
| parse_shadow_color | 520-549 | REAL | — | #rrggbb or rgba(); alpha range-checked. |
| build_set | 552-661 | REAL | — | Hard-requires the six accent-family semantic roles (606-622); default-accent ramp mined from file, others derived+contrast-checked via color::derive_ramp. |
| resolve_reference | 665-687 | REAL | — | Linear scan O(n) per accent — fine at n=295. |
| ramp_from_file | 690-713 | REAL | — | Requires per-mode bright/icon/tinted; Single for accent/strong/on-accent. |
| rust_category_prefix | 716-736 | REAL | — | Closed category whitelist; unknown rejected. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pipeline::committed_source_parses_with_the_documented_shape | YES | Exact counts 123/86/86, modes, 9 accents, 6 accent-family css names — any parser drift trips it (verified logic, executed in baseline). |
| pipeline::layer_rules_are_enforced / per_mode_coverage_is_enforced / value_parsing_rejects_bad_values / reference_errors_are_actionable / meta_errors_are_actionable / unsupported_types_and_categories_are_rejected | YES | Crafted invalid mini-sources asserting specific error text. |
| diff::committed_outputs_match_regeneration | YES | Re-parses committed source and byte-compares to committed outputs — catches silent emission drift. |
| Execution note | UNVERIFIED (own run) | `cargo test -q -p theme-tokens` failed at link with `ld terminated with signal 7 [Bus error]` — sandbox disk/linker fault, same class as baseline attempt1-diskpoisoned. Baseline (exit_code=0, 38 tests) cited instead. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "W3C Design Tokens draft subset" | parse.rs:3-4 module doc | SHALLOW | Subset is real but $type precedence inverts the draft (F-001). |
| Enforced rules list (layers, per-mode coverage, acyclic refs, contrast ≥4.5:1) | parse.rs:9-17 | REAL | Layer rules:338-361; coverage:310-317; cycles:264-267; contrast delegated to color::derive_ramp (645-646) and asserted by pipeline::accent_ramps_follow_the_contrast_rule (baseline pass). |

## 7. Dead / duplicated / unreachable code
Dead arm F-003. `resolve_reference` duplicates lookup logic of `lookup()`+`index` but only for extension references (O(n) scan) — acceptable duplication, noted.

## 8. Dependencies & flags
serde_json only (lib.rs:30-31). No feature gates. Not a dependency of any release-lane crate.

## 9. Open questions for the next agent
- Does `color::derive_ramp` actually enforce WCAG ≥4.5:1 or just warn? (color.rs owned by another auditor; contrast claim rests there.)
