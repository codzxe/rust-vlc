# AUDIT: crates/product-config/tests/fixtures_matrix.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 378 / 29 (helpers rejected, one; 27 #[test] fns) |
| File verdict | REAL (strict rejection-matrix tests; executed in orchestrator baseline) |
| Reachable from shipped entry point? | Tests of `product_config::load_file` (structural validation), which IS production code: apps/control-server/src/main.rs:145-154 parses product.schema.json and calls `product_config::load_file`. The test binary itself runs in the workspace test lane (baseline: fixtures_matrix 27 passed, exit_code=0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | diff of the 4 assigned fixtures vs product.toml (exit 1, drift quantified); rg fixture-name coverage over tests/ (exit 0); sed verifications (exit 0). Baseline: audit/runs/baseline/cargo-test-workspace.txt — `Running tests/fixtures_matrix.rs ... running 27 tests / test result: ok. 27 passed`. |

## 1. One-paragraph verdict
A genuinely strict structural-validation matrix: every fixture must be REJECTED (`rejected()` fails on Ok), each test asserts EXACTLY ONE diagnostic for the documented key (`one()` asserts hits.len()==1), then pins rule text, found value, and source line via content markers. All 42 fixtures in tests/fixtures/ are consumed (verified by rg: 27 here, 17 in semantic.rs incl. deferred negatives, 1 in golden_s35.rs — matches fixtures/README.md's map). The one real defect is inherited, not local: the fixtures are STALE copies of an old product.toml, so the README's "exactly one documented mutation" regeneration contract is false today.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt: `Running tests/fixtures_matrix.rs (target/debug/deps/fixtures_matrix-84f1764e40a46a07) / running 27 tests / test result: ok. 27 passed; 0 failed`. Package exit_code=0.
- Fixture-consumption completeness: rg each of the 42 fixture filenames across crates/product-config/tests/*.rs → every fixture referenced; no orphan fixtures.
- diff of the 4 assigned fixtures vs product.toml → the intended mutation present in each (schema_version=2; migrations entry from=3,to=2; urls.help http://; icon-512.png PNG-magic ok / icon.ico "NOT-ICO" bytes via od).

## 3. Findings
F-001 [S3]
Where: crates/product-config/tests/fixtures/README.md:3-6 (contract) — evidenced via the fixtures consumed by this suite
Code:
```
Each fixture is the repo-root `product.toml` (doc 07 §2 draft) with exactly
one documented mutation — regenerate a fixture by re-applying its mutation
to a fresh copy of `product.toml` (the generation script was a one-off;
mutations are listed below).
```
What is wrong: FALSE as of the current tree. 39 of 40 fixtures carry `display_version = "0.1.0-dev"` while product.toml:27 has `display_version = "0.1.3"` (rg counts: 39/40). Every fixture also predates: the w28 min_width/min_height 940x600 fix (fixtures still 480x320), streaming-http=true (fixtures: false), the ADR-001-accepted comments, and the whole `[web.size_budgets]` section. So each fixture = old-product.toml + one mutation, i.e. MANY deltas from the current root, not "exactly one". Tests still pass because assertions are content-marker based, but the documented maintenance contract (regenerate from fresh product.toml) would change 40 files and the map's "Mutation" column understates the diff. Why it matters: fixture drift hides which validator behaviors are actually current-product-shaped (e.g. no fixture exercises [web.size_budgets] keys or the new 940 floor as VALID values); future regeneration is a silent 40-file churn. What correct looks like: regenerate all fixtures from the current product.toml, or restate the README contract as "frozen base + mutation".
Confidence: HIGH — Verification: ran diff + rg (39/40 stale).

F-002 [S4]
Where: crates/product-config/tests/fixtures_matrix.rs:197-211
Code:
```
fn range_limits_multi_reports_all_four_violations() {
    let name = "range-limits-multi.toml";
    let src = read_fixture(name);
```
(…and at 210:) `    let _ = src;`
What is wrong: `src` is read and then explicitly discarded — the only test in the file that does not pin line numbers via `marker_line`. Dead read + weakest assertion set of the matrix (`d.line.is_some()` instead of the exact line).
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/product-config/tests/fixtures_matrix.rs:306-312
Code:
```
    let d = one(&diags, "urls.help", name);
    assert!(d.rule.contains("https"), "{}", d);
    assert!(d.found.starts_with("\"http://"), "{}", d);
```
What is wrong: weakest rule assertion in the file — `contains("https")` would also match a rule merely mentioning "https" in prose; sibling pattern tests assert `"must match the pattern"`. The fixture (pattern-urls.toml, assigned) is correct; the assertion is looser than the rest of the matrix.
Confidence: HIGH — Verification: read.

## 4. Per-function table (test inventory — all 27 #[test])
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| invalid_syntax_is_a_parse_diagnostic_with_line | 39-47 | REAL | — | Stage::Parse + exact line. |
| missing_identity_section_is_reported | 52-60 | REAL | — | line None for missing table — honest. |
| missing_identity_field_points_at_the_table_header | 63-70 | REAL | — | Line = [identity] header. |
| missing_window_fields_reports_every_violation | 73-85 | REAL | — | Two diagnostics, both at [window] header — "every violation" doc 07 §3.4 stage 2. |
| missing_brand_asset_is_reported | 88-93 | REAL | — | |
| missing_defaults_key_is_reported | 96-101 | REAL | — | |
| missing_limits_key_is_reported | 104-109 | REAL | — | |
| enum_subtitle_mode_is_rejected_with_the_legal_values | 114-132 | REAL | — | Legal set enumerated in rule text; found+line pinned. |
| enum_hw_decode_policy_...post_adr001 | 141-159 | REAL | — | Asserts matrix-vocabulary term "hw-first" stays rejected — good negative vocabulary test. |
| enum_crash_reports_is_rejected | 162-168 | REAL | — | |
| range_window_width_is_rejected_with_line | 173-183 | REAL | — | Both bounds + expected 480..=3840 + line. |
| range_default_rate_is_rejected | 186-193 | REAL | — | |
| range_limits_multi_reports_all_four_violations | 196-211 | REAL | S4 | F-002. |
| type_window_width_is_rejected | 216-222 | REAL | — | |
| type_features_value_is_rejected | 225-231 | REAL | — | |
| pattern_short_name_is_rejected | 236-242 | REAL | — | |
| pattern_display_version_is_rejected | 245-251 | REAL | — | |
| pattern_accent_color_is_rejected | 254-260 | REAL | — | |
| pattern_reverse_dns_is_rejected | 263-269 | REAL | — | |
| pattern_guid_is_rejected | 272-278 | REAL | — | |
| pattern_title_template_unknown_variable_is_rejected | 281-294 | REAL | — | Hint names the only legal variable {product_name}. |
| pattern_msgid_description_key_is_rejected | 297-303 | REAL | — | |
| pattern_https_url_is_rejected | 306-312 | REAL | S4 | F-003. |
| pattern_bcp47_language_tag_is_rejected | 315-328 | REAL | — | Array index key path `.0` + line. |
| reserved_name_is_rejected | 333-342 | REAL | — | VLC reserved; rule+found+line. |
| unknown_key_typo_gets_a_did_you_mean_hint | 347-367 | REAL | — | TWO diagnostics asserted (unknown key + missing required) — every-violation semantics. |
| unknown_top_level_section_is_rejected | 370-378 | REAL | — | |
| helpers rejected/one | 11-34 | REAL | — | Fail on Ok / on != 1 hits; render_all in the panic text. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 27 | YES | Each requires rejection + exactly-one diagnostic + rule substring + found value; ~half also pin the exact 1-based line via content marker. A validator that silently accepted the mutation, emitted duplicates, or mis-tracked lines fails. Verified passing in baseline; NOT re-executed locally (sandbox linker fault — see log). |
| Fixture honesty | Partially | Fixtures contain the claimed mutation (verified on the 4 assigned via diff) but are stale base copies (F-001) — mutation-only claim false, tests unaffected. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every fixture in tests/fixtures/ must be rejected with the documented key path, rule, expected-vs-found and line" | fixtures_matrix.rs:1-3 (self) | REAL | 27 tests + rg coverage: all 42 fixtures consumed across the three suites; line asserted in most. |
| "Each fixture is the repo-root product.toml with exactly one documented mutation" | tests/fixtures/README.md:3-4 | FALSE | F-001: 39/40 fixtures are pre-0.1.3 copies with many deltas (diff, rg). |

## 7. Dead / duplicated / unreachable code
`let _ = src;` (F-002). Duplication: local `one()` duplicates semantic.rs's `one()` (identical body) — acceptable test-local helper.

## 8. Dependencies & flags
Test-only file; depends on product_config (load via common) + tests/common/mod.rs (root_schema() reads the committed product.schema.json — so the matrix also re-validates the schema file itself parses).

## 9. Open questions for the next agent
- Should the matrix grow a positive control (a CURRENT product.toml copy accepted)? load_root_product in common covers acceptance only inside semantic.rs's none-pass.
