# AUDIT: crates/product-config/tests/fixtures/id-consistency.toml
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines | 226 (stale base + 1 mutation) |
| File verdict | REAL (fixture; serves its rule row) |
| Reachable from shipped entry point? | N/A — consumed by `tests/semantic.rs::id_consistency_rule_3_3_b` (baseline: ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (shared stale base) |
| Commands run | `diff` vs unknown-top-level base (exactly 1 mutation, exit 1); `sed -n '42p' | od -c` (byte-verified) |

## 1. One-paragraph verdict
Valid fixture for doc 07 §3.3(b) id consistency: `app_user_model_id` mutated to `"wrong.id.Here"`. The consuming test asserts exactly one diagnostic for `identity.ids.windows.app_user_model_id` containing "PascalCase", expected `"dev.lumen.Lumen"`, found the mutated literal, the marker line, AND that the other three platform ids produce no diagnostics — a genuinely precise negative test.

## 2. What actually works (verified)
— `sed -n '42p' | od -c` -> `app_user_model_id = "wrong.id.Here"\n`; baseline `semantic.rs` suite: `id_consistency_rule_3_3_b ... ok`. The fixture must also pass STRUCTURAL validation (`semantic_diagnostics` unreachable!-guards it) — proving the stale base still clears the current schema.

## 3. Findings
F-001 [S4]
Where: crates/product-config/tests/fixtures/id-consistency.toml:42
Code:
```
app_user_model_id = "wrong.id.Here"
```
What is wrong: nothing in the mutation itself. The shared defect is the stale base (see unknown-top-level.toml audit F-001: 0.1.0-dev / version_code 1 / 480x320 / streaming-http=false / no `[web]`). One nuance: the test pins `expected == "dev.lumen.Lumen"` — the CONSISTENCY rule derives the expectation from `reverse_dns` + `name` inside the same fixture, so this pin is self-contained and drift-proof for the rule, unlike hardcoded copies elsewhere.
Confidence: HIGH — Verification: diff + od.

## 4. Per-function table
Data file — N/A. Mutation (byte-verified): L42 only.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| semantic::id_consistency_rule_3_3_b | YES | pins rule word "PascalCase", expected/found values, marker line, AND asserts zero diagnostics for the sibling macos bundle_id (over-firing would fail) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`app_user_model_id = "wrong.id.Here"`" (fixture map, Unique-identity (b)) | tests/fixtures/README.md:55 | REAL | od-verified |

## 7. Dead / duplicated / unreachable code
Duplicated stale base (by design).

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
None. The PascalCase derivation rule itself lives in src/semantic.rs (not my file).
