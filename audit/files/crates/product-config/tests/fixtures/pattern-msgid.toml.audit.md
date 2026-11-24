# AUDIT: crates/product-config/tests/fixtures/pattern-msgid.toml
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines | 226 (stale base + 1 mutation) |
| File verdict | REAL (fixture; serves its rule row) |
| Reachable from shipped entry point? | N/A — consumed by `tests/fixtures_matrix.rs::pattern_msgid_description_key_is_rejected` (baseline: ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 (shared stale base; pattern-vs-key-grammar duplication) |
| Commands run | `diff` vs base (exactly 1 mutation); `sed -n '32p' | od -c` (byte-verified); python3 dump of schema `$defs.msgid` |

## 1. One-paragraph verdict
Valid fixture for the msgid pattern rule: `description_key` mutated to literal prose `"Product Description"`. The test pins exactly one diagnostic for `identity.description_key` with rule "must match the pattern" and found `"Product Description"`. Cross-checked: the schema's `$defs.msgid` pattern is `^[a-z0-9-]+(\.[a-z0-9-]+)+$` with a hand-written matcher (schema.rs refuses unmatched patterns at schema load), and the same shape is enforced independently by the localization crate's `MessageKey` grammar — two implementations of one grammar that could drift.

## 2. What actually works (verified)
— `sed -n '32p' | od -c` -> `description_key = "Product Description"\n`; baseline `pattern_msgid_description_key_is_rejected ... ok`; schema.json `$defs.msgid` pattern dumped and matches the rejection.

## 3. Findings
F-001 [S4] Stale base — identical to unknown-top-level.toml F-001 (diff-verified; display_version/version_code/min sizes/streaming-http/[web] all outdated).
F-002 [S4]
Where: product.schema.json `$defs.msgid` vs crates/localization/src/key.rs (MessageKey grammar)
Code (fixture, the rejected value):
```
description_key = "Product Description"
```
What is wrong: the msgid shape is defined twice — product.schema.json's pattern (validated by product-config's hand-written matcher registry) and localization's `MessageKey` ([a-z0-9-]+ segments, 2–5, per locales/en-US.toml:29-31). The two grammars differ in scope (schema allows unbounded segments; MessageKey caps at 5) so a description_key with 6 segments PASSES product-config and FAILS localization — the reference-complete audit rule (doc 07 §6.4 rule 4) is the only thing that reconciles them, and that reconciliation lives in a crate no shipped app depends on (see catalog.rs audit).
Confidence: HIGH — Verification: read of schema $defs + locales/en-US.toml header; localization key.rs not line-audited by me (UNVERIFIED beyond the documented grammar comment).
Confidence (F-001): HIGH — Verification: diff.

## 4. Per-function table
Data file — N/A. Mutation (byte-verified): L32 only.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_matrix::pattern_msgid_description_key_is_rejected | YES | `one()` pins exactly 1 diagnostic + rule text + found literal; removing the pattern matcher for msgid fails it (and schema load would also fail loudly on an unmatched pattern) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`description_key = "Product Description"`" (fixture map, Patterns) | tests/fixtures/README.md:45 | REAL | od-verified |
| "Product description: a localization key, never literal text (MP §5)" | fixture comment L30-31 (mirrors doc 07 §2.1) | REAL | the pattern + this fixture enforce it structurally |

## 7. Dead / duplicated / unreachable code
Duplicated stale base (by design).

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
Does any test pin the SCHEMA-parsed pattern and the MessageKey grammar against each other (6-segment msgid case)? If not, that's a latent drift gap between product-config and localization.
