# AUDIT: crates/product-config/tests/fixtures/missing-window-fields.toml
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 223 / 0 (fixture data) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test data). Consumed by `tests/fixtures_matrix.rs::missing_window_fields_reports_every_violation` (fixtures_matrix.rs:73-85). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | baseline citation: audit/runs/baseline/cargo-test-workspace.txt:5087 `test missing_window_fields_reports_every_violation ... ok` (fresh rerun blocked by sandbox disk-full linker error — environment, UNVERIFIED live); diff vs root product.toml; rg -c title_template/title_idle_key -> 0 hits in the fixture (both removed) |

## 1. One-paragraph verdict
Correct multi-field required-keys fixture: BOTH `title_template` and `title_idle_key` are removed from `[window]`, and the test asserts exactly one diagnostic for EACH key (proving doc 07 §3.4 stage 2's "report every violation, not just the first") with both diagnostics pointing at the `[window]` header line. This is the counterexample to first-error-only validation. Only defect: shared stale base (S4, README audit) — and this fixture's stale base carries the OLD 480x320 min sizes while the current product uses 940x600, though both remain schema-valid.

## 2. What actually works (verified)
— Baseline: `missing_window_fields_reports_every_violation ... ok` (cargo-test-workspace.txt:5087).
— Mutation verified: `rg -c "title_template|title_idle_key"` on the fixture -> 0 (both keys absent; the `[window]` section retains width/height/min_width/min_height/start_maximized/remember_geometry). Verbatim, L63-70:
```
[window]
width = 1280                  # schema 480..=3840
height = 720                  # schema 320..=2160
min_width = 480               # schema: min_width < width
min_height = 320              # schema: min_height < height
start_maximized = false
```
— Test pins `for_key(...).len() == 1` per key and `line == [window] header` for both — a missing required key inside an existing table attributes to the table header (consistent with missing-identity-field's behavior).

## 3. Findings
F-001 [S4] Stale base snapshot (identical to the other big fixtures; see README.md audit). Confidence: HIGH — Verification: diff. Note: the README intro says "exactly one documented mutation" while this fixture (and range-limits-multi) mutate multiple fields — the README's own table documents this honestly ("two diagnostics: every violation is reported"), so this is an intro-wording imprecision, covered in the README audit.

## 4-9. (short fixture audit)
Test honesty: would fail if (a) validation stopped at the first violation (assert len==1 per key), (b) line attribution for missing keys changed, or (c) either key stopped being required. No dead code, no deps. Open questions: none.
