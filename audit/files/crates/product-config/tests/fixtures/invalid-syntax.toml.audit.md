# AUDIT: crates/product-config/tests/fixtures/invalid-syntax.toml
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 2 / 0 (fixture data) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test data). Consumed by `tests/fixtures_matrix.rs::invalid_syntax_is_a_parse_diagnostic_with_line` (fixtures_matrix.rs:39-47). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | cat (full content, 2 lines); baseline citation: audit/runs/baseline/cargo-test-workspace.txt:5080 `test invalid_syntax_is_a_parse_diagnostic_with_line ... ok` (fresh rerun blocked by sandbox disk-full — environment) |

## 1. One-paragraph verdict
Minimal and correct: a two-line document whose second line is a valueless key — a genuine TOML syntax error at a known line. The test asserts Parse stage, key `<document>`, line == the `width = ` line (via marker_line), and rule containing "well-formed TOML". Zero findings.

## 2. What actually works (verified)
— Full content (verbatim, both lines):
```
schema_version = 1
width = 
```
— `width = ` with no value is a toml parse error (unexpected newline after `=`), so `load_file` returns a Parse-stage diagnostic; the test's `marker_line(&src, "width = ")` resolves to line 2. Baseline: `invalid_syntax_is_a_parse_diagnostic_with_line ... ok` (cargo-test-workspace.txt:5080).
— Honest scoping: unlike every other fixture, this is NOT the product.toml base with a mutation (a syntax error cannot coexist with a valid base) — and the README's table says exactly that (§3.4 stage-1 row).

## 3. Findings
None. Checked: content is exactly the documented mutation; the parse diagnostic path (load.rs parse_diagnostic with line attribution) is exercised; no stale-base concern applies (no base); no consumers beyond the one test; no secrets/binaries.

## 4-9. (short fixture audit)
Test honesty: would fail if parse errors lost line numbers, stage tagging, or the "well-formed TOML" rule text. No dead code, no deps. Open questions: none.
