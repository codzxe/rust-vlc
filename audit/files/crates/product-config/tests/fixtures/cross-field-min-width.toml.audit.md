# AUDIT: crates/product-config/tests/fixtures/cross-field-min-width.toml
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines | 227 (full product.toml copy, one mutation) |
| File verdict | REAL |
| Reachable | Test fixture — consumed by crates/product-config/tests/semantic.rs:46-55 `cross_field_min_width_must_be_below_width` (passes, baseline:5063 area, `test cross_field_min_width_must_be_below_width ... ok`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
Single-mutation fixture: a full copy of an older product.toml with `min_width = 1600` against `width = 1280` (line 66 vs 64), violating the cross-field rule `min_width < window.width` that JSON Schema cannot express. The consuming test (semantic.rs:45-55) asserts exactly one diagnostic for `window.min_width`, `Stage::Semantic`, rule text containing "must be < window.width", found "1600", and the exact source line — it would fail if the semantic rule were deleted, inverted, or misattributed. Honest fixture, honest test.
```
width = 1280                  # schema 480..=3840
height = 720                  # schema 320..=2160
min_width = 1600
```
(verified sed -n '64,67p' — note the mutation also drops the base file's `# schema: min_width < width` comment, so nothing in the fixture mislabels the line.)

F-001 [S4] The fixture base is a STALE copy of product.toml: `display_version = "0.1.0-dev"`, `version_code = 1`, old 480/320 minimums as the unmutated base, `streaming-http = false`, pre-ADR-001 comments, and no `[web.size_budgets]` — the committed product.toml is 0.1.3/version_code 4/940x600/streaming-http=true/ADR-001-accepted. Drift is loud, not silent: the `semantic_diagnostics` helper (semantic.rs:13-19) asserts every fixture passes structural validation first (`unreachable!` on failure), so any newly-required schema key breaks these tests immediately. The staleness still means no fixture exercises the current shipped shape. Confidence: HIGH — Verification: read + `diff product.toml <fixture>`.

## Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| semantic.rs:46 cross_field_min_width_must_be_below_width | YES | pins stage, rule text, found value, and line number of the sole diagnostic |
