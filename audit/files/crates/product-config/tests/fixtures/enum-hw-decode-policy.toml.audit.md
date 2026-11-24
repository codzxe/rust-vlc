# AUDIT: crates/product-config/tests/fixtures/enum-hw-decode-policy.toml
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Lines | 230 (product.toml snapshot + 1 mutation) |
| File verdict | REAL (fixture doing its one job) |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/fixtures_matrix.rs:141-159 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | sed/grep quote verification; diff vs product.toml; baseline citation |

## 1. Verdict
The post-ADR-001 enum-violation fixture: `hw_decode_policy = "hw-first"` (ADR-001's feature-matrix term) must be rejected against the legal config vocabulary software-only/prefer-hardware/hardware-only. The consuming test asserts exactly one `playback.hw_decode_policy` diagnostic, the legal-value text, found value, and the line — verified passing in baseline `cargo-test-workspace.txt:5077` (`enum_hw_decode_policy_is_rejected_with_the_post_adr001_legal_values ... ok`, 27/27 in fixtures_matrix). My re-run was blocked by disk-full (cargo-build-budget) — baseline cited instead.

## 2. What actually works (verified)
— baseline audit/runs/baseline/cargo-test-workspace.txt:5075-5107: fixtures_matrix 27 passed / 0 failed.

## 3. Findings
F-001 [S4]
Where: fixtures/enum-hw-decode-policy.toml vs product.toml (whole-file drift)
Code (fixture:189):
```
hw_decode_policy = "hw-first"
```
What is wrong: the fixture is a STALE snapshot of product.toml — 32 diff lines vs the committed root (display_version 0.1.0-dev vs 0.1.3, version_code 1 vs 4, min_width/min_height 480/320 vs 940/600, streaming-http false vs true, missing [web.size_budgets]). The README's premise "each fixture is the repo-root product.toml with exactly one documented mutation" no longer holds literally; the mutation still fires (schema range/enum checks are field-local), but a future schema rule on any drifted field would fail the matrix confusingly. Also inherited stale comment: product.toml:205's `# schema 1 MiB..=64 MiB` on probe_max_bytes contradicts the schema's enforced 16 MiB floor (product.schema.json minimum 16777216; the schema's own $comment calls the inline comment "the outlier").
Confidence: HIGH — Verification: read (diff run).

## 7. Dead/dup
None. ## 9. Open questions: none.
