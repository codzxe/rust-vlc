# AUDIT: crates/product-config/tests/fixtures/range-default-rate.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 226 / n/a (test fixture) |
| File verdict | REAL (stale copy) |
| Reachable from shipped entry point? | NO — test input only; consumed by tests/fixtures_matrix.rs:186-193 `range_default_rate_is_rejected`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `diff product.toml <fixture>`; rg marker lines; baseline cargo-test-workspace.txt:5040 (fixtures_matrix 27 passed, exit 0). |

## 1. One-paragraph verdict
Real negative fixture: `default_rate = 10.0` at line 187 exceeds the schema ceiling 0.25..=4.0 and is rejected with an exact assertion. Same stale-copy drift as the rest of the set — the README's "exactly one documented mutation" claim is false for this file too.

## 2. What actually works (verified)
— verified by baseline fixtures_matrix run (27 passed, exit 0): rejected at `playback.default_rate`, rule contains ">= 0.25" and "<= 4", found "10".
— mutation confirmed at line 187 (sed); root product.toml:193 carries `default_rate = 1.0` (in range).

## 3. Findings
F-001 [S4]
Where: range-default-rate.toml:27,29,66-67,153 vs repo-root product.toml
Code:
```
display_version = "0.1.0-dev"
```
```
min_width = 480               # schema: min_width < width
```
```
streaming-http = false          # Phase 6
```
What is wrong: Undocumented drift (README:5-6 promises exactly one mutation): 0.1.0-dev/1 vs 0.1.3/4, 480/320 vs 940/600 (w28), streaming-http=false vs true (w31.3), pre-ADR-001 comments, `[web.size_budgets]` missing. Latent spurious-failure source on schema tightening; today harmless. Confidence: HIGH — Verification: ran (diff).
Mutation verified: `default_rate = 10.0` at :187 — matches README:34 ("above 4.0").

## 4. Per-function table
N/A (fixture).
## 5. Test honesty
`range_default_rate_is_rejected` (fixtures_matrix.rs:186-193): YES — asserts both bounds in the rule text and the found value.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exactly one documented mutation" | fixtures/README.md:5-6 | FALSE (drift) | diff vs product.toml |
| Mutation `default_rate = 10.0` (above 4.0) | fixtures/README.md:34 | REAL | fixture:187 + rejecting test |
## 7. Dead / duplicated / unreachable code
See fixture-set note in enum-crash-reports.toml.audit.md F-001.
## 8. Dependencies & flags
None (TOML test data).
## 9. Open questions for the next agent
None.
