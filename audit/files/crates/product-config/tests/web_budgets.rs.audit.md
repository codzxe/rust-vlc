# AUDIT: crates/product-config/tests/web_budgets.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines / functions | 70 / 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file). Code under test: ProductConfig.web — same non-release-lane status as the rest of product-config. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n` quote check (exit 0); `rg` xtask web_budget.rs ordering enforcement (exit 0); baseline: all 3 web_budgets tests `ok` |

## 1. One-paragraph verdict
Three real tests over the REAL committed product.toml: (1) pins the [web.size_budgets] values 3/6 MiB with an explicit "deliberate pin" comment naming the review that owns the number; (2) asserts the full_p0 ≥ phase2 ordering invariant on committed values (and I verified the xtask gate independently enforces the same invariant on arbitrary product.toml, web_budget.rs:346-351 — so the rule is real, not test-only); (3) proves a layer document carrying [web] is rejected. All three pass in baseline. Weaknesses are pin-quality only: the layer-override test asserts merely `!diags.is_empty()` without pinning which key/rule rejected it.

## 2. What actually works (verified)
— verified by baseline `cargo-test-workspace.txt` product-config section: `committed_budgets_match_doc_09_section_2_4_proposal ... ok`, `budgets_respect_the_ordering_invariant ... ok`, `layers_cannot_override_web_budgets ... ok`. Cross-verified the gate's independent enforcement: `rg` shows web_budget.rs:346-351 `if self.full_p0_max_mib < self.phase2_vertical_max_mib` → typed error.

## 3. Findings
F-001 [S4]
Where: crates/product-config/tests/web_budgets.rs:61-69
Code:
```
    let diags = match load_partial_str("layer.toml", src, &schema) {
        Ok(_) => Vec::new(),
        Err(diags) => diags,
    };
    assert!(
        !diags.is_empty(),
```
What is wrong: the assertion proves only that SOMETHING rejected the layer document — not that `[web]` was rejected as an unknown key. If load_partial_str failed for an unrelated reason (schema drift, a parse bug in the partial loader), the test still passes. Contrast with the fixture-matrix tests, which pin key path + rule + line via `one()`. Mutation check: if PartialProduct grew a `web` field, the load would succeed → diags empty → test fails (the core regression IS caught); only mis-attribution escapes.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/product-config/tests/web_budgets.rs:39-55 vs tools/xtask/src/cmd/web_budget.rs:306-336
Code (test):
```
    // The schema documents this cross-field rule as enforced by the
    // web-budget gate (product.schema.json's full_p0_max_mib $comment);
    // it must hold on the committed values themselves.
```
What is wrong: the ordering invariant is enforced nowhere in product-config itself — not in schema.json (the $comment defers to the gate), not in semantic.rs (no cross-field rule for web). It lives only in the xtask gate, which re-parses product.toml with its OWN reader (`int_at`) instead of going through this crate — a second parallel implementation of [web] parsing (same split-brain family as the xtask config-validator enum hole). The test is honest about this ("must hold on the committed values themselves"), but a rebranded product.toml with inverted budgets is caught only if someone runs the xtask gate.
Confidence: HIGH — Verification: rg over schema.json ($comment) + web_budget.rs.

## 4. Per-function table
Test file — see §5.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_budgets_match_doc_09_section_2_4_proposal | YES | pins both values against the real committed file (3/6); any budget edit without updating the pin fails |
| budgets_respect_the_ordering_invariant | YES | asserts full_p0 >= phase2 on committed values; inverting product.toml fails it |
| layers_cannot_override_web_budgets | PARTIAL | catches the override regression (load would succeed) but not mis-attribution (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the wasm size budgets live in product.toml … never as literals in the cargo xtask web-budget gate's code" | web_budgets.rs:1-5 header | REAL (with caveat) | gate reads web.size_budgets.* via int_at (web_budget.rs:335-336); caveat: the gate re-implements the parsing (F-002) |
| "full_p0_max_mib >= phase2_vertical_max_mib is enforced by the web-budget gate" | product.schema.json $comment (L225) | REAL | web_budget.rs:346-351 typed error on inversion |
| "NOT layer-overridable in v1: PartialProduct has no web field" | model.rs:29-31 | REAL | layers_cannot_override_web_budgets passes; partial.rs has no web field |

## 7. Dead / duplicated / unreachable code
None in-file.

## 8. Dependencies & flags
product_config (load_partial_str), common module. No feature gates.

## 9. Open questions for the next agent
- tools/xtask/src/cmd/web_budget.rs is not my file: its own TOML reader (`int_at`) bypasses product-config — if THAT reader has holes (like config_validator's enum arm), the budget gate could mis-read. Flagged for the xtask owner.
- baseline full_gate says the "web-budget" section is RED (audit/runs/baseline/full_gate.txt) — these unit tests pass while the gate fails; worth checking which side (measurement vs config) — UNVERIFIED by me (baseline file not read in detail).
