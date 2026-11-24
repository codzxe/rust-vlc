# AUDIT: crates/product-config/tests/fixtures/enum-subtitle-mode.toml
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Lines | 226 (product.toml snapshot + 1 mutation) |
| File verdict | REAL (fixture doing its one job) |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/fixtures_matrix.rs:114-132 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | sed/grep quote verification; diff vs sibling fixtures; baseline citation |

## 1. Verdict
Enum-violation fixture: `subtitle_mode = "always"` must be rejected with "must be one of: auto, forced-only, on, off", found `"always"`, exact line. Test verified passing in baseline cargo-test-workspace.txt:5073 (`enum_subtitle_mode_is_rejected_with_the_legal_values ... ok`). Mutation isolated correctly — `hw_decode_policy = "software-only"` (valid) in the same fixture, so only one diagnostic fires.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:5075-5107 (fixtures_matrix 27/27); cross-diff vs enum-hw-decode-policy.toml shows exactly the two intended field differences.

## 3. Findings
F-001 [S4]
Where: fixtures/enum-subtitle-mode.toml:179
Code:
```
subtitle_mode = "always"
```
plus whole-file staleness (same 32-line drift vs root product.toml as the sibling fixtures: old display_version/version_code/min_width/min_height, streaming-http=false, no [web.size_budgets]). README's "exactly one documented mutation off the current product.toml" premise is stale; the enum check itself is field-local so the matrix still passes. Confidence: HIGH — Verification: read (diff run).

## 7. Dead/dup
None. ## 9. Open questions: none.
