# AUDIT: crates/theme-tokens/src/parse.rs
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 736 / 18 (+4 inner closures) |
| File verdict | UNREACHABLE from shipped entry points (real, baseline-test-executed dev/codegen tool) |
| Reachable from shipped entry point? | NO from `lumen`/`lumen-tauri` — no crate depends on `theme-tokens` (rg over all `Cargo.toml` in crates/ and apps/: only self-references). Real call chains that DO reach it: `cargo xtask tokens` → `tools/xtask/src/cmd/tokens.rs run()` → `cargo run -p theme-tokens --bin tokens-gen` → `crates/theme-tokens/src/bin/tokens_gen.rs` `run()` `parse(&source)` (bin:111); and `cargo test --workspace` → `crates/theme-tokens/tests/pipeline.rs:27,375,385` + `tests/diff.rs:23` call `parse()` directly — executed in baseline run #3 (`audit/runs/baseline/test-workspace.log.txt`, 1681 passed; `crates/*` glob membership per root Cargo.toml). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 3 |
| Commands run | `Read` parse.rs 1-737 (complete); `rg -n "theme.tokens" --glob Cargo.toml crates apps` (zero dependents → unshipped); `cat tools/xtask/src/cmd/tokens.rs` + `src/bin/tokens_gen.rs` (call chain); `rg -n "value_for" -A 12 model.rs` (PerMode+None → None, basis of F-003 misleading error); `rg -n "inherit|$type" docs/08-ui-figma-fidelity.md` (doc 08 §211 names the W3C draft, no local precedence override); `rg -n "members" -A 12 Cargo.toml` (crates/* glob → tests run in workspace); python walk of `themes/tokens.json` (295 tokens, 0 own-vs-inherited `$type` conflicts → F-001 latent); `sed -n` on every quoted range (verbatim verified). |

## 1. One-paragraph verdict
This is a genuinely implemented, defensively-written parser for the W3C design-token subset — real error taxonomy, real cycle detection with a visiting stack, real memoized resolution, real negative tests — and baseline `cargo test --workspace` executed it against the committed 295-token source (1681 passed). It is also unreachable from anything shipped: it is a codegen tool consumed by `cargo xtask tokens` and the test gate only, and its outputs (tokens.css/tokens.rs) reach the UI only as a verbatim byte-copy (ui/scripts/gen-tokens.mjs) into a webview app whose host crate cannot compile at HEAD (W14). Three latent defects survive inside it: `$type` precedence is inverted against the DTCG draft it claims to implement (inherited group type silently overrides a token's own `$type`); source-authoring errors (missing accent-family token, non-color accent primary, missing ramp source) are classified `TokenError::Internal` and rendered to the author as "a bug in theme-tokens — please report it"; and `ramp_from_file` hard-codes the mode names "dark"/"light" ignoring the registered `meta.modes`, failing with a wrong message ("is not a color") on any source that registers other mode names.

## 2. What actually works (verified)
— Verified by baseline `audit/runs/baseline/test-workspace.log.txt` (run #3, 1681 passed/1 failed/24 ignored — the failure is `apps/desktop/tests/zero_alloc_transport.rs`, unrelated): `parse()` runs end-to-end over the committed `themes/tokens.json` inside `pipeline.rs::committed_source_parses_with_the_documented_shape` and the diff-gate `diff.rs`, proving parse+resolve+build_set on 295 tokens.
— Verified by read: cycle detection (`resolve_token:264-268` pushes/compares full path chain, `TokenError::cycle` renders `a -> b -> a`); memoization is sound (`resolved[i]` set only after success, line 322); per-mode coverage and unknown-mode rejection are both enforced (283-288, 310-317); layer rules match the module-doc contract exactly (primitive=raw, alias→primitive|alias, semantic→alias only; `check_layer_ref:338-361`); `parse_px`/`parse_dimension` both require finite, non-negative values (427, 445).
— Verified by read: `parse_shadow_color` rejects alpha outside 0..=1 (542-544) and non-#rrggbb/rgba forms with typed messages; `parse_hex` (color.rs:29-41) enforces exactly 7 bytes `#rrggbb`.

## 3. Findings

### F-001 [S3]
Where: crates/theme-tokens/src/parse.rs:103-113 (precedence), 91-95 (own_type), 3-7 (module-doc contract)
Code:
```rust
            let token_type = own_type
                .clone()
                .or_else(|| {
                    child
                        .get("$type")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
```
What is wrong: a token's own `$type` is consulted only when the enclosing group supplied none (`or_else` on `own_type`). The W3C Design Tokens draft this module claims to implement (module doc, lines 3-4: "W3C Design Tokens draft subset — nested groups, `$value`, `$type` (inheritable via groups)") defines the opposite precedence: a token-level `$type` overrides the inherited group-level `$type`. Here a group declaring `"color"` with a child token declaring `"$type": "dimension"` is parsed and validated as a color (or fails with a color error), the author's explicit type silently discarded.
Why it matters: this is a validator whose one job is to catch authoring mistakes; it silently accepts a contradictory one. A future token under a typed group (exactly the "token AND group" node the file's own lumen extension at 120-129 encourages) gets the wrong `ResolvedValue` shape and the wrong generated constant. Latent today: the committed `themes/tokens.json` has 0 own-vs-inherited conflicts (verified by a full walk: 295 tokens, no token carries an own `$type` differing from its inherited group type), so current outputs are correct — the defect is the trap, not the output.
What correct looks like: `child.get("$type").and_then(Value::as_str).map(str::to_owned).or(own_type.clone())` — own type first, inherited as fallback — plus a negative test with a conflicting token.
Confidence: HIGH — Verification: read (both branches traced; DTCG precedence is the published draft rule; committed-source latency verified by scripted walk of themes/tokens.json).

### F-002 [S3]
Where: crates/theme-tokens/src/parse.rs:616-620 (also 630-632, 692-699 — same class)
Code:
```rust
        let Some(token) = semantic.iter().find(|t| t.path == role) else {
            return Err(TokenError::internal(format!(
                "accent-family semantic token `{role}` is missing (doc 08 §3.3)"
            )));
        };
```
What is wrong: a defect in the token SOURCE (the author omitted `semantic.color.accent`, or pointed `accent-primaries` at a non-color token, lines 630-632, or omitted a ramp source, lines 692-699) is reported as `TokenError::Internal`, whose Display (lib.rs) renders "internal error: … (a bug in theme-tokens — please report it)". These are parse/validation failures of user input, not crate bugs. lib.rs claims "All variants carry actionable context" — the action it tells the author to take is "report a bug" for their own typo.
Why it matters: the whole error taxonomy exists to route authoring mistakes to `Parse`; misrouting them to Internal sends a theme author on a bug-report goose chase and breaks the crate's own diagnostic contract. Blast radius limited to the dev tool, but the tool's entire user interface IS its diagnostics.
What correct looks like: `TokenError::parse(&role, "missing required accent-family semantic token (doc 08 §3.3)")` for all three sites (Internal reserved for the genuine invariants at 560-562/683-685, which are correctly classified).
Confidence: HIGH — Verification: read (all three sites traced; lib.rs Display text read).

### F-003 [S3]
Where: crates/theme-tokens/src/parse.rs:706-710 (hard-coded modes), model.rs:91-99 (value_for)
Code:
```rust
        tinted_dark: color("color.accent.tinted", Some("dark"))?,
        tinted_light: color("color.accent.tinted", Some("light"))?,
        bright_light: color("color.accent.bright", Some("light"))?,
        bright_dark: color("color.accent.bright", Some("dark"))?,
```
What is wrong: `ramp_from_file` hard-codes the mode names "dark"/"light" instead of `meta.modes`. The pipeline otherwise treats modes as data (`read_meta` validates arbitrary non-empty mode lists; `resolve_token:283-288` accepts any registered mode). A source registering e.g. `["night","day"]` reaches `build_set`, and `value_for(Some("dark"))` on a per-mode token without "dark" returns `None` (model.rs:94-97), which the `color` closure converts to "ramp source token `color.accent.tinted` is not a color" — a false statement about the token (it IS a color; the mode name is wrong).
Why it matters: the extension block invites custom mode sets, and this function answers a mode-name mismatch with a factually wrong diagnostic pointing at the token instead of the mode list. Latent for the committed source (registers exactly ["dark","light"], read from themes/tokens.json `$extensions`).
What correct looks like: derive the two mode names from `meta.modes` (or require exactly dark/light at `read_meta` with a parse error naming the actual registered modes), and make the `color` closure distinguish "missing mode" from "not a color".
Confidence: HIGH — Verification: read (value_for traced; committed modes read).

### F-004 [S4]
Where: crates/theme-tokens/src/parse.rs:389-394
Code:
```rust
        "number" | "fontWeight" => {
            let n = v
                .as_f64()
                .ok_or_else(|| err("value must be a JSON number".into()))?;
            Ok(ResolvedValue::Number(n))
        }
```
What is wrong: no `is_finite` check (contrast `parse_dimension`:427 and `parse_px`:445, which both validate finiteness) — a JSON `1e999` would flow through as a non-finite `Number` into generated output; `fontWeight` additionally accepts any float (450.5) and any range (CSS allows 1-1000 integers only). Also `"fontFamily"` (395-408) accepts an empty array (`Family(vec![])`).
Why it matters: minor — a validator that checks finiteness for dimensions but not numbers is inconsistent, and a bogus font weight/empty family would surface as broken generated CSS/Rust constants rather than a parse error.
What correct looks like: `if !n.is_finite() { return Err(...) }` plus a 1..=1000 integer check for fontWeight and a non-empty check for fontFamily arrays.
Confidence: MEDIUM — Verification: read (absence of checks verified; the serde_json inf-passthrough itself not executed — UNVERIFIED, no cargo allowed; the missing-check fact is read-verified).

### F-005 [S4]
Where: crates/theme-tokens/src/parse.rs:359
Code:
```rust
        _ => Ok(()),
    }
```
What is wrong: dead wildcard arm in `check_layer_ref`. Every token path is rooted at one of the three layer groups (`parse:59-64` iterates exactly `["primitive","alias","semantic"]`), and the three prior arms cover all combinations of those `from_layer`s, so `_ => Ok(())` can never execute — worse, if it ever DID execute it would silently approve a layer violation.
Why it matters: a permissive dead arm in a validator is a loaded footgun — any future fourth layer would be silently exempt from all layering rules instead of failing.
What correct looks like: `_ => unreachable!(...)` or, better, an explicit `Err` naming the unrecognized layer.
Confidence: HIGH — Verification: read (all call sites of check_layer_ref are inside resolve_token with layer-rooted paths).

### F-006 [S4]
Where: crates/theme-tokens/src/parse.rs:168-179 (strings closure), 180-215 (no dedup)
Code:
```rust
    let strings = |key: &str| -> Result<Vec<String>, TokenError> {
        ext.get(key)
            .and_then(Value::array)
            .ok_or_else(|| TokenError::parse("$extensions", format!("missing `{key}` array")))?
            .iter()
```
What is wrong: `read_meta` validates presence, stringness, and non-emptiness of `modes`/`accents`/`on-accent-candidates` but never rejects duplicate entries — `"modes": ["dark","dark","light"]` passes validation, and the duplicate propagates into `TokenSet.modes` and every downstream consumer (emit, coverage checks).
Why it matters: a duplicate mode makes the per-mode coverage check semantics ambiguous and the emitted mode lists wrong; it is exactly the class of typo a validator exists to catch.
What correct looks like: after `strings()`, check `modes.len() == modes.iter().collect::<HashSet<_>>().len()` (same for accents) and emit a parse error naming the duplicate.
Confidence: HIGH — Verification: read (no dedup anywhere between 180-249).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse` | 50-78 | REAL | — | Entry point; three-layer walk, index, memoized resolve, build_set. Error paths typed. Executed by baseline tests. |
| `walk_group` | 81-135 | REAL | F-001 | `$type` inheritance; lumen token+group extension (120-129) correctly descends both; recursion bounded by JSON tree depth. |
| `classify_value` | 140-154 | REAL | — | per-mode detection = all-keys-are-modes; empty object rejected (144-146). Sound. |
| `read_meta` (fn) | 156-249 | REAL | F-006 | Validates presence/membership of default-mode/default-accent; no duplicate rejection. |
| `read_meta::strings` (closure) | 168-179 | REAL | — | Correct array-of-strings extraction. |
| `resolve_token` | 252-324 | REAL | — | Memoized DFS with visiting-stack cycle detection; per-mode reference to per-mode target resolves per-mode correctly (292-304); coverage check 310-317. |
| `reference_target` | 327-334 | REAL | — | `{...}` detection, len>2; non-reference strings pass through. |
| `check_layer_ref` | 338-361 | REAL | F-005 | Layer grammar enforced exactly as documented; dead `_ => Ok(())` arm. |
| `lookup` | 363-368 | REAL | — | Typed unknown-reference error. Trivial, correct. |
| `parse_typed_value` | 371-413 | REAL | F-004 | Dispatch color/dimension/number/fontWeight/fontFamily/shadow; no finite/range/empty checks (F-004). |
| `parse_dimension` | 416-435 | REAL | — | px/ms suffixes, finite + non-negative enforced. Correct. |
| `parse_px` | 438-453 | REAL | — | px only, finite + non-negative. Correct. |
| `parse_shadow` (fn) | 459-484 | REAL | — | Object form + shorthand delegation; `as f32` narrowing of validated f64s is lossless for sane px magnitudes. |
| `parse_shadow_shorthand` (+`field` closure) | 489-517 | REAL | — | Whitespace-split 4 geometry fields + remainder-as-color; rejects missing fields/colors with typed messages. Correct. |
| `parse_shadow_color` (+`chan` closure) | 520-549 | REAL | — | #rrggbb or rgba(0-255, 0-255, 0-255, 0..=1); alpha range enforced (542-544). Correct. |
| `build_set` (fn) | 552-661 | REAL | F-002 | Assembles TokenSet; accent-family roles; ramps (default from file, rest derived+contrast-checked via color.rs). Internal misclassification at 616-620/630-632. |
| `build_set::find` (closure) | 627-634 | REAL | F-002 | resolve_reference to Rgb8; non-color → Internal (misclassified). |
| `resolve_reference` | 665-687 | REAL | — | Extension `{path}` → Single value; per-mode target rejected (680-682, correctly Internal — this one IS a source/authoring-shape mismatch though "internal" is debatable; per-mode primaries are a parse-time shape question). |
| `ramp_from_file` (+`color` closure) | 690-713 | REAL | F-003 | Hard-coded dark/light mode names; misleading "not a color" on mode mismatch. |
| `rust_category_prefix` | 716-736 | REAL | — | 10-category prefix map; unknown category → typed parse error. Correct. |

## 5. Test honesty (tests live in crates/theme-tokens/tests/, not this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pipeline.rs::committed_source_parses_with_the_documented_shape | YES | Parses the real committed source, asserts layer counts and extension invariants against doc 08. |
| pipeline.rs::resolved_values_match_doc_08_section_1_2 | YES | Spot-resolves specific token paths to specific RGB/dimension values from the doc — a real oracle. |
| pipeline.rs::accent_ramps_follow_the_contrast_rule | YES | Recomputes WCAG contrast per ramp and asserts ≥4.5:1 — would catch a broken derive_ramp/ramp_from_file. |
| pipeline.rs::emission_is_deterministic + emitted_documents_have_the_documented_structure | YES | Re-runs parse+emit twice, asserts byte-equality and structural selectors. |
| pipeline.rs::layer_rules_are_enforced / reference_errors_are_actionable / per_mode_coverage_is_enforced / value_parsing_rejects_bad_values / meta_errors_are_actionable / unsupported_types_and_categories_are_rejected | YES | Real negative tests via `expect_error(source, needle)` — each feeds a malformed mini-source and asserts the error text. Would fail if the corresponding validation were removed. |
| diff.rs (3 drift tests) | YES | Regenerates from committed source and byte-compares against the three committed outputs (crate CSS, crate Rust, UI bridge copy) — catches any parse/emit drift. |
| Gaps | — | No test covers: own `$type` overriding group `$type` (F-001's trap), non-default mode names (F-003), duplicate modes/accents (F-006), non-finite numbers (F-004). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "parse" is the doc 01 §2.3 token pipeline entry | doc 01 §2.3 (per module doc line 49) | REAL | parse():50-78 exists, wired to tokens-gen bin + tests; baseline test run #3 executed it. |
| "W3C Design Tokens draft subset — … `$type` (inheritable via groups)" | module doc 3-4, citing doc 08 §211 ("W3C Design Tokens JSON format (draft)") | FALSE (partially) | F-001: draft precedence (token overrides group) is inverted in code; committed source unaffected (0 conflicts, scripted walk). |
| "All variants carry actionable context" (error taxonomy) | lib.rs TokenError doc | FALSE (partially) | F-002: source errors routed to Internal whose text says "a bug in theme-tokens — please report it". |
| "every accent has a primary color and an on-accent glyph with WCAG contrast ≥ 4.5:1 (doc 08 §3.3)" | module doc 16-17 | REAL | Enforced for derived ramps via color::derive_ramp → TokenError::Contrast (parse.rs:645-647); pipeline.rs::accent_ramps_follow_the_contrast_rule recomputes it; ran in baseline #3. |
| Per-mode coverage "every per-mode token covers every registered mode" | module doc 14 | REAL | resolve_token:310-317; negative-tested (pipeline.rs per_mode_coverage_is_enforced). |

## 7. Dead / duplicated / unreachable code
- `check_layer_ref` wildcard arm `_ => Ok(())` (line 359) — dead (F-005).
- Whole file is unreachable from shipped binaries (`lumen`, `lumen-tauri`): zero dependents in any Cargo.toml (verified by rg across crates/ and apps/ manifests). It is legitimately-scoped dev tooling (codegen + gate), so this is noted, not condemned — but nothing the end user runs executes this parser.

## 8. Dependencies & flags
- `serde_json` 1.0.154 sole dependency (matches Cargo.toml's pre-audit policy note). No feature flags, no cfg gating, no unsafe. Workspace lints inherited.

## 9. Open questions for the next agent
- emit.rs / color.rs / lint.rs / model.rs are NOT in my assignment — F-003's blast radius into `emit` (how modes are rendered into `:root[data-theme=…]` selectors) and F-001's into `lint.rs` (does lint_source check `$type` precedence?) need their owner's eyes.
- Whoever audits `ui/src/tokens/`: confirm the bridge copy `ui/src/tokens/generated/tokens.css` is byte-identical to `themes/generated/tokens.css` (diff.rs asserts it, but the assertion runs only under `cargo test`, which the UI-only npm pipeline never invokes).
