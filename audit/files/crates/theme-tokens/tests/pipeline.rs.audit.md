# AUDIT: crates/theme-tokens/tests/pipeline.rs
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2025-06-14 |
| Lines / functions | 510 / 12 tests + 5 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only file; the code it exercises (theme_tokens::{parse,emit,color,model}) ships via ui/src/tokens bridge + `crates/theme-tokens/src/generated/tokens.rs` (verified: ui/src/tokens/generated/tokens.ts:4 cites the CSS bridge). Test itself runs in `cargo test` lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p theme-tokens --test pipeline` -> 12 passed / 0 failed (0.01s); rg over docs/08 + docs/waves/w26 for value provenance; sed quote verification. |

## 1. One-paragraph verdict
This is a real, hostile-grade test suite: it parses the committed themes/tokens.json, pins exact RGB/px/ms values, computes WCAG contrast at test time over all 9 accent ramps, proves emission byte-determinism, and runs 7 mutation-based negative tests against a minimal fixture (MINI) asserting specific diagnostic substrings. All 12 pass (ran). The one substantive problem is provenance drift: the test named `resolved_values_match_doc_08_section_1_2` pins light-mode and accent-bright values that directly contradict doc 08 §1.2's own tables — the true source is w26 Lead repins recorded in docs/waves/w26 logs, and doc 08 was never updated, so the test name is false and doc 08 is stale.

## 2. What actually works (verified)
— verified by `cargo test -p theme-tokens --test pipeline` -> `test result: ok. 12 passed; 0 failed`, including `accent_ramps_follow_the_contrast_rule` (computes `contrast_ratio(ramp.on_accent, ramp.primary) >= 4.5` for every ramp — a real computation over real data, not a golden echo).

## 3. Findings
F-001 [S3]
Where: crates/theme-tokens/tests/pipeline.rs:83-141 vs docs/08-ui-figma-fidelity.md:64-70
Code (test):
```
    // Light table (w26 g03/g05 Lead repin — frame warm grays).
    assert_eq!(
        color_of(&set, "--color-surface", Some("light"))?,
        Rgb8::new(0xc2, 0xc2, 0xc2)
    );
```
and (docs/08-ui-figma-fidelity.md:66-69):
```
| `#0E0E0E` @ 0.95 playlist panel | `#E7E7E7` @ 0.8 | |
| `#090909` seek track | `#DEDEDE` | |
| `#371F06` progress fill | `#D8BA9A` | |
| `#D37918` knob | `#E8851A` | brighter for contrast on light |
```
What is wrong: the test name promises "match doc 08 section 1.2", but the pinned light surface (#c2c2c2/#bababa) contradicts doc 08 §1.4's light table (#E7E7E7 panels), and `--color-accent-bright` dark=#e8851a / light=#b35f0d inverts doc 08's mapping (doc: #E8851A is the *light* knob value; dark knob stays #D37918). The values trace to real decisions (docs/waves/w26/logs/w26.gfix-C.md, w26.f15.md — found by rg), but doc 08 was never amended, and #b35f0d appears nowhere in docs/. Why it matters: anyone implementing from doc 08 ships different colors than the committed tokens; the test name asserts a fidelity that does not hold. What correct looks like: rename the test to cite the repin logs, and patch doc 08 §1.2/§1.4 with the w26 values.
Confidence: HIGH — Verification: read + ran (pins pass against committed tokens.json, so the test reflects the tokens, not the doc).

F-002 [S4]
Where: crates/theme-tokens/tests/pipeline.rs:206-219
Code:
```
    emit_css(&set, &mut css_a)?;
    emit_css(&set, &mut css_b)?;
    emit_rust(&set, &mut rust_a)?;
    emit_rust(&set, &mut rust_b)?;
    assert_eq!(css_a, css_b, "CSS emission must be byte-stable");
```
What is wrong: emitting the same in-memory `TokenSet` twice in one process is near-vacuous — emit.rs sorts every block by construction (emit.rs:197 `decls.sort_by`, :477 `names.sort_unstable`, :488 `semantic_consts.sort_by`), and no per-call randomness exists, so this can only fail if emit gains interior mutability. The guard that actually matters (fresh emission == committed artifacts, catching cross-process/cross-seed drift) lives in tests/diff.rs (audited by W11-A06). Not harmful; kept as a cheap smoke. Confidence: HIGH — Verification: read of emit.rs sort sites.

## 4. Per-function table (test file; >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| workspace_root / committed_source / committed_set / color_of | 16-42 | REAL | — | Honest helpers, errors propagate |
| committed_source_parses_with_the_documented_shape | 44-80 | REAL | — | Pins 123/86/86 counts, 9 accents, 6 accent-family css names |
| resolved_values_match_doc_08_section_1_2 | 82-159 | REAL | S3 | F-001: name lies for light + accent-bright pins |
| accent_ramps_follow_the_contrast_rule | 161-203 | REAL | — | Computes contrast over all ramps; gray-neutrality check |
| emission_is_deterministic | 205-219 | REAL | S4 | F-002: near-vacuous same-process double-emit |
| emitted_documents_have_the_documented_structure | 221-330 | REAL | — | Structure + j18 regression (no ACCENT_* leak in semantic section) |
| MINI + expect_error | 338-381 | REAL | — | Mutation harness asserting diagnostic substrings |
| mini_source_is_valid_and_counts_survive | 383-393 | REAL | — | Guards the fixture itself |
| layer_rules_are_enforced | 395-421 | REAL | — | 3 mutations: alias-layer, raw primitives, layer group |
| reference_errors_are_actionable | 423-445 | REAL | — | Unknown ref + cycle detection |
| per_mode_coverage_is_enforced | 447-458 | REAL | — | Missing-mode light |
| value_parsing_rejects_bad_values | 460-464 | REAL | — | #D3791 / un-hashed |
| meta_errors_are_actionable | 466-484 | REAL | — | Bad default-mode; missing $extensions |
| unsupported_types_and_categories_are_rejected | 486-510 | REAL | — | $type duration; semantic category elevation |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 12 | YES | Each pins concrete bytes/counts/values of the committed source; the 7 MINI tests mutate one token and require a specific error substring — breaking any validator arm fails its test (verified by run, and by mutation reasoning: e.g. deleting the cycle detector fails `reference_errors_are_actionable`) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "spot-check resolved values against doc 08 §1.2" | pipeline.rs:1-3 | PARTLY FALSE | F-001: light-mode + accent-bright pins come from w26 logs, not doc 08 |
| "on-accent glyph meets doc 08 §3.3 (≥ 4.5:1)" | pipeline.rs:172-179 | REAL | Contrast computed at test time, all 9 ramps, test passes |

## 7. Dead / duplicated / unreachable code
None. No #[ignore], no cfg gates.

## 8. Dependencies & flags
None; pure std + crate-under-test.

## 9. Open questions for the next agent
- Doc 08 owner: §1.2/§1.4 light tables and the accent-bright mapping need a w26 amendment (F-001).
- #b35f0d (light accent-bright) has no doc anchor at all — only wave logs.
